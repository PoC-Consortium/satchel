//! Shared command responses (our markdown dialect) — one source of truth for
//! every protocol front-end (Discord slash commands, Telegram commands).

use std::sync::{Arc, RwLock};

use anyhow::{bail, Context, Result};

use crate::book::Book;
use crate::cash::CashRate;
use crate::config::{unix_now, Config, Pair};
use crate::render::{fmt_usd, render_book, RenderCtx};

/// Depth shown per side — UI parity with the Corkboard's DEPTH_CAP.
const DEPTH: usize = 8;

/// The /book response. `arg` = user-supplied pair ("btcx/btc"), default =
/// first configured announce pair.
pub fn book_view(
    book: &Arc<RwLock<Book>>,
    cfg: &Config,
    cash: &CashRate,
    arg: Option<&str>,
) -> Result<String> {
    let pair = match arg {
        Some(s) => Pair::parse(s)?,
        None => cfg
            .announce
            .pairs
            .first()
            .cloned()
            .context("no pair given and none configured")?,
    };
    if !cfg.coins.contains_key(&pair.a) || !cfg.coins.contains_key(&pair.b) {
        bail!("unknown coin in pair {}/{}", pair.a, pair.b);
    }
    let ladder = book
        .read()
        .map_err(|_| anyhow::anyhow!("book lock"))?
        .ladder(&pair, DEPTH);
    let rctx = RenderCtx::for_pair(cfg, &pair, cash.fresh());
    Ok(render_book(&ladder, &rctx))
}

/// The /top response: best bid/ask for every pair on the board (commands are
/// not limited by the announce whitelist — only by renderability).
pub fn top_view(book: &Arc<RwLock<Book>>, cfg: &Config, cash: &CashRate) -> String {
    let Ok(guard) = book.read() else {
        return "book unavailable".to_string();
    };
    let known: Vec<Pair> = guard
        .pairs()
        .into_iter()
        .filter(|p| cfg.coins.contains_key(&p.a) && cfg.coins.contains_key(&p.b))
        .collect();
    if known.is_empty() {
        return "the board is empty".to_string();
    }
    let mut lines = Vec::new();
    for pair in &known {
        let ladder = guard.ladder(pair, 1);
        let rctx = RenderCtx::for_pair(cfg, pair, cash.fresh());
        let fmt_side = |lv: Option<&crate::book::Level>| match lv {
            Some(l) => format!(
                "{} {} @ {}",
                rctx.size_str(l.size_base_sats),
                rctx.base.symbol,
                rctx.price_str_natural(l.price)
            ),
            None => "—".to_string(),
        };
        lines.push(format!(
            "**{}** ({}) · bid {} · ask {}",
            rctx.pair_label(),
            rctx.unit_label(),
            fmt_side(ladder.bids.first()),
            fmt_side(ladder.asks.first()),
        ));
    }
    lines.join("\n")
}

/// The /status response: relay connectivity + book freshness + cash ref.
pub async fn status_view(
    book: &Arc<RwLock<Book>>,
    cfg: &Config,
    cash: &CashRate,
    nostr_client: &nostr_sdk::Client,
    started: u64,
) -> String {
    let mut relays = Vec::new();
    for (url, relay) in nostr_client.relays().await {
        let up = matches!(relay.status(), nostr_sdk::RelayStatus::Connected);
        relays.push(format!("{} {url}", if up { "🟢" } else { "🔴" }));
    }
    let (offers, last_poll) = match book.read() {
        Ok(b) => (b.len(), b.last_poll),
        Err(_) => (0, 0),
    };
    let now = unix_now();
    let poll_age = if last_poll == 0 {
        "never".to_string()
    } else {
        format!("{}s ago", now.saturating_sub(last_poll))
    };
    let cash_line = match cash.last() {
        Some((rate, age)) => format!("1 BTC ≈ ${} ({}s old)", fmt_usd(rate), age),
        None => "unavailable".to_string(),
    };
    let uptime_mins = now.saturating_sub(started) / 60;
    format!(
        "**crier** — network `{}` · {} offers tracked · last poll {} · up {}m\ncash ref: {}\n{}",
        cfg.network,
        offers,
        poll_age,
        uptime_mins,
        cash_line,
        relays.join("\n"),
    )
}
