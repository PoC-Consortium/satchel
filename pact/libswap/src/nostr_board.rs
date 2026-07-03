//! `NostrBoard` — the sync [`Noticeboard`] facade over the local Nostr
//! buffers (spec/protocol.md §8.8).
//!
//! It performs no network I/O. Every operation reads or writes the
//! `nostr_*` SQLite tables; the async relay-pool service (Phase 3) drains
//! the outbox to relays and fills the inbox/caches from subscriptions.
//! Because the inbox uses a local autoincrement id, `relay_poll` returns
//! the exact `(i64, blob)` contract the HTTP board does, so the engine's
//! cursor/dispatch loop is shared unchanged.

use anyhow::Result;
use serde_json::Value;

use crate::board::Noticeboard;
use crate::engine::local_now;
use crate::messages::Envelope;
use crate::store::Store;

/// Borrows the engine's store; lives only for the duration of one
/// `Engine::boards()` call.
pub struct NostrBoard<'a> {
    store: &'a Store,
}

impl<'a> NostrBoard<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }
}

impl Noticeboard for NostrBoard<'_> {
    /// Queue the signed offer for publication as an addressable event; the
    /// service maps it via `pact_nostr::offer_event`. The offer id is the
    /// envelope's swap_id (same as the HTTP board returns).
    fn post_offer(&self, offer: &Envelope) -> Result<String> {
        self.store
            .nostr_outbox_push("offer", None, &serde_json::to_string(offer)?, local_now())?;
        Ok(offer.swap_id.clone())
    }

    /// Active cached offers discovered from relays (filled by the service).
    fn offers(&self) -> Result<Vec<Envelope>> {
        let now = local_now();
        let mut out = Vec::new();
        for envelope_json in self.store.nostr_offer_cache_active(now)? {
            if let Ok(env) = serde_json::from_str::<Envelope>(&envelope_json) {
                out.push(env);
            }
        }
        Ok(out)
    }

    /// Queue a revocation; the service publishes a deletion/replacement for
    /// the offer's d_tag and drops it from the local cache immediately so
    /// our own `offers()` stops listing it.
    fn revoke(&self, revocation: &Envelope) -> Result<()> {
        self.store.nostr_offer_cache_remove(&revocation.swap_id)?;
        self.store.nostr_outbox_push(
            "revoke",
            None,
            &serde_json::to_string(revocation)?,
            local_now(),
        )?;
        Ok(())
    }

    /// Queue a pre-sealed blob for gift-wrapping to `to`.
    fn relay_send_blob(&self, to: &str, blob: &str) -> Result<()> {
        self.store
            .nostr_outbox_push("giftwrap", Some(to), blob, local_now())?;
        Ok(())
    }

    /// Read our local inbox (filled by the service from `#p` gift-wraps)
    /// newer than the engine's cursor. `poll.body["since_id"]` carries the
    /// cursor, exactly as the HTTP board expects.
    fn relay_poll(&self, poll: &Envelope) -> Result<Vec<(i64, String)>> {
        let since = poll
            .body
            .get("since_id")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        self.store.nostr_inbox_since(since)
    }

    /// Queue an encrypted-to-self snapshot for publication as an addressable
    /// event (issue #54). Payload carries the `swap_id` (mapped to an opaque
    /// d-tag by the service, so it stays local) and the sealed blob; the
    /// service builds the event via `pact_nostr::snapshot_event`.
    fn publish_snapshot(&self, swap_id: &str, sealed_blob: &str) -> Result<()> {
        let payload = serde_json::json!({ "swap_id": swap_id, "blob": sealed_blob }).to_string();
        self.store
            .nostr_outbox_push("snapshot", None, &payload, local_now())?;
        Ok(())
    }

    /// Queue a snapshot tombstone (NIP-09 delete of the swap's snapshot
    /// coordinate); the service maps it via `pact_nostr::snapshot_tombstone_event`.
    fn tombstone_snapshot(&self, swap_id: &str) -> Result<()> {
        self.store
            .nostr_outbox_push("snapshot_tombstone", None, swap_id, local_now())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn store() -> Store {
        // A fresh store under a UNIQUE temp dir (no seed needed: NostrBoard
        // never touches keys). The dir is keyed by process id AND a per-call
        // counter so parallel tests can't collide on the same SQLite file —
        // keying on the process id alone made them race under CI's parallelism.
        use std::sync::atomic::{AtomicU32, Ordering};
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "pact-nostrboard-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        Store::open(&dir, None).unwrap()
    }

    fn offer_envelope(swap_id: &str) -> Envelope {
        Envelope {
            v: 1,
            msg_type: "offer".into(),
            swap_id: swap_id.into(),
            from: "aa".repeat(32),
            body: json!({ "give_asset": "pocx", "get_asset": "btc" }),
            sig: "bb".repeat(64),
        }
    }

    #[test]
    fn post_offer_queues_outbox_and_returns_swap_id() {
        let st = store();
        let board = NostrBoard::new(&st);
        let id = board.post_offer(&offer_envelope("dead")).unwrap();
        assert_eq!(id, "dead");
        let pending = st.nostr_outbox_pending().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].1, "offer"); // kind
    }

    #[test]
    fn offers_reads_cache_and_honors_expiry() {
        let st = store();
        let board = NostrBoard::new(&st);
        let now = local_now();
        // active offer + an already-expired one
        st.nostr_offer_cache_upsert(
            "e1",
            "live",
            &serde_json::to_string(&offer_envelope("live")).unwrap(),
            now,
            now + 9999,
        )
        .unwrap();
        st.nostr_offer_cache_upsert(
            "e2",
            "dead",
            &serde_json::to_string(&offer_envelope("dead")).unwrap(),
            now,
            now.saturating_sub(1),
        )
        .unwrap();
        let offers = board.offers().unwrap();
        assert_eq!(offers.len(), 1);
        assert_eq!(offers[0].swap_id, "live");
    }

    #[test]
    fn relay_poll_reads_inbox_by_cursor() {
        let st = store();
        let board = NostrBoard::new(&st);
        assert!(st
            .nostr_inbox_insert("ev1", "PACTSEALED1:blob-a", local_now())
            .unwrap());
        assert!(st
            .nostr_inbox_insert("ev2", "PACTSEALED1:blob-b", local_now())
            .unwrap());
        // dup event id is ignored
        assert!(!st
            .nostr_inbox_insert("ev1", "PACTSEALED1:blob-a", local_now())
            .unwrap());

        let poll = Envelope {
            v: 1,
            msg_type: "relay_poll".into(),
            swap_id: "-".into(),
            from: String::new(),
            body: json!({ "since_id": 0 }),
            sig: String::new(),
        };
        let mail = board.relay_poll(&poll).unwrap();
        assert_eq!(mail.len(), 2);
        // cursor past the first row returns only the second
        let poll2 = Envelope {
            body: json!({ "since_id": mail[0].0 }),
            ..poll
        };
        assert_eq!(board.relay_poll(&poll2).unwrap().len(), 1);
    }
}
