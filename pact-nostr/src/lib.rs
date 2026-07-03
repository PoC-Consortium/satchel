//! pact-nostr — map Pact wire envelopes to and from Nostr events.
//!
//! This crate is pure mapping + crypto: no relay connections, no async,
//! no engine. It is the wire-format half of the Nostr transport described
//! in `spec/protocol.md §8.8`; the relay pool and buffering live in the
//! background service (`pactd/src/nostr_service.rs`), and the
//! [`Noticeboard`] facade lives in `libswap`.
//!
//! Three event shapes, all carrying the *unchanged* Pact [`Envelope`] (or
//! a sealed blob) so the engine's existing `messages::verify` and
//! `seal::open_envelope` still apply on the far side:
//!
//! - **Offer** — kind [`OFFER_KIND`], an addressable (NIP-33) advert keyed
//!   by `d = swap_id`, signed by the maker's identity key, content = the
//!   signed `offer` envelope JSON, with a NIP-40 `expiration` derived from
//!   the offer's own `created + ttl_secs`.
//! - **Gift wrap** — kind [`GIFTWRAP_KIND`], a NIP-59-style wrapper signed
//!   by a fresh one-time key and `#p`-tagged to the recipient, content =
//!   the existing `PACTSEALED1:` blob. The ephemeral author hides the
//!   sender from relays; the seal hides the contents from everyone.
//!
//! [`Envelope`]: pact_proto::envelope::Envelope
//! [`Noticeboard`]: # "see libswap"

use anyhow::{ensure, Context, Result};
use nostr::prelude::*;
use pact_proto::envelope::Envelope;

/// Addressable offer advert (NIP-33), one per `(maker pubkey, swap_id)`.
/// Custom kind keeps non-spendable swap offers out of generic Nostr
/// marketplace clients (spec/protocol.md §8.8).
pub const OFFER_KIND: u16 = 31510;

/// Gift-wrapped relay message: ephemeral author, `#p` recipient, sealed
/// content. NIP-59-style structure with a `PACTSEALED1:` payload.
pub const GIFTWRAP_KIND: u16 = 1059;

/// Fallback for an offer body's FINAL lifetime when it omits `ttl_secs`
/// (mirrors `OfferBody::expired`'s 24h default in libswap).
const DEFAULT_TTL_SECS: u64 = 24 * 3600;

/// Rolling relay TTL: a published offer event drops from relays this long after
/// it was published unless the maker re-publishes (refreshes) first. Short, so a
/// listing reflects a maker who is actually online; the engine refreshes on a
/// shorter cadence (`Engine::REFRESH_SECS`, 10 min) so a live offer never lapses.
pub const RELAY_TTL_SECS: u64 = 30 * 60;

/// Build `nostr::Keys` from a 32-byte secp256k1 secret in hex. The
/// resulting npub equals the Pact identity pubkey, since both are BIP340
/// x-only keys over the same secret.
pub fn keys_from_secret_hex(secret_hex: &str) -> Result<Keys> {
    Keys::parse(secret_hex).context("invalid identity secret for nostr keys")
}

/// NIP-40 expiration (unix secs) for a freshly published offer event: a short
/// ROLLING relay TTL (`now + RELAY_TTL_SECS`) so the listing drops soon after the
/// maker stops refreshing — but never later than the offer's own FINAL expiry
/// (`created + ttl_secs`), past which the engine stops refreshing anyway. `now`
/// is the publish time. Returns `None` for legacy offers without a `created`
/// stamp (so no premature relay drop).
fn offer_expiration(offer: &Envelope, now: u64) -> Option<u64> {
    let created = offer.body.get("created").and_then(|v| v.as_u64())?;
    if created == 0 {
        return None;
    }
    let ttl = offer
        .body
        .get("ttl_secs")
        .and_then(|v| v.as_u64())
        .unwrap_or(DEFAULT_TTL_SECS);
    let final_expiry = created + ttl;
    Some((now + RELAY_TTL_SECS).min(final_expiry))
}

