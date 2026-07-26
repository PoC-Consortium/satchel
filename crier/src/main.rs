//! crier — read-only Discord announcer for the Pact orderbook on Nostr.
//! See PLAN.md for the design and README.md for deployment.

mod announce;
mod book;
mod cash;
mod config;
mod discord;
mod ingest;
mod offer;
mod render;

use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use anyhow::Result;
use clap::Parser;

use crate::book::Book;
use crate::cash::CashRate;
use crate::config::Config;
use crate::render::{render_book, RenderCtx};

#[derive(Parser)]
#[command(name = "crier", about = "Cry the Pact orderbook into Discord")]
struct Args {
    /// Path to crier.toml (missing file = defaults: mainnet, btc/btcx).
    #[arg(long, default_value = "crier.toml")]
    config: PathBuf,
    /// No Discord: print the live book and would-be announcements to stdout.
    #[arg(long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,serenity=warn".into()),
        )
        .init();

    // rustls 0.23 links both aws-lc-rs and ring via our dependency graph and
    // refuses to auto-pick a provider — without this every wss:// relay
    // handshake fails (same footgun pactd hit).
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let args = Args::parse();
    let cfg = Arc::new(Config::load(&args.config)?);
    tracing::info!(
        "crier starting: network {}, {} relay(s), pairs {}",
        cfg.network,
        cfg.relays.len(),
        cfg.announce
            .pairs
            .iter()
            .map(|p| format!("{}/{}", p.a, p.b))
            .collect::<Vec<_>>()
            .join(", ")
    );

    let book = Arc::new(RwLock::new(Book::new()));
    let cash = CashRate::new(&cfg.cash);
    if cfg.cash.enabled {
        tokio::spawn(cash.clone().run(cfg.cash.refresh_secs));
    }

    let ingest = ingest::Ingest::connect(&cfg.relays, &cfg.network).await?;

    let nostr_client = ingest.client.clone();
    tokio::spawn(ingest.run(book.clone(), cfg.poll_secs));

    if args.dry_run {
        tokio::spawn(dry_run_printer(book.clone(), cfg.clone(), cash.clone()));
        // Dry-run announcer: stdout sink, in-memory state (every run starts
        // fresh so the review always sees the first announcements).
        tokio::spawn(announce::run(
            book,
            cfg,
            cash,
            announce::Sink::Stdout,
            false,
        ));
        tokio::signal::ctrl_c().await?;
        println!("\nbye");
        return Ok(());
    }

    discord::run(cfg, book, cash, nostr_client).await
}

/// --dry-run: print each configured pair's ladder whenever the book changed.
async fn dry_run_printer(book: Arc<RwLock<Book>>, cfg: Arc<Config>, cash: CashRate) {
    let mut last_rev = u64::MAX;
    let mut tick = tokio::time::interval(Duration::from_secs(15));
    loop {
        tick.tick().await;
        let rev = match book.read() {
            Ok(b) => b.rev,
            Err(_) => continue,
        };
        if rev == last_rev {
            continue;
        }
        last_rev = rev;
        let snapshot = match book.read() {
            Ok(b) => {
                let mut out = Vec::new();
                for pair in &cfg.announce.pairs {
                    out.push((pair.clone(), b.ladder(pair, 8)));
                }
                (out, b.len())
            }
            Err(_) => continue,
        };
        println!("\n--- book rev {rev} ({} offers tracked) ---", snapshot.1);
        for (pair, ladder) in snapshot.0 {
            let ctx = RenderCtx::for_pair(&cfg, &pair, cash.fresh());
            println!("{}", render_book(&ladder, &ctx));
        }
    }
}
