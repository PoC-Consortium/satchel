//! Corkboard client + relay-based swap coordination.
//!
//! The board is a noticeboard (signed offers) plus a blind
//! store-and-forward relay. This module gives the engine a board-driven
//! handshake: the same protocol envelopes that travel as files in the
//! manual flow travel through the relay here, with two additions that are
//! coordination-layer only (not part of pact-htlc-v1):
//!
//! - `offer`  — a posted advert: amounts, network, timelock durations.
//! - `take`   — a taker's interest, echoing the maker's signed offer back
//!   so the maker can rebuild the terms statelessly and trustlessly.
//!
//! Flow (maker = Alice/initiator, taker = Bob/participant):
//!
//! ```text
//! maker  --offer-->  board  <--list--  taker
//! taker  --take----> relay  --------->  maker (verifies own offer sig)
//! maker  --init----> relay  --------->  taker (verifies terms == offer)
//! taker  --accept--> relay  --------->  maker → fund A → funded(a) → …
//! ```

use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};

use crate::messages::{self, Envelope};
use crate::rpc::http_json;

// Relay sealing lives in pact-proto now; re-exported so existing
// `crate::board::{seal_envelope, open_envelope}` callers are unchanged.
pub use pact_proto::seal::{open_envelope, seal_envelope};

pub struct BoardClient {
    base: String,
}

impl BoardClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub fn post_offer(&self, offer: &Envelope) -> Result<String> {
        let reply = http_json(
            &format!("{}/v1/offers", self.base),
            Some(&serde_json::to_value(offer)?),
        )?;
        reply["offer_id"]
            .as_str()
            .map(str::to_string)
            .context("board returned no offer_id")
    }

    pub fn offers(&self) -> Result<Vec<Envelope>> {
        let reply = http_json(&format!("{}/v1/offers", self.base), None)?;
        Ok(serde_json::from_value(reply["offers"].clone()).unwrap_or_default())
    }

    pub fn revoke(&self, revocation: &Envelope) -> Result<()> {
        http_json(
            &format!("{}/v1/offers/revoke", self.base),
            Some(&serde_json::to_value(revocation)?),
        )?;
        Ok(())
    }

    /// Send a pre-sealed blob to `to` (x-only identity pubkey, hex)
    /// through the blind relay (seal with [`seal_envelope`]).
    pub fn relay_send_blob(&self, to: &str, blob: &str) -> Result<()> {
        http_json(
            &format!("{}/v1/relay", self.base),
            Some(&json!({ "to": to, "blob": blob })),
        )?;
        Ok(())
    }

    /// Fetch our raw mail newer than `since_id` (open with
    /// [`open_envelope`]). `poll` must be a signed `relay_poll` envelope
    /// (proves we own the recipient identity).
    pub fn relay_poll(&self, poll: &Envelope) -> Result<Vec<(i64, String)>> {
        let reply = http_json(
            &format!("{}/v1/relay/poll", self.base),
            Some(&serde_json::to_value(poll)?),
        )?;
        let mut out = Vec::new();
        for message in reply["messages"].as_array().cloned().unwrap_or_default() {
            let id = message["id"].as_i64().context("relay message without id")?;
            let blob = message["blob"]
                .as_str()
                .context("relay message without blob")?;
            out.push((id, blob.to_string()));
        }
        Ok(out)
    }
}