/// Build a signed Nostr offer event from a signed Pact `offer` envelope.
/// The maker's identity key signs both layers: the inner Pact signature
/// authenticates the terms, the Nostr signature lets relays/clients accept
/// the event. `keys` MUST be the maker's identity key (matching
/// `offer.from`).
pub fn offer_event(offer: &Envelope, keys: &Keys, now: u64) -> Result<Event> {
    ensure!(offer.msg_type == "offer", "not an offer envelope");
    ensure!(
        offer.from == keys.public_key().to_hex(),
        "offer.from does not match the signing key"
    );
    let content = serde_json::to_string(offer)?;
    let mut builder = EventBuilder::new(Kind::Custom(OFFER_KIND), content)
        .tag(Tag::identifier(offer.swap_id.clone()));
    // Plain (non-indexed) descriptive tags; discovery subscribes by kind
    // and filters pair/network client-side, mirroring the HTTP board.
    for (key, field) in [
        ("network", "network"),
        ("give", "give_asset"),
        ("get", "get_asset"),
    ] {
        if let Some(v) = offer.body.get(field).and_then(|v| v.as_str()) {
            builder = builder.tag(Tag::parse([key, v])?);
        }
    }
    if let Some(exp) = offer_expiration(offer, now) {
        builder = builder.tag(Tag::expiration(Timestamp::from(exp)));
    }
    builder.sign_with_keys(keys).context("sign offer event")
}

/// Parse a Pact `offer` envelope back out of a Nostr offer event. Verifies
/// the Nostr event signature and that its author matches the inner
/// `offer.from`; the caller still verifies the inner Pact signature
/// (`messages::verify`) and freshness before trusting terms.
pub fn offer_from_event(event: &Event) -> Result<Envelope> {
    ensure!(
        event.kind.as_u16() == OFFER_KIND,
        "not a pact offer event (kind {})",
        event.kind.as_u16()
    );
    event
        .verify()
        .map_err(|e| anyhow::anyhow!("bad nostr event signature: {e}"))?;
    let envelope: Envelope = serde_json::from_str(&event.content)
        .context("offer event content is not a Pact envelope")?;
    ensure!(
        envelope.msg_type == "offer",
        "embedded envelope is not an offer"
    );
    ensure!(
        envelope.from == event.pubkey.to_hex(),
        "offer.from does not match the nostr event author"
    );
    Ok(envelope)
}

/// Build a NIP-09 deletion event for one of our offers, referencing the
/// addressable coordinate `31510:<our pubkey>:<swap_id>` so relays drop the
/// listing. Signed by the maker's identity key.
pub fn revocation_event(swap_id: &str, keys: &Keys) -> Result<Event> {
    let coordinate = format!("{OFFER_KIND}:{}:{swap_id}", keys.public_key().to_hex());
    EventBuilder::new(Kind::EventDeletion, "")
        .tag(Tag::parse(["a", &coordinate])?)
        .sign_with_keys(keys)
        .context("sign offer revocation event")
}

/// Gift-wrap a pre-sealed relay blob (`PACTSEALED1:…`, from
/// `pact_proto::seal::seal_envelope`) as a kind-1059 event signed by a
/// fresh one-time key and `#p`-tagged to the recipient. The ephemeral
/// author hides the sender from relays.
pub fn giftwrap(recipient_xonly_hex: &str, sealed_blob: &str) -> Result<Event> {
    let recipient = PublicKey::from_hex(recipient_xonly_hex).context("invalid recipient pubkey")?;
    let ephemeral = Keys::generate();
    EventBuilder::new(Kind::Custom(GIFTWRAP_KIND), sealed_blob.to_string())
        .tag(Tag::public_key(recipient))
        .sign_with_keys(&ephemeral)
        .context("sign gift-wrap event")
}

/// Extract the sealed blob from a gift-wrap event for
/// `pact_proto::seal::open_envelope`. Verifies the (ephemeral) event
/// signature for integrity; `open_envelope` then proves the message is
/// actually addressed to us.
pub fn unwrap_giftwrap(event: &Event) -> Result<String> {
    ensure!(
        event.kind.as_u16() == GIFTWRAP_KIND,
        "not a gift-wrap event (kind {})",
        event.kind.as_u16()
    );
    event
        .verify()
        .map_err(|e| anyhow::anyhow!("bad nostr event signature: {e}"))?;
    Ok(event.content.clone())
}

// ---- Subscription filters (used by the Phase 3 relay service) ----

/// Filter for discovering all Pact offers (subscribe by kind; pair/network
/// filtering is client-side).
pub fn offers_filter() -> Filter {
    Filter::new().kind(Kind::Custom(OFFER_KIND))
}

