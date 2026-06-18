//! Background Nostr relay client (docs/NOSTR_TRANSPORT.md, "The sync boundary").
//!
//! All async relay I/O is isolated here. Each scheduler tick runs three
//! steps, so the engine lock is only ever held for fast SQLite work and
//! never across a relay round-trip:
//!
//!   A. [`prep`]  — under the registry lock, read the active merchant's
//!      identity + pending outbox + fetch cursors from its store.
//!   B. [`NostrService::round`] — lock-free: publish the outbox to relays
//!      and fetch new offers and gift-wrap mailbox events.
//!   C. [`apply`] — under the lock again, write the results into the local
//!      `nostr_*` buffers and advance the cursors.
//!
//! Polling (fetch-per-tick) rather than long-lived subscriptions keeps this
//! aligned with how the engine already drives the HTTP relay each tick, and
//! sidesteps subscription/reconnect lifecycle. Cross-relay/overlap dups are
//! absorbed by the `event_id` uniqueness in the buffer tables.

use std::time::Duration;

use anyhow::{Context, Result};
use libswap::store::Store;
use nostr_sdk::prelude::*;
use pact_nostr as pn;

const FETCH_TIMEOUT: Duration = Duration::from_secs(10);

/// Snapshot read from the active merchant's store under the lock (step A).
pub struct Prep {
    secret_hex: String,
    me: String,
    /// `(outbox_id, kind, recipient, payload)` rows awaiting publication.
    outbox: Vec<(i64, String, Option<String>, String)>,
    offers_since: u64,
    mailbox_since: u64,
}

/// Step B's results, applied to the store under the lock (step C).
#[derive(Default)]
pub struct Apply {
    sent_outbox: Vec<i64>,
    inbox: Vec<(String, String, u64)>,
    offers: Vec<(String, String, String, u64, u64)>,
    offers_since: u64,
    mailbox_since: u64,
}