/// A transport the engine can post offers to and relay sealed messages
/// through. The HTTP Corkboard ([`BoardClient`]) is one impl; a Nostr
/// transport (`NostrBoard`) is another. The engine fans every operation out
/// across all configured boards and polls mail from each, so two parties
/// need only *one* board in common.
///
/// Everything crossing this trait is a transport-agnostic signed
/// [`Envelope`]; the relay carries opaque sealed blobs (see
/// [`seal_envelope`]). `relay_poll` returns `(cursor, blob)` pairs where
/// `cursor` is a per-board monotonic id the engine persists to avoid
/// reprocessing — for HTTP it is the board's autoincrement id; a Nostr
/// board mimics the same contract via a local autoincrement inbox.
///
/// Not `Send + Sync`: boards are built on demand and used synchronously
/// within one engine call (no await, no thread hop), and `NostrBoard`
/// borrows the engine's `Store` (whose rusqlite connection is `!Sync`).
pub trait Noticeboard {
    fn post_offer(&self, offer: &Envelope) -> Result<String>;
    fn offers(&self) -> Result<Vec<Envelope>>;
    fn revoke(&self, revocation: &Envelope) -> Result<()>;
    fn relay_send_blob(&self, to: &str, blob: &str) -> Result<()>;
    fn relay_poll(&self, poll: &Envelope) -> Result<Vec<(i64, String)>>;

    /// Publish an encrypted-to-self swap-state snapshot for seed-only rescue
    /// (issue #54). Nostr-only — the HTTP board has no rescue channel, so the
    /// default is a no-op. The Nostr service maps `swap_id` to an OPAQUE
    /// replaceable-event tag (`snapshot_dtag`), so the swap_id never leaves the
    /// machine in the clear. `seq` ranks snapshots of the SAME swap (v2:
    /// accept 0, Signed 1) — stamped into the event's `created_at` so a
    /// later-state snapshot strictly replaces an earlier one even when both
    /// publish within the same second (an equal-created_at NIP-33 replacement
    /// ties by lowest id and could keep the STALE state).
    fn publish_snapshot(&self, _swap_id: &str, _sealed_blob: &str, _seq: u64) -> Result<()> {
        Ok(())
    }
    /// Tombstone a swap's snapshot on a terminal state so a rescued machine
    /// never resurrects it. Nostr-only; default no-op.
    fn tombstone_snapshot(&self, _swap_id: &str) -> Result<()> {
        Ok(())
    }
}

// Inherent methods stay (direct callers in pactd use a concrete
// `BoardClient`); the trait impl delegates to them. Inherent methods take
// resolution priority, so `BoardClient::post_offer` here is not recursive.
impl Noticeboard for BoardClient {
    fn post_offer(&self, offer: &Envelope) -> Result<String> {
        BoardClient::post_offer(self, offer)
    }
    fn offers(&self) -> Result<Vec<Envelope>> {
        BoardClient::offers(self)
    }
    fn revoke(&self, revocation: &Envelope) -> Result<()> {
        BoardClient::revoke(self, revocation)
    }
    fn relay_send_blob(&self, to: &str, blob: &str) -> Result<()> {
        BoardClient::relay_send_blob(self, to, blob)
    }
    fn relay_poll(&self, poll: &Envelope) -> Result<Vec<(i64, String)>> {
        BoardClient::relay_poll(self, poll)
    }
}

/// Body of an `offer` envelope. Timelocks are *durations*: the absolute
/// T1/T2 are fixed only when an `init` is actually created.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OfferBody {
    pub protocol: String,
    /// Wire-compatibility epoch of `protocol` (see [`crate::wire_epoch`]).
    /// Signed with the body; absent (a pre-rc10 maker) parses as 1, so old
    /// v1 offers stay takeable and old v2 offers gate cleanly.
    #[serde(default = "default_wire")]
    pub wire: u32,
    pub network: String,
    pub give_asset: String,
    pub give_amount: u64,
    pub get_asset: String,
    pub get_amount: u64,
    pub t1_secs: u32,
    pub t2_secs: u32,
    pub ttl_secs: Option<u64>,
    /// Unix creation time, *inside the signed body* so expiry can be
    /// verified from the envelope alone (the board's listing TTL is
    /// only a courtesy).
    pub created: u64,
}

/// Absent `wire` on any wire body = epoch 1, the pre-rc10 era.
pub(crate) fn default_wire() -> u32 {
    1
}