/// Filter for our gift-wrap mailbox: kind-1059 events `#p`-tagged to us.
pub fn mailbox_filter(me_xonly_hex: &str) -> Result<Filter> {
    let me = PublicKey::from_hex(me_xonly_hex).context("invalid identity pubkey")?;
    Ok(Filter::new().kind(Kind::Custom(GIFTWRAP_KIND)).pubkey(me))
}

/// Filter for NIP-09 deletions (kind 5). The relay can't pre-filter these to
/// "offer deletions only", so the ownership/coordinate check is done client-side
/// in [`revoked_offer_from_event`].
pub fn deletions_filter() -> Filter {
    Filter::new().kind(Kind::EventDeletion)
}

/// If `event` is a NIP-09 deletion that revokes one of its OWN offers, return
/// that offer's `swap_id`. Verifies the event signature and that the deletion's
/// author matches the pubkey in the addressable coordinate
/// (`{OFFER_KIND}:<author>:<swap_id>`, as built by [`revocation_event`]) — so a
/// maker can only revoke offers it signed, never someone else's. `None` for
/// foreign, unrelated, or malformed deletions.
pub fn revoked_offer_from_event(event: &Event) -> Option<String> {
    if event.kind != Kind::EventDeletion {
        return None;
    }
    event.verify().ok()?;
    let author = event.pubkey.to_hex();
    for tag in event.tags.iter() {
        let s = tag.as_slice();
        if s.first().map(String::as_str) != Some("a") {
            continue;
        }
        let mut parts = s.get(1)?.split(':');
        let kind = parts.next()?;
        let pubkey = parts.next()?;
        let swap_id = parts.next().unwrap_or("");
        if kind.parse::<u16>().ok() == Some(OFFER_KIND) && pubkey == author && !swap_id.is_empty() {
            return Some(swap_id.to_string());
        }
    }
    None
}

// ---- Encrypted swap-state snapshots (seed-only cross-machine rescue) ----
//
// A party backs up its in-flight swap state to the relays, encrypted to
// ITSELF, so a machine restored from the seed alone can rediscover and resume
// (or refund) live swaps. Shape: an addressable (NIP-33) event under the
// party's own identity key, `d` = an OPAQUE per-swap tag (so a relay can't
// link the snapshot to the party's public offer for the same swap), content =
// a `PACTSEALED1:` blob sealed to the party's own identity key (so only they
// can read it). Replaceable → the relay keeps only the latest per swap;
// tombstoned via NIP-09 on terminal. See issue #54 / the safety handbook.

/// Addressable encrypted swap-state snapshot, one per `(our pubkey, swap)`.
/// Distinct kind from offers so the two never collide.
pub const SNAPSHOT_KIND: u16 = 31512;

/// The opaque, deterministic `d`-tag for a swap's snapshot. Derived from the
/// `swap_id` so every update to the same swap REPLACES the prior event, but
/// opaque so a relay cannot correlate it with the public offer (kind 31510,
/// `d = swap_id`) for the same swap. We never need to reverse it: rescue
/// fetches all our snapshots by author+kind and reads the swap_id from inside
/// the (decrypted) payload.
pub fn snapshot_dtag(swap_id: &str) -> String {
    hex::encode(pact_proto::crypto::tagged_hash(
        "pact/rescue/dtag/v1",
        swap_id.as_bytes(),
    ))
}

/// Build a signed addressable snapshot event carrying a `PACTSEALED1:` blob
/// (sealed to our own identity via `pact_proto::seal::seal_envelope`). `keys`
/// MUST be our identity key. No NIP-40 expiration: a live swap's snapshot must
/// persist until we tombstone it on completion, and we publish sparsely (at
/// accept and, for v2, at signing) rather than refreshing on a timer.
pub fn snapshot_event(sealed_blob: &str, dtag: &str, keys: &Keys) -> Result<Event> {
    EventBuilder::new(Kind::Custom(SNAPSHOT_KIND), sealed_blob.to_string())
        .tag(Tag::identifier(dtag.to_string()))
        .sign_with_keys(keys)
        .context("sign snapshot event")
}

/// NIP-09 deletion for one of our snapshots (coordinate
/// `SNAPSHOT_KIND:<our pubkey>:<dtag>`), published when the swap reaches a
/// terminal state so a rescued machine never resurrects a finished swap.
pub fn snapshot_tombstone_event(dtag: &str, keys: &Keys) -> Result<Event> {
    let coordinate = format!("{SNAPSHOT_KIND}:{}:{dtag}", keys.public_key().to_hex());
    EventBuilder::new(Kind::EventDeletion, "")
        .tag(Tag::parse(["a", &coordinate])?)
        .sign_with_keys(keys)
        .context("sign snapshot tombstone event")
}