fn since(store: &Store, key: &str) -> u64 {
    store
        .meta_get(key)
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// Step A: read identity, pending outbox and cursors. Returns `None` when
/// the relay round should be skipped this tick (nostr not configured, or
/// the seed is locked / unreadable).
pub fn prep(store: &Store, nostr_configured: bool) -> Result<Option<Prep>> {
    if !nostr_configured {
        return Ok(None);
    }
    let seed = match store.seed() {
        Ok(seed) => seed,
        Err(_) => return Ok(None), // locked / no seed yet
    };
    let kp = seed.identity_keypair()?;
    Ok(Some(Prep {
        secret_hex: hex::encode(kp.secret_bytes()),
        me: kp.x_only_public_key().0.to_string(),
        outbox: store.nostr_outbox_pending()?,
        offers_since: since(store, "nostr_since:offers"),
        mailbox_since: since(store, "nostr_since:mailbox"),
    }))
}

/// Step C: persist a relay round's results and advance the cursors.
pub fn apply(store: &Store, a: &Apply) -> Result<()> {
    for id in &a.sent_outbox {
        store.nostr_outbox_mark_sent(*id)?;
    }
    for (event_id, blob, created) in &a.inbox {
        store.nostr_inbox_insert(event_id, blob, *created)?;
    }
    for (event_id, d_tag, envelope, created, expires) in &a.offers {
        store.nostr_offer_cache_upsert(event_id, d_tag, envelope, *created, *expires)?;
    }
    store.meta_set("nostr_since:offers", &a.offers_since.to_string())?;
    store.meta_set("nostr_since:mailbox", &a.mailbox_since.to_string())?;
    Ok(())
}

/// A relay-pool client. One instance per pactd process (relays are
/// process-level config shared across merchants); the per-merchant identity
/// is supplied per [`round`](Self::round) call.
pub struct NostrService {
    client: Client,
}

impl NostrService {
    /// Connect to a comma-separated `wss://…` relay list. Best-effort: a
    /// relay that fails to add is logged and skipped.
    pub async fn connect(relays: &str) -> Result<Self> {
        let client = Client::default();
        for url in relays.split(',').map(str::trim).filter(|u| !u.is_empty()) {
            if let Err(err) = client.add_relay(url).await {
                tracing::warn!("nostr: add_relay {url} failed: {err:#}");
            }
        }
        client.connect().await;
        Ok(Self { client })
    }

    /// Step B: publish the outbox and fetch new events. No store access.
    pub async fn round(&self, prep: &Prep) -> Apply {
        let mut out = Apply {
            offers_since: prep.offers_since,
            mailbox_since: prep.mailbox_since,
            ..Apply::default()
        };
        let keys = match pn::keys_from_secret_hex(&prep.secret_hex) {
            Ok(keys) => keys,
            Err(err) => {
                tracing::warn!("nostr: bad identity keys: {err:#}");
                return out;
            }
        };

        // ---- publish the outbox ----
        for (id, kind, recipient, payload) in &prep.outbox {
            match build_event(kind, recipient.as_deref(), payload, &keys) {
                // Fire-and-forget: the relay pool buffers + delivers once
                // connected, so an `Ok` means dispatched — mark it sent and move
                // on (re-publishing every tick would spam relays; genuine loss is
                // covered by the app-level re-post). A hard send error keeps the
                // row queued to retry next round.
                Some(event) => match self.client.send_event(&event).await {
                    Ok(_) => out.sent_outbox.push(*id),
                    Err(err) => tracing::warn!("nostr: send {kind} failed: {err:#}; will retry"),
                },
                // Unbuildable row: build_event already logged why. Drop it so a
                // permanently-malformed row can't wedge the queue.
                None => out.sent_outbox.push(*id),
            }
        }

        // ---- fetch offers (public, by kind) ----
        if let Ok(events) = self
            .fetch(pn::offers_filter().since(Timestamp::from(prep.offers_since)))
            .await
        {
            for ev in events {
                let created = ev.created_at.as_secs();
                if let Ok(env) = pn::offer_from_event(&ev) {
                    if let Ok(json) = serde_json::to_string(&env) {
                        out.offers.push((
                            ev.id.to_hex(),
                            env.swap_id.clone(),
                            json,
                            created,
                            expiration_of(&ev),
                        ));
                    }
                }
                out.offers_since = out.offers_since.max(created);
            }
        }

        // ---- fetch our gift-wrap mailbox ----
        if let Ok(filter) = pn::mailbox_filter(&prep.me) {
            if let Ok(events) = self
                .fetch(filter.since(Timestamp::from(prep.mailbox_since)))
                .await
            {
                for ev in events {
                    let created = ev.created_at.as_secs();
                    if let Ok(blob) = pn::unwrap_giftwrap(&ev) {
                        out.inbox.push((ev.id.to_hex(), blob, created));
                    }
                    out.mailbox_since = out.mailbox_since.max(created);
                }
            }
        }

        out
    }

    async fn fetch(&self, filter: Filter) -> Result<Vec<Event>> {
        let events = self.client.fetch_events(filter, FETCH_TIMEOUT).await?;
        Ok(events.into_iter().collect())
    }

    /// Per-relay connectivity for the header indicator: `(url, connected)`.
    pub async fn relay_status(&self) -> Vec<(String, bool)> {
        self.client
            .relays()
            .await
            .into_iter()
            .map(|(url, relay)| {
                (
                    url.to_string(),
                    matches!(relay.status(), RelayStatus::Connected),
                )
            })
            .collect()
    }
}

/// Map one outbox row to the Nostr event to publish. Logs (rather than silently
/// swallows) the reason a row can't be built, so a mapping/identity bug surfaces
/// instead of an offer quietly never reaching a relay.
fn build_event(kind: &str, recipient: Option<&str>, payload: &str, keys: &Keys) -> Option<Event> {
    let built: Result<Event> = (|| match kind {
        "offer" => {
            let env = serde_json::from_str(payload).context("parse offer payload")?;
            pn::offer_event(&env, keys)
        }
        "giftwrap" => pn::giftwrap(recipient.context("giftwrap row has no recipient")?, payload),
        "revoke" => {
            let v: serde_json::Value =
                serde_json::from_str(payload).context("parse revoke payload")?;
            let swap_id = v
                .get("swap_id")
                .and_then(|x| x.as_str())
                .context("revoke payload has no swap_id")?;
            pn::revocation_event(swap_id, keys)
        }
        other => anyhow::bail!("unknown outbox kind '{other}'"),
    })();
    match built {
        Ok(event) => Some(event),
        Err(err) => {
            tracing::warn!("nostr: dropping unbuildable {kind} row: {err:#}");
            None
        }
    }
}

/// Read the NIP-40 expiration (unix secs) from an event's tags, or 0.
fn expiration_of(ev: &Event) -> u64 {
    for tag in ev.tags.iter() {
        let s = tag.as_slice();
        if s.first().map(|k| k == "expiration").unwrap_or(false) {
            if let Some(secs) = s.get(1).and_then(|x| x.parse::<u64>().ok()) {
                return secs;
            }
        }
    }
    0
}

#[cfg(test)]
mod tests {
    //! End-to-end data-path test for the Nostr transport *without* a live
    //! relay: it drives a maker's outbox through the same `build_event` /
    //! pact-nostr mapping the relay round uses, hands the resulting events to
    //! a taker by hand, and checks the taker's `NostrBoard` buffers surface
    //! them. This covers everything except the websocket hop (that needs a
    //! relay binary — the live-relay e2e in docs/NOSTR_TRANSPORT.md).

    use super::*;
    use libswap::board::Noticeboard;
    use libswap::nostr_board::NostrBoard;
    use libswap::store::Store;
    use pact_proto::envelope::Envelope;

    struct Party {
        store: Store,
        keys: Keys,
        xonly: String,
    }

    fn party(tag: &str) -> Party {
        let dir = std::env::temp_dir().join(format!("pact-nostr-e2e-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = Store::init(&dir, None).unwrap();
        let kp = store.seed().unwrap().identity_keypair().unwrap();
        let keys = pn::keys_from_secret_hex(&hex::encode(kp.secret_bytes())).unwrap();
        let xonly = kp.x_only_public_key().0.to_string();
        Party { store, keys, xonly }
    }

    fn signed_offer(maker: &Party) -> Envelope {
        // `created = now` so the derived NIP-40 expiration is in the future
        // and the offer survives the active-cache filter.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut env = Envelope {
            v: 1,
            msg_type: "offer".into(),
            swap_id: "00aa11bb22cc33dd".into(),
            from: String::new(),
            body: serde_json::json!({
                "protocol": "pact-htlc-v1", "network": "regtest",
                "give_asset": "pocx", "give_amount": 1000u64,
                "get_asset": "btc", "get_amount": 10u64,
                "t1_secs": 28800u32, "t2_secs": 14400u32,
                "ttl_secs": 3600u64, "created": now,
            }),
            sig: String::new(),
        };
        let kp = maker.store.seed().unwrap().identity_keypair().unwrap();
        pact_proto::envelope::sign(&mut env, &kp).unwrap();
        env
    }

    #[test]
    fn offer_travels_maker_outbox_to_taker_board() {
        let maker = party("offer-maker");
        let taker = party("offer-taker");
        let offer = signed_offer(&maker);

        // Maker posts: lands in the outbox as an "offer" row.
        NostrBoard::new(&maker.store).post_offer(&offer).unwrap();
        let pending = maker.store.nostr_outbox_pending().unwrap();
        assert_eq!(pending.len(), 1);

        // The relay round maps the row to an event (same path as round()).
        let (_, kind, recipient, payload) = &pending[0];
        let event = build_event(kind, recipient.as_deref(), payload, &maker.keys).unwrap();

        // Taker receives the event and caches it (what apply() does).
        let parsed = pn::offer_from_event(&event).unwrap();
        assert_eq!(parsed, offer); // inner Pact envelope survived intact
        taker
            .store
            .nostr_offer_cache_upsert(
                &event.id.to_hex(),
                &parsed.swap_id,
                &serde_json::to_string(&parsed).unwrap(),
                event.created_at.as_secs(),
                expiration_of(&event),
            )
            .unwrap();

        // Taker's board now lists the maker's offer.
        let offers = NostrBoard::new(&taker.store).offers().unwrap();
        assert_eq!(offers.len(), 1);
        assert_eq!(offers[0].swap_id, offer.swap_id);
        assert_eq!(offers[0].from, maker.xonly);
    }

    #[test]
    fn giftwrapped_take_travels_to_taker_inbox() {
        // Maker sends a sealed envelope to the taker via a gift wrap.
        let maker = party("gw-maker");
        let taker = party("gw-taker");
        let payload_env = signed_offer(&maker); // any signed envelope as the relayed body

        let blob = libswap::board::seal_envelope(&taker.xonly, &payload_env).unwrap();
        NostrBoard::new(&maker.store)
            .relay_send_blob(&taker.xonly, &blob)
            .unwrap();
        let (_, kind, recipient, payload) = maker.store.nostr_outbox_pending().unwrap().remove(0);
        let event = build_event(&kind, recipient.as_deref(), &payload, &maker.keys).unwrap();
        assert_eq!(event.kind.as_u16(), pact_nostr::GIFTWRAP_KIND);
        // Author is ephemeral — not the maker (sender-hiding).
        assert_ne!(event.pubkey.to_hex(), maker.xonly);

        // Taker unwraps the nostr layer and buffers the inner sealed blob.
        let inner = pn::unwrap_giftwrap(&event).unwrap();
        taker
            .store
            .nostr_inbox_insert(&event.id.to_hex(), &inner, event.created_at.as_secs())
            .unwrap();

        // Taker's relay_poll returns it, and open_envelope recovers the original.
        let poll = Envelope {
            v: 1,
            msg_type: "relay_poll".into(),
            swap_id: "-".into(),
            from: String::new(),
            body: serde_json::json!({ "since_id": 0 }),
            sig: String::new(),
        };
        let mail = NostrBoard::new(&taker.store).relay_poll(&poll).unwrap();
        assert_eq!(mail.len(), 1);
        let taker_kp = taker.store.seed().unwrap().identity_keypair().unwrap();
        let opened = libswap::board::open_envelope(&taker_kp, &mail[0].1).unwrap();
        assert_eq!(opened, payload_env);
    }
}
