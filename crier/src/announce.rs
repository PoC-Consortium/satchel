//! The announcer: diffs each whitelisted pair's top-of-book signature against
//! the last ANNOUNCED state, debounces churn, and posts through a sink
//! (Discord channel, or stdout in --dry-run).
//!
//! Restart hygiene: the last-announced signatures persist in a small state
//! file, and announcements are suppressed for an initial-sync grace period —
//! so a deploy never causes an announcement storm, only genuine diffs vs the
//! persisted state get posted.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use poise::serenity_prelude as serenity;
use serde::{Deserialize, Serialize};

use crate::book::{Book, TopSig};
use crate::cash::CashRate;
use crate::config::{unix_now, Config};
use crate::render::{render_announcement, Announcement, RenderCtx};

#[derive(Debug, Default, Serialize, Deserialize)]
struct StateFile {
    /// Last announced signature per pair key ("a/b" normalized).
    pairs: HashMap<String, PairState>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PairState {
    sig: TopSig,
    last_post: u64,
}

fn load_state(path: &PathBuf) -> StateFile {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|err| {
            tracing::warn!(
                "state file {} unreadable ({err}) — starting fresh",
                path.display()
            );
            StateFile::default()
        }),
        Err(_) => StateFile::default(),
    }
}

fn save_state(path: &PathBuf, state: &StateFile) {
    match serde_json::to_string_pretty(state) {
        Ok(json) => {
            if let Err(err) = std::fs::write(path, json) {
                tracing::warn!("state file {} not writable: {err}", path.display());
            }
        }
        Err(err) => tracing::warn!("state serialize: {err}"),
    }
}

/// Where announcements go. The announcer fans one rendered announcement out
/// to every configured sink (Discord channel, Telegram chat, stdout).
pub enum Sink {
    /// --dry-run: print what WOULD be posted.
    Stdout,
    Discord {
        http: Arc<serenity::Http>,
        channel_id: u64,
    },
    Telegram {
        tg: Arc<crate::telegram::Telegram>,
        chat_id: String,
    },
}

impl Sink {
    async fn post(&self, a: &Announcement) {
        match self {
            Sink::Stdout => {
                println!("\n=== ANNOUNCE ===============================");
                println!("{}", a.title);
                println!("{}", a.body);
                println!("[{}]", a.footer);
                println!("============================================");
            }
            Sink::Discord { http, channel_id } => {
                let embed = serenity::CreateEmbed::new()
                    .title(a.title.clone())
                    .description(a.body.clone())
                    .footer(serenity::CreateEmbedFooter::new(a.footer.clone()));
                let msg = serenity::CreateMessage::new().embed(embed);
                if let Err(err) = serenity::ChannelId::new(*channel_id)
                    .send_message(http.as_ref(), msg)
                    .await
                {
                    tracing::warn!("discord: announce post failed: {err:#}");
                }
            }
            Sink::Telegram { tg, chat_id } => {
                let html = format!(
                    "<b>{}</b>\n{}\n<i>{}</i>",
                    crate::telegram::md_to_html(&a.title),
                    crate::telegram::md_to_html(&a.body),
                    crate::telegram::md_to_html(&a.footer),
                );
                if let Err(err) = tg.send_html(chat_id, &html).await {
                    tracing::warn!("telegram: announce post failed: {err:#}");
                }
            }
        }
    }
}

/// Pure debounce/interval decision for one pair, one tick. Returns whether to
/// post now. `pending_since` is the tick time when the current divergence was
/// first seen (None = book matches the announced state).
#[allow(clippy::too_many_arguments)]
fn should_post(
    announced: &TopSig,
    current: &TopSig,
    pending_since: &mut Option<u64>,
    last_post: u64,
    now: u64,
    debounce_secs: u64,
    min_interval_secs: u64,
    min_size_delta_pct: u64,
) -> bool {
    if !announced.changed(current, min_size_delta_pct) {
        *pending_since = None; // churn settled back — never announce
        return false;
    }
    let since = *pending_since.get_or_insert(now);
    now.saturating_sub(since) >= debounce_secs && now.saturating_sub(last_post) >= min_interval_secs
}

pub async fn run(
    book: Arc<RwLock<Book>>,
    cfg: Arc<Config>,
    cash: CashRate,
    sinks: Vec<Sink>,
    persist_state: bool,
) {
    let started = unix_now();
    let mut state = if persist_state {
        load_state(&cfg.state_file)
    } else {
        StateFile::default()
    };
    let mut pending: HashMap<String, Option<u64>> = HashMap::new();

    let mut tick = tokio::time::interval(Duration::from_secs(5));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        let now = unix_now();
        if now.saturating_sub(started) < cfg.announce.initial_sync_secs {
            continue; // initial relay sync — judge diffs only once settled
        }
        for pair in &cfg.announce.pairs {
            let key = format!("{}/{}", pair.a, pair.b);
            let current = match book.read() {
                Ok(b) => b.top_sig(pair),
                Err(_) => continue,
            };
            let entry = state.pairs.entry(key.clone()).or_insert_with(|| PairState {
                sig: TopSig::default(),
                last_post: 0,
            });
            let pending_since = pending.entry(key).or_insert(None);
            if should_post(
                &entry.sig,
                &current,
                pending_since,
                entry.last_post,
                now,
                cfg.announce.debounce_secs,
                cfg.announce.min_interval_secs,
                cfg.announce.min_size_delta_pct,
            ) {
                let ctx = RenderCtx::for_pair(&cfg, pair, cash.fresh());
                let announcement = render_announcement(&entry.sig, &current, &ctx);
                for sink in &sinks {
                    sink.post(&announcement).await;
                }
                entry.sig = current;
                entry.last_post = now;
                *pending_since = None;
                if persist_state {
                    save_state(&cfg.state_file, &state);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::book::SideTop;

    fn sig(ask_num: u64) -> TopSig {
        TopSig {
            ask: Some(SideTop {
                num: ask_num,
                den: 100_000_000,
                size_base_sats: 100_000_000,
            }),
            bid: None,
        }
    }

    #[test]
    fn debounce_and_interval_gate_posts() {
        let announced = sig(69_100);
        let current = sig(67_600);
        let mut pending = None;

        // First sighting: pending starts, no post before debounce.
        assert!(!should_post(
            &announced,
            &current,
            &mut pending,
            0,
            1000,
            30,
            60,
            10
        ));
        assert_eq!(pending, Some(1000));
        // Still inside debounce.
        assert!(!should_post(
            &announced,
            &current,
            &mut pending,
            0,
            1020,
            30,
            60,
            10
        ));
        // Debounce passed, min-interval passed → post.
        assert!(should_post(
            &announced,
            &current,
            &mut pending,
            0,
            1031,
            30,
            60,
            10
        ));

        // Churn that settles back: pending clears, nothing posts.
        let mut pending = Some(1000);
        assert!(!should_post(
            &announced,
            &announced.clone(),
            &mut pending,
            0,
            1031,
            30,
            60,
            10
        ));
        assert_eq!(pending, None);

        // Min-interval floor: change confirmed but a post just went out.
        let mut pending = None;
        assert!(!should_post(
            &announced,
            &current,
            &mut pending,
            1000,
            1031,
            30,
            60,
            10
        ));
        assert!(!should_post(
            &announced,
            &current,
            &mut pending,
            1000,
            1059,
            30,
            60,
            10
        ));
        assert!(should_post(
            &announced,
            &current,
            &mut pending,
            1000,
            1061,
            30,
            60,
            10
        ));
    }
}