/// Filter to fetch all OUR snapshots on rescue: our own addressable snapshot
/// events (by author + kind). Content is opened with our identity key.
pub fn my_snapshots_filter(me_xonly_hex: &str) -> Result<Filter> {
    let me = PublicKey::from_hex(me_xonly_hex).context("invalid identity pubkey")?;
    Ok(Filter::new().kind(Kind::Custom(SNAPSHOT_KIND)).author(me))
}

/// Verify a snapshot event and return its sealed blob for
/// `pact_proto::seal::open_envelope`. Checks the event signature and that its
/// author is us (`me_xonly_hex`) — a foreign snapshot event is ignored.
pub fn snapshot_blob_from_event(event: &Event, me_xonly_hex: &str) -> Result<String> {
    ensure!(
        event.kind.as_u16() == SNAPSHOT_KIND,
        "not a snapshot event (kind {})",
        event.kind.as_u16()
    );
    event
        .verify()
        .map_err(|e| anyhow::anyhow!("bad nostr event signature: {e}"))?;
    ensure!(
        event.pubkey.to_hex() == me_xonly_hex,
        "snapshot event author is not us"
    );
    Ok(event.content.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1::{Keypair, Secp256k1, SecretKey};

    fn identity(seed: u8) -> (Keypair, Keys, String) {
        let sk = SecretKey::from_slice(&[seed; 32]).unwrap();
        let kp = Keypair::from_secret_key(&Secp256k1::new(), &sk);
        let keys = keys_from_secret_hex(&hex::encode(sk.secret_bytes())).unwrap();
        let xonly = kp.x_only_public_key().0.to_string();
        (kp, keys, xonly)
    }

    fn signed_offer(kp: &Keypair) -> Envelope {
        let body = serde_json::json!({
            "protocol": "pact-htlc-v1",
            "network": "regtest",
            "give_asset": "pocx",
            "give_amount": 1000u64,
            "get_asset": "btc",
            "get_amount": 10u64,
            "t1_secs": 28800u32,
            "t2_secs": 14400u32,
            "ttl_secs": 3600u64,
            "created": 1_700_000_000u64,
        });
        let mut env = Envelope {
            v: 1,
            msg_type: "offer".into(),
            swap_id: "0011223344556677".into(),
            from: String::new(),
            body,
            sig: String::new(),
        };
        pact_proto::envelope::sign(&mut env, kp).unwrap();
        env
    }

    #[test]
    fn nostr_pubkey_equals_pact_identity() {
        let (_, keys, xonly) = identity(0x11);
        assert_eq!(keys.public_key().to_hex(), xonly);
    }

    #[test]
    fn offer_event_roundtrip_and_verifies() {
        let (kp, keys, _) = identity(0x22);
        let offer = signed_offer(&kp);
        let event = offer_event(&offer, &keys, 1_700_000_000).unwrap();
        assert_eq!(event.kind.as_u16(), OFFER_KIND);
        // d-tag carries the swap_id; expiration tag present.
        let tags: Vec<Vec<String>> = event.tags.iter().map(|t| t.clone().to_vec()).collect();
        assert!(tags.iter().any(|t| t[0] == "d" && t[1] == offer.swap_id));
        // Rolling relay TTL: now (1_700_000_000) + RELAY_TTL_SECS, capped at the
        // body's final expiry (created 1_700_000_000 + ttl 3600 = 1_700_003_600).
        let exp = tags
            .iter()
            .find(|t| t[0] == "expiration")
            .expect("expiration tag");
        assert_eq!(exp[1], (1_700_000_000 + RELAY_TTL_SECS).to_string());
        // And a publish time near the final expiry is capped, not exceeded.
        let late = offer_event(&offer, &keys, 1_700_003_000).unwrap();
        let late_tags: Vec<Vec<String>> = late.tags.iter().map(|t| t.clone().to_vec()).collect();
        let late_exp = late_tags
            .iter()
            .find(|t| t[0] == "expiration")
            .expect("expiration tag");
        assert_eq!(late_exp[1], "1700003600"); // final expiry, not 1_700_003_000+1800

        let back = offer_from_event(&event).unwrap();
        assert_eq!(back, offer);
        // Inner Pact signature still validates independently.
        pact_proto::envelope::verify(&back).unwrap();
    }

    #[test]
    fn tampered_offer_event_is_rejected() {
        let (kp, keys, _) = identity(0x23);
        let offer = signed_offer(&kp);
        let mut event_json =
            serde_json::to_value(offer_event(&offer, &keys, 1_700_000_000).unwrap()).unwrap();
        // Flip a byte in content; the nostr id/sig no longer match.
        event_json["content"] = serde_json::Value::String("garbage".into());
        let tampered: Event = serde_json::from_value(event_json).unwrap();
        assert!(offer_from_event(&tampered).is_err());
    }

    #[test]
    fn giftwrap_roundtrip_through_seal() {
        let (_maker_kp, maker_keys, _maker_x) = identity(0x31);
        let (recipient_kp, _recipient_keys, recipient_x) = identity(0x32);

        // A take envelope sealed to the recipient, then gift-wrapped.
        let take = signed_offer(&_maker_kp); // any signed envelope works as payload
        let blob = pact_proto::seal::seal_envelope(&recipient_x, &take).unwrap();
        let wrap = giftwrap(&recipient_x, &blob).unwrap();
        assert_eq!(wrap.kind.as_u16(), GIFTWRAP_KIND);
        // #p tag addresses the recipient; author is ephemeral (not the maker).
        assert_ne!(wrap.pubkey.to_hex(), maker_keys.public_key().to_hex());

        let unwrapped = unwrap_giftwrap(&wrap).unwrap();
        let opened = pact_proto::seal::open_envelope(&recipient_kp, &unwrapped).unwrap();
        assert_eq!(opened, take);
    }

    #[test]
    fn revocation_event_roundtrips_to_swap_id() {
        let (_kp, keys, _x) = identity(0x41);
        let ev = revocation_event("deadbeefcafe0011", &keys).unwrap();
        assert_eq!(ev.kind, Kind::EventDeletion);
        assert_eq!(
            revoked_offer_from_event(&ev).as_deref(),
            Some("deadbeefcafe0011")
        );
    }

    #[test]
    fn foreign_author_cannot_revoke_anothers_offer() {
        // Maker A signs a deletion pointing at maker B's coordinate — must be
        // ignored (a maker may only revoke offers it signed).
        let (_a_kp, a_keys, _a_x) = identity(0x41);
        let (_b_kp, _b_keys, b_x) = identity(0x42);
        let coordinate = format!("{OFFER_KIND}:{b_x}:beefbeef");
        let forged = EventBuilder::new(Kind::EventDeletion, "")
            .tag(Tag::parse(["a", &coordinate]).unwrap())
            .sign_with_keys(&a_keys)
            .unwrap();
        assert_eq!(revoked_offer_from_event(&forged), None);
    }

    #[test]
    fn snapshot_event_seals_to_self_and_roundtrips() {
        let (kp, keys, xonly) = identity(0x51);
        // A swap-state snapshot is an envelope sealed to OUR OWN identity.
        let snap = signed_offer(&kp); // any signed envelope works as the payload
        let blob = pact_proto::seal::seal_envelope(&xonly, &snap).unwrap();
        let dtag = snapshot_dtag(&snap.swap_id);
        let ev = snapshot_event(&blob, &dtag, &keys).unwrap();

        assert_eq!(ev.kind.as_u16(), SNAPSHOT_KIND);
        assert_eq!(ev.pubkey.to_hex(), xonly); // authored by us (not ephemeral)
        let tags: Vec<Vec<String>> = ev.tags.iter().map(|t| t.clone().to_vec()).collect();
        assert!(tags.iter().any(|t| t[0] == "d" && t[1] == dtag));
        // The d-tag is opaque — it is NOT the swap_id (so a relay can't link it
        // to our public offer for the same swap).
        assert_ne!(dtag, snap.swap_id);
        assert_eq!(dtag, snapshot_dtag(&snap.swap_id)); // deterministic

        // Only we can open it; the recovered blob decrypts to the snapshot.
        let got = snapshot_blob_from_event(&ev, &xonly).unwrap();
        assert_eq!(pact_proto::seal::open_envelope(&kp, &got).unwrap(), snap);

        // A foreign author's snapshot event is rejected.
        assert!(snapshot_blob_from_event(&ev, &identity(0x52).2).is_err());

        // Tombstone references our snapshot coordinate.
        let tomb = snapshot_tombstone_event(&dtag, &keys).unwrap();
        assert_eq!(tomb.kind, Kind::EventDeletion);
    }
}
