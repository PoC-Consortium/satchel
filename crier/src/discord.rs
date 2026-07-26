//! Discord side: poise slash commands (/book, /top, /status) + gateway
//! bootstrap. Read-only by construction — the only write path is the
//! announcer posting into its configured channel.

use std::sync::{Arc, RwLock};

use anyhow::{Context as _, Result};
use poise::serenity_prelude as serenity;

use crate::book::Book;
use crate::cash::CashRate;
use crate::config::{unix_now, Config, Pair};
use crate::render::{fmt_usd, render_book, RenderCtx};

pub struct Data {
    pub book: Arc<RwLock<Book>>,
    pub cfg: Arc<Config>,
    pub cash: CashRate,
    pub ingest_client: nostr_sdk::Client,
    pub started: u64,
}

type Error = anyhow::Error;
type Ctx<'a> = poise::Context<'a, Data, Error>;

/// Depth shown per side — UI parity with the Corkboard's DEPTH_CAP.
const DEPTH: usize = 8;

fn resolve_pair(data: &Data, arg: Option<String>) -> Result<Pair> {
    match arg {
        Some(s) => Pair::parse(&s),
        None => data
            .cfg
            .announce
            .pairs
            .first()
            .cloned()
            .context("no pair given and none configured"),
    }
}

/// Show the order book for a pair.
#[poise::command(slash_command)]
async fn book(
    ctx: Ctx<'_>,
    #[description = "Pair like btcx/btc (default: first configured)"] pair: Option<String>,
) -> Result<(), Error> {
    let data = ctx.data();
    let pair = resolve_pair(data, pair)?;
    if !data.cfg.coins.contains_key(&pair.a) || !data.cfg.coins.contains_key(&pair.b) {
        ctx.say("unknown coin in pair").await?;
        return Ok(());
    }
    let ladder = data
        .book
        .read()
        .map_err(|_| anyhow::anyhow!("book lock"))?
        .ladder(&pair, DEPTH);
    let rctx = RenderCtx::for_pair(&data.cfg, &pair, data.cash.fresh());
    ctx.say(render_book(&ladder, &rctx)).await?;
    Ok(())
}

/// Best bid/ask for every pair on the board.
#[poise::command(slash_command)]
async fn top(ctx: Ctx<'_>) -> Result<(), Error> {
    let data = ctx.data();
    let pairs = data
        .book
        .read()
        .map_err(|_| anyhow::anyhow!("book lock"))?
        .pairs();
    // Commands browse ALL pairs on the board, but only ones whose coins we
    // can render (both ids known).
    let known: Vec<Pair> = pairs
        .into_iter()
        .filter(|p| data.cfg.coins.contains_key(&p.a) && data.cfg.coins.contains_key(&p.b))
        .collect();
    if known.is_empty() {
        ctx.say("the board is empty").await?;
        return Ok(());
    }
    let mut lines = Vec::new();
    for pair in &known {
        let ladder = data
            .book
            .read()
            .map_err(|_| anyhow::anyhow!("book lock"))?
            .ladder(pair, 1);
        let rctx = RenderCtx::for_pair(&data.cfg, pair, data.cash.fresh());
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
    ctx.say(lines.join("\n")).await?;
    Ok(())
}

/// Relay connectivity and book freshness.
#[poise::command(slash_command)]
async fn status(ctx: Ctx<'_>) -> Result<(), Error> {
    let data = ctx.data();
    let relays = {
        let mut out = Vec::new();
        for (url, relay) in data.ingest_client.relays().await {
            let up = matches!(relay.status(), nostr_sdk::RelayStatus::Connected);
            out.push(format!("{} {url}", if up { "🟢" } else { "🔴" }));
        }
        out
    };
    let (offers, last_poll) = {
        let book = data.book.read().map_err(|_| anyhow::anyhow!("book lock"))?;
        (book.len(), book.last_poll)
    };
    let now = unix_now();
    let poll_age = if last_poll == 0 {
        "never".to_string()
    } else {
        format!("{}s ago", now.saturating_sub(last_poll))
    };
    let cash = match data.cash.last() {
        Some((rate, age)) => format!("1 BTC ≈ ${} ({}s old)", fmt_usd(rate), age),
        None => "unavailable".to_string(),
    };
    let uptime_mins = now.saturating_sub(data.started) / 60;
    ctx.say(format!(
        "**crier** — network `{}` · {} offers tracked · last poll {} · up {}m\ncash ref: {}\n{}",
        data.cfg.network,
        offers,
        poll_age,
        uptime_mins,
        cash,
        relays.join("\n"),
    ))
    .await?;
    Ok(())
}

/// Run the gateway + slash commands; also spawns the announcer once the
/// client is built (it shares the client's HTTP handle).
pub async fn run(
    cfg: Arc<Config>,
    book_state: Arc<RwLock<Book>>,
    cash: CashRate,
    ingest_client: nostr_sdk::Client,
) -> Result<()> {
    let token = cfg
        .discord
        .token
        .clone()
        .context("no Discord token — set CRIER_DISCORD_TOKEN (or run --dry-run)")?;
    let guild_id = cfg.discord.guild_id;
    let data = Data {
        book: book_state.clone(),
        cfg: cfg.clone(),
        cash: cash.clone(),
        ingest_client,
        started: unix_now(),
    };

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![book(), top(), status()],
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                if guild_id != 0 {
                    poise::builtins::register_in_guild(
                        ctx,
                        &framework.options().commands,
                        serenity::GuildId::new(guild_id),
                    )
                    .await?;
                    tracing::info!("slash commands registered in guild {guild_id}");
                } else {
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                    tracing::info!("slash commands registered globally");
                }
                Ok(data)
            })
        })
        .build();

    // Read-only bot: no message-content or member intents needed.
    let mut client =
        serenity::ClientBuilder::new(&token, serenity::GatewayIntents::non_privileged())
            .framework(framework)
            .await
            .context("discord client build")?;

    if cfg.discord.announce_channel_id != 0 {
        let sink = crate::announce::Sink::Discord {
            http: client.http.clone(),
            channel_id: cfg.discord.announce_channel_id,
        };
        tokio::spawn(crate::announce::run(
            book_state,
            cfg.clone(),
            cash,
            sink,
            true,
        ));
    } else {
        tracing::info!("discord.announce_channel_id = 0 — announcer disabled, commands only");
    }

    client.start().await.context("discord gateway")
}