impl OfferBody {
    pub fn expired(&self, now: u64) -> bool {
        now > self.created + self.ttl_secs.unwrap_or(24 * 3600)
    }
}

/// Validate a `take` envelope and extract the maker's own offer from it.
/// The taker echoes the full signed offer, so the maker needs no local
/// state and cannot be tricked into different terms: the offer signature
/// is checked AND the identity must be the maker's own.
pub fn offer_from_take(take: &Envelope, our_identity: &str) -> Result<(Envelope, OfferBody)> {
    ensure!(take.msg_type == "take", "not a take envelope");
    messages::verify(take)?;
    let offer: Envelope = serde_json::from_value(take.body["offer"].clone())
        .context("take without embedded offer")?;
    ensure!(
        offer.msg_type == "offer",
        "embedded envelope is not an offer"
    );
    messages::verify(&offer)?;
    ensure!(
        offer.from == our_identity,
        "take echoes an offer signed by {} — not ours",
        offer.from
    );
    ensure!(take.swap_id == offer.swap_id, "take/offer id mismatch");
    let body: OfferBody =
        serde_json::from_value(offer.body.clone()).context("malformed offer body")?;
    ensure!(
        body.protocol == crate::PROTOCOL_VERSION
            || body.protocol == crate::adaptor_swap::PROTOCOL_V2,
        "offer protocol {} unsupported",
        body.protocol
    );
    Ok((offer, body))
}

/// Validate that an incoming `init` honors the terms of the offer the
/// taker took: identical amounts/assets and timelocks within tolerance of
/// the advertised durations (the maker fixes absolute times at init).
pub fn init_matches_offer(init_body: &Value, offer: &OfferBody, now: u64) -> Result<()> {
    let tol: i64 = 15 * 60; // clock skew + relay latency tolerance
    ensure!(
        init_body["amount_a"].as_u64() == Some(offer.give_amount),
        "init amount_a != offer"
    );
    ensure!(
        init_body["amount_b"].as_u64() == Some(offer.get_amount),
        "init amount_b != offer"
    );
    ensure!(
        init_body["chain_a"]["asset"].as_str() == Some(offer.give_asset.as_str())
            && init_body["chain_b"]["asset"].as_str() == Some(offer.get_asset.as_str()),
        "init assets != offer"
    );
    for (key, dur) in [("t1", offer.t1_secs), ("t2", offer.t2_secs)] {
        let t = init_body[key].as_u64().context("init missing timelock")? as i64;
        let expected = now as i64 + i64::from(dur);
        ensure!(
            (t - expected).abs() <= tol,
            "init {key} deviates from the offered duration by {}s",
            (t - expected).abs()
        );
    }
    Ok(())
}

#[cfg(test)]
mod wire_tests {
    use super::*;

    /// The wire epochs are PROTOCOL constants (rc10): absent `wire` on a
    /// pre-rc10 body parses as 1, v1 stays 1, v2 is 2. Changing an epoch is
    /// a deliberate flag-day — see `crate::wire_epoch`.
    #[test]
    fn wire_defaults_and_epochs() {
        let old: OfferBody = serde_json::from_value(serde_json::json!({
            "protocol": "pact-htlc-v1",
            "network": "regtest",
            "give_asset": "btcx",
            "give_amount": 1u64,
            "get_asset": "btc",
            "get_amount": 1u64,
            "t1_secs": 1u32,
            "t2_secs": 1u32,
            "ttl_secs": null,
            "created": 0u64,
        }))
        .expect("a pre-rc10 offer body (no `wire`) must still parse");
        assert_eq!(old.wire, 1);
        assert_eq!(crate::wire_epoch(crate::PROTOCOL_VERSION), crate::WIRE_V1);
        assert_eq!(
            crate::wire_epoch(crate::adaptor_swap::PROTOCOL_V2),
            crate::WIRE_V2
        );
        assert_eq!(crate::WIRE_V1, 1);
        assert_eq!(crate::WIRE_V2, 2);
    }
}
