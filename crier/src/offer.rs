//! Verified book entries from raw Nostr events.
//!
//! `pact_nostr::offer_from_event` checks the Nostr signature and that the
//! event author equals the envelope's `from`; on top of that we verify the
//! inner Pact signature (BIP340 over canonical JSON) and sanity-check the
//! terms. Only offers passing ALL checks enter the book.

use anyhow::{bail, Context, Result};
use nostr_sdk::prelude::Event;
use serde::Deserialize;

/// Minimal serde mirror of libswap's `OfferBody` (`pact/libswap/src/board.rs`)
/// — wire-compat by field name, tolerant of unknown fields so engine additions
/// never break crier. Follow-up: hoist the real type into pact-proto.
#[derive(Debug, Deserialize)]
struct OfferBodyMirror {
    network: String,
    give_asset: String,
    give_amount: u64,
    get_asset: String,
    get_amount: u64,
    #[serde(default)]
    ttl_secs: Option<u64>,
    #[serde(default)]
    created: u64,
}

/// Mirrors `OfferBody::expired`'s 24h fallback in libswap.
const DEFAULT_TTL_SECS: u64 = 24 * 3600;

/// One verified offer as tracked in the book.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookOffer {
    pub maker: String,
    pub swap_id: String,
    pub give_asset: String,
    pub give_amount: u64,
    pub get_asset: String,
    pub get_amount: u64,
    /// Body's own post time + lifetime (final expiry = created + ttl).
    pub created: u64,
    pub ttl_secs: u64,
    /// NIP-33 replacement order.
    pub event_created_at: u64,
    /// NIP-40 rolling relay TTL, if tagged. A maker who stops refreshing
    /// drops off the book when this passes.
    pub relay_expiration: Option<u64>,
}

impl BookOffer {
    pub fn final_expiry(&self) -> u64 {
        self.created.saturating_add(self.ttl_secs)
    }

    /// Gone from the book: past the final expiry, or past the rolling relay
    /// TTL (the maker stopped refreshing — a fresh event resets this).
    pub fn expired(&self, now: u64) -> bool {
        now > self.final_expiry() || self.relay_expiration.is_some_and(|e| now > e)
    }
}

/// Verify + decode a kind-31510 event into a `BookOffer`.
pub fn offer_from_nostr_event(event: &Event, network: &str) -> Result<BookOffer> {
    let envelope = pact_nostr::offer_from_event(event)?;
    pact_proto::envelope::verify(&envelope).context("inner pact signature")?;
    let body: OfferBodyMirror =
        serde_json::from_value(envelope.body.clone()).context("offer body shape")?;
    if body.network != network {
        bail!("offer is for network '{}', not '{network}'", body.network);
    }
    if body.give_amount == 0 || body.get_amount == 0 {
        bail!("zero-amount offer");
    }
    if body.give_asset.is_empty() || body.get_asset.is_empty() || body.give_asset == body.get_asset
    {
        bail!("malformed asset pair");
    }
    let event_created_at = event.created_at.as_secs();
    let relay_expiration = event.tags.iter().find_map(|t| {
        let s = t.as_slice();
        (s.first().map(String::as_str) == Some("expiration"))
            .then(|| s.get(1).and_then(|v| v.parse::<u64>().ok()))
            .flatten()
    });
    Ok(BookOffer {
        maker: envelope.from,
        swap_id: envelope.swap_id,
        give_asset: body.give_asset.to_lowercase(),
        give_amount: body.give_amount,
        get_asset: body.get_asset.to_lowercase(),
        get_amount: body.get_amount,
        // Legacy offers without a created stamp: fall back to the event time
        // so the final-expiry math still has an anchor.
        created: if body.created == 0 {
            event_created_at
        } else {
            body.created
        },
        ttl_secs: body.ttl_secs.unwrap_or(DEFAULT_TTL_SECS),
        event_created_at,
        relay_expiration,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1::{Keypair, Secp256k1, SecretKey};
    use pact_proto::envelope::Envelope;

    pub(crate) fn identity(seed: u8) -> (Keypair, nostr::Keys) {
        let sk = SecretKey::from_slice(&[seed; 32]).unwrap();
        let kp = Keypair::from_secret_key(&Secp256k1::new(), &sk);
        let keys = pact_nostr::keys_from_secret_hex(&hex_of(&sk.secret_bytes())).unwrap();
        (kp, keys)
    }

    fn hex_of(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    pub(crate) fn signed_offer_event(
        seed: u8,
        swap_id: &str,
        give: (&str, u64),
        get: (&str, u64),
        created: u64,
        now: u64,
    ) -> Event {
        let (kp, keys) = identity(seed);
        let body = serde_json::json!({
            "protocol": "pact-htlc-v2",
            "wire": 1u32,
            "network": "mainnet",
            "give_asset": give.0,
            "give_amount": give.1,
            "get_asset": get.0,
            "get_amount": get.1,
            "t1_secs": 28800u32,
            "t2_secs": 14400u32,
            "ttl_secs": 86400u64,
            "created": created,
        });
        let mut env = Envelope {
            v: 1,
            msg_type: "offer".into(),
            swap_id: swap_id.into(),
            from: String::new(),
            body,
            sig: String::new(),
        };
        pact_proto::envelope::sign(&mut env, &kp).unwrap();
        pact_nostr::offer_event(&env, &keys, now).unwrap()
    }

    #[test]
    fn verified_offer_decodes() {
        let ev = signed_offer_event(
            0x21,
            "aabbccdd00112233",
            ("btcx", 100_0000_0000),
            ("btc", 6760_0000),
            1_700_000_000,
            1_700_000_000,
        );
        let offer = offer_from_nostr_event(&ev, "mainnet").unwrap();
        assert_eq!(offer.give_asset, "btcx");
        assert_eq!(offer.get_amount, 6760_0000);
        assert_eq!(offer.final_expiry(), 1_700_000_000 + 86400);
        // NIP-40 rolling TTL present and enforced.
        assert!(offer.relay_expiration.is_some());
        assert!(offer.expired(1_700_000_000 + pact_nostr::RELAY_TTL_SECS + 1));
        assert!(!offer.expired(1_700_000_000 + 60));
    }

    #[test]
    fn wrong_network_rejected() {
        let ev = signed_offer_event(
            0x22,
            "aabbccdd00112244",
            ("btcx", 1000),
            ("btc", 10),
            1_700_000_000,
            1_700_000_000,
        );
        assert!(offer_from_nostr_event(&ev, "regtest").is_err());
    }

    #[test]
    fn tampered_content_rejected() {
        let ev = signed_offer_event(
            0x23,
            "aabbccdd00112255",
            ("btcx", 1000),
            ("btc", 10),
            1_700_000_000,
            1_700_000_000,
        );
        let mut json = serde_json::to_value(&ev).unwrap();
        json["content"] = serde_json::Value::String("{}".into());
        let tampered: Event = serde_json::from_value(json).unwrap();
        assert!(offer_from_nostr_event(&tampered, "mainnet").is_err());
    }
}
