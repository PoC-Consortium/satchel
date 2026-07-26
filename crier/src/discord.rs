//! Discord side: poise slash commands (/book, /top, /status) + gateway
//! bootstrap. Read-only by construction — the only write path is the
//! announcer posting into its configured channel (spawned from main, which
//! owns the sink fan-out across protocols).

use std::sync::{Arc, RwLock};

use anyhow::{Context as _, Result};
use poise::serenity_prelude as serenity;

use crate::book::Book;
use crate::cash::CashRate;
use crate::config::{unix_now, Config};
use crate::views;

pub struct Data {
    pub book: Arc<RwLock<Book>>,
    pub cfg: Arc<Config>,
    pub cash: CashRate,
    pub ingest_client: nostr_sdk::Client,
    pub started: u64,
}

type Error = anyhow::Error;
type Ctx<'a> = poise::Context<'a, Data, Error>;

/// Show the order book for a pair.
#[poise::command(slash_command)]
async fn book(
    ctx: Ctx<'_>,
    #[description = "Pair like btcx/btc (default: first configured)"] pair: Option<String>,
) -> Result<(), Error> {
    let data = ctx.data();
    let reply = match views::book_view(&data.book, &data.cfg, &data.cash, pair.as_deref()) {
        Ok(md) => md,
        Err(err) => format!("{err:#}"),
    };
    ctx.say(reply).await?;
    Ok(())
}

/// Best bid/ask for every pair on the board.
#[poise::command(slash_command)]
async fn top(ctx: Ctx<'_>) -> Result<(), Error> {
    let data = ctx.data();
    ctx.say(views::top_view(&data.book, &data.cfg, &data.cash))
        .await?;
    Ok(())
}

/// Relay connectivity and book freshness.
#[poise::command(slash_command)]
async fn status(ctx: Ctx<'_>) -> Result<(), Error> {
    let data = ctx.data();
    ctx.say(
        views::status_view(
            &data.book,
            &data.cfg,
            &data.cash,
            &data.ingest_client,
            data.started,
        )
        .await,
    )
    .await?;
    Ok(())
}

/// Run the gateway + slash commands (blocks until shutdown).
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
        book: book_state,
        cfg,
        cash,
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

    client.start().await.context("discord gateway")
}
