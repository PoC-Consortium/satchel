//! Relay ingest: poll-per-tick over a nostr-sdk relay pool, mirroring
//! pactd's proven `nostr_service` shape (fetch with `since` cursors rather
//! than long-lived subscriptions — sidesteps subscription/reconnect
//! lifecycle, and the announcer debounces anyway so sub-poll latency buys
//! nothing). Includes the #146 cursor clamp: a peer-controlled far-future
//! `created_at` can never poison a `since` cursor and deafen the bot.

use std::sync::{Arc, RwLock};
use std::time::Duration;

use anyhow::Result;
use nostr_sdk::prelude::*;
use pact_nostr as pn;

use crate::book::Book;
use crate::config::unix_now;
use crate::offer::offer_from_nostr_event;

const FETCH_TIMEOUT: Duration = Duration::from_secs(10);
/// #146: ceiling for cursor advancement (peer-controlled `created_at`).
const CURSOR_FUTURE_SKEW_SECS: u64 = 15 * 60;
/// How far back the first deletions fetch reaches. Offers only live ~30 min
/// on relays without a refresh, so older deletions cannot matter.
const DELETIONS_BACKFILL_SECS: u64 = 3600;

fn advance_cursor(cursor: u64, created: u64, now: u64) -> u64 {
    cursor.max(created.min(now + CURSOR_FUTURE_SKEW_SECS))
}

pub struct Ingest {
    pub client: Client,
    network: String,
}

impl Ingest {
    /// Best-effort connect (a relay that fails to add is logged and skipped),
    /// same as pactd.
    pub async fn connect(relays: &[String], network: &str) -> Result<Ingest> {
        let client = Client::default();
        for url in relays {
            if let Err(err) = client.add_relay(url).await {
                tracing::warn!("nostr: add_relay {url} failed: {err:#}");
            }
        }
        client.connect().await;
        Ok(Ingest {
            client,
            network: network.to_string(),
        })
    }

    async fn fetch(&self, filter: Filter) -> Vec<Event> {
        match self.client.fetch_events(filter, FETCH_TIMEOUT).await {
            Ok(events) => events.into_iter().collect(),
            Err(err) => {
                tracing::warn!("nostr: fetch failed: {err:#}");
                Vec::new()
            }
        }
    }

    /// Poll forever, feeding the shared book.
    pub async fn run(self, book: Arc<RwLock<Book>>, poll_secs: u64) {
        let mut offers_since: u64 = 0; // full backfill on first poll
        let mut deletions_since: u64 = unix_now().saturating_sub(DELETIONS_BACKFILL_SECS);
        let mut tick = tokio::time::interval(Duration::from_secs(poll_secs));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            let now = unix_now();

            // ---- offers (kind 31510) ----
            let events = self
                .fetch(pn::offers_filter().since(Timestamp::from(offers_since)))
                .await;
            let mut fresh = 0usize;
            {
                let mut book = match book.write() {
                    Ok(b) => b,
                    Err(_) => continue,
                };
                for ev in &events {
                    offers_since = advance_cursor(offers_since, ev.created_at.as_secs(), now);
                    match offer_from_nostr_event(ev, &self.network) {
                        Ok(offer) => {
                            if book.upsert(offer) {
                                fresh += 1;
                            }
                        }
                        Err(err) => tracing::debug!("nostr: skip offer event: {err:#}"),
                    }
                }
            }

            // ---- revocations (kind 5, ownership enforced client-side) ----
            let events = self
                .fetch(pn::deletions_filter().since(Timestamp::from(deletions_since)))
                .await;
            {
                let mut book = match book.write() {
                    Ok(b) => b,
                    Err(_) => continue,
                };
                for ev in &events {
                    deletions_since = advance_cursor(deletions_since, ev.created_at.as_secs(), now);
                    if let Some(swap_id) = pn::revoked_offer_from_event(ev) {
                        if book.revoke(&ev.pubkey.to_hex(), &swap_id, now) {
                            tracing::info!("offer {swap_id} revoked by its maker");
                        }
                    }
                }
                book.sweep(now);
                book.last_poll = now;
            }
            if fresh > 0 {
                tracing::debug!("ingest: {fresh} fresh offer event(s)");
            }
        }
    }
}
