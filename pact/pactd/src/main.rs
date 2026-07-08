//! pactd — the Pact daemon (Bitcoin-Core-shaped).
//!
//! Exposes the `libswap` engine over **JSON-RPC 2.0** (POST `/`), runs the
//! scheduler (auto-redeem/refund/fee-bump with no human present), and
//! coordinates over the Corkboard relay. Auth mirrors bitcoind: an
//! auto-generated `.cookie` (HTTP Basic, the zero-config local default)
//! and/or `rpcuser`/`rpcpassword` from `<datadir>/pact.conf`.
//!
//! Analogue: `pactd` ≈ `bitcoind`, `pact-cli` ≈ `bitcoin-cli`,
//! Satchel ≈ `bitcoin-qt`. There is no web UI here — clients read the
//! cookie from the filesystem (the CLI, Satchel); a browser cannot.
//!
//! Modes:
//!   pactd --data-dir D ...            : serve JSON-RPC + scheduler
//!   pactd --data-dir D ... --once     : one scheduler pass, print, exit

mod merchants;
mod nostr_service;

use anyhow::{anyhow, bail, ensure, Context, Result};
use axum::extract::{Request, State};
use axum::http::{header::AUTHORIZATION, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use clap::Parser;
use libswap::chain::SendFee;
use libswap::engine::Engine;
use libswap::messages::Envelope;
use libswap::params::{parse_coin_amount, Network};
use merchants::{EngineConfig, MerchantRegistry};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;

const COOKIE_FILE: &str = ".cookie";
const CONF_FILE: &str = "pact.conf";

/// Initialise tracing to BOTH stdout and a rolling daily file under
/// `<data_dir>/logs/pactd.log` (RC2: managed Satchel discards stdout, so a file
/// is the only way devs see what the engine did). The returned guard flushes
/// the non-blocking file writer on drop — keep it alive for the whole process.
/// The log carries config/scheduler narration only (txids, states); it never
/// logs secrets (seed/preimage/nonces are never passed to `tracing`). Falls back
/// to stdout-only if the log dir can't be created.
fn init_logging(data_dir: &Path) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::{fmt, EnvFilter};

    let filter = || EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let log_dir = data_dir.join("logs");
    match std::fs::create_dir_all(&log_dir) {
        Ok(()) => {
            let file = tracing_appender::rolling::daily(&log_dir, "pactd.log");
            let (non_blocking, guard) = tracing_appender::non_blocking(file);
            tracing_subscriber::registry()
                .with(filter())
                .with(fmt::layer())
                .with(fmt::layer().with_ansi(false).with_writer(non_blocking))
                .init();
            Some(guard)
        }
        Err(e) => {
            tracing_subscriber::registry()
                .with(filter())
                .with(fmt::layer())
                .init();
            eprintln!(
                "warning: pactd file logging disabled ({}): {e}",
                log_dir.display()
            );
            None
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "pactd", version, about = "Pact swap daemon (PoCX trading)")]
struct Args {
    /// Data directory (seed, SQLite state, .cookie, pact.conf). Defaults to
    /// the platform data dir — %APPDATA%\Pact on Windows, ~/Library/
    /// Application Support/Pact on macOS, ~/.pact elsewhere — nested per
    /// network (mainnet at the root, testnet/regtest beneath), bitcoind-style.
    #[arg(long)]
    data_dir: Option<PathBuf>,
    /// Optional coin-templates file (`coins.toml`) that adds coins beyond the
    /// two built-ins (btcx, btc). Loaded at startup and merged with the
    /// built-ins (a file coin that collides with a built-in id is dropped).
    /// Satchel passes the file it ships next to the executables.
    #[arg(long)]
    coins_file: Option<PathBuf>,
    /// Per-coin chain-data backend, repeatable: `--coin <coin_id>=<url[,url]>`
    /// (e.g. `--coin btcx=http://user:pass@host:port/wallet/x`). The first URL
    /// is the wallet-qualified Core-RPC primary (funds swaps); the rest may be
    /// Electrum (`tcp://`/`ssl://`). This is the one generic way to attach a
    /// chain — every coin (built-in or file-added) is wired the same way; pass
    /// the flag once per coin to run a multi-coin engine.
    #[arg(long = "coin")]
    coins: Vec<String>,
    /// Per-coin confirmation depth (reorg-safety/finality), repeatable:
    /// `--coin-confs <coin_id>=<N>` (e.g. `--coin-confs btc=3`). The number of
    /// confirmations before a funding/redeem on that coin is treated final;
    /// gates auto-redeem + completion in v1 and v2. Omitted coins use the
    /// network/spacing default. Satchel's Coins setup page passes these.
    #[arg(long = "coin-confs")]
    coin_confs: Vec<String>,
    /// Listen address for the JSON-RPC endpoint. Loopback only.
    #[arg(long, default_value = "127.0.0.1:9737")]
    listen: std::net::SocketAddr,
    /// Network: regtest | testnet | mainnet.
    #[arg(long, default_value = "regtest")]
    network: String,
    /// Corkboard base URL(s), comma-separated.
    #[arg(long)]
    board_url: Option<String>,
    /// Nostr relay URL(s), comma-separated `wss://…`. When set, a
    /// decentralized Nostr transport runs alongside any --board-url
    /// (spec/protocol.md §8.8). Empty/absent disables it.
    #[arg(long)]
    nostr_relay: Option<String>,
    /// Auto-fund our HTLC leg in board-driven swaps (griefing trade-off).
    #[arg(long)]
    auto_fund: bool,
    /// Scheduler interval (s); 0 disables the background loop.
    #[arg(long, default_value_t = 30)]
    tick_secs: u64,
    /// One scheduler pass, print events as JSON, exit.
    #[arg(long)]
    once: bool,
    /// Create the seed + state on first run (for launchers like Satchel).
    #[arg(long)]
    auto_init: bool,
    /// Own merchants under `<data-dir>/merchants/<id>/` with an in-process
    /// registry (C10 — the Bitcoin-Core wallet layout). Satchel passes this so
    /// it can create/switch merchants at runtime. Without it pactd runs the
    /// legacy *flat* layout (one seed in the data-dir root), which the harness
    /// and `pact-cli` rely on. Ignored once a flat seed already exists.
    #[arg(long)]
    merchants: bool,
}

#[derive(Clone)]
struct App {
    /// pactd owns merchants (C10): the registry holds the manifest + the **one**
    /// loaded merchant's engine (Phase 1). Engine work goes through the active
    /// merchant; the `*merchant` RPCs mutate the registry itself.
    registry: Arc<Mutex<MerchantRegistry>>,
    network: Network,
    /// Expected `Authorization` header values (cookie and/or pact.conf).
    auth_headers: Arc<Vec<String>>,
    shutdown: Arc<Notify>,
    /// Set by `stop skip_delist=true` (Satchel's config-change relaunch) so the
    /// clean-shutdown path SKIPS the soft de-list — surviving offers keep their
    /// relay listings across the ~2s relaunch instead of being pulled and then
    /// re-read as a self-revocation on boot (#97). A plain `stop`/ctrl_c leaves it
    /// false, so a genuine close still de-lists.
    skip_delist_on_stop: Arc<std::sync::atomic::AtomicBool>,
    /// The process-wide Nostr relay client (None when no relays configured).
    /// Carried here so the `boardstatus` RPC can report relay connectivity and
    /// so merchant-load can kick an immediate relay pass (no wait for a tick).
    nostr: Option<Arc<nostr_service::NostrService>>,
}

/// Run blocking engine work off the async runtime (SQLite + sync RPC), against
/// the **active merchant's** engine. Errors clearly when no merchant is loaded.
async fn blocking<T, F>(app: &App, work: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce(&Engine) -> Result<T> + Send + 'static,
{
    let registry = app.registry.clone();
    tokio::task::spawn_blocking(move || -> Result<T> {
        let reg = registry.lock().expect("registry mutex poisoned");
        work(reg.active()?)
    })
    .await
    .map_err(|e| anyhow!("task panicked: {e}"))?
}

/// Same as [`blocking`] but for seed-lifecycle work that mutates the engine
/// (createseed / importseed / unlock set the in-memory passphrase). After the
/// work runs, the registry re-captures the active merchant's identity/lock
/// state into the manifest (so a freshly provisioned/unlocked seed shows up in
/// listmerchants without a reload).
async fn blocking_mut<T, F>(app: &App, work: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce(&mut Engine) -> Result<T> + Send + 'static,
{
    let registry = app.registry.clone();
    tokio::task::spawn_blocking(move || -> Result<T> {
        let mut reg = registry.lock().expect("registry mutex poisoned");
        let out = work(reg.active_mut()?)?;
        // Best-effort manifest refresh — never fail the RPC on a metadata write.
        let _ = reg.refresh_active_identity();
        Ok(out)
    })
    .await
    .map_err(|e| anyhow!("task panicked: {e}"))?
}

/// Run blocking work that needs the **registry itself** (the `*merchant` RPCs:
/// create/load/unload/list/info — they may swap the active engine in-process).
async fn blocking_registry<T, F>(app: &App, work: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce(&mut MerchantRegistry) -> Result<T> + Send + 'static,
{
    let registry = app.registry.clone();
    tokio::task::spawn_blocking(move || -> Result<T> {
        work(&mut registry.lock().expect("registry mutex poisoned"))
    })
    .await
    .map_err(|e| anyhow!("task panicked: {e}"))?
}

/// One Nostr relay round for the active merchant: read identity + outbox +
/// cursors under the lock (A), publish/fetch lock-free (B), write results
/// back under the lock (C). Keeps the engine lock off the relay round-trip.
async fn nostr_pass(app: &App, svc: &nostr_service::NostrService) -> Result<()> {
    let prep = blocking(app, |e| {
        let configured = e
            .nostr_relays
            .as_deref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        nostr_service::prep(&e.store, configured)
    })
    .await?;
    let Some(prep) = prep else { return Ok(()) }; // locked / not configured
    let apply = svc.round(&prep).await;
    blocking(app, move |e| nostr_service::apply(&e.store, &apply)).await
}

/// Kick one Nostr relay pass right now (best-effort, FIRE-AND-FORGET) — called
/// after a merchant becomes usable (load / seed provisioned / unlocked) so its
/// offers populate soon instead of waiting up to a full scheduler tick. Spawned
/// on the runtime so the calling RPC (importseed/loadmerchant/unlock) returns at
/// once: a relay round is a network round-trip to every configured relay, two
/// fetches each up to FETCH_TIMEOUT (10s) — tens of seconds on cold/slow relays
/// — and must NEVER block the UI (this caused a ~30s hang after seed creation).
/// The scheduler's next tick runs the pass regardless. No-op when unconfigured.
fn kick_nostr(app: &App) {
    let Some(svc) = app.nostr.clone() else {
        return;
    };
    let app = app.clone();
    tokio::spawn(async move {
        if let Err(err) = nostr_pass(&app, &svc).await {
            tracing::warn!("nostr: on-load pass failed: {err:#}");
        }
        // Seed-only rescue (#54), DETECTION ONLY: report any in-flight swap
        // this machine is missing from our own encrypted relay snapshots, but
        // never adopt one silently — if the machine that ran it is still alive,
        // two drivers on one seed can double-fund the same swap. Adoption is
        // the explicit `restorefromrelay` RPC / `pact-cli restore`.
        match detect_rescue(&app).await {
            Ok((n, _)) if n > 0 => {
                tracing::warn!(count = n, "rescue: {}", RESCUE_PENDING_WARNING)
            }
            Ok(_) => {}
            Err(err) => tracing::warn!("rescue: relay detection failed: {err:#}"),
        }
    });
}

/// The standing #54 warning attached to every rescue surface: detection log,
/// `rescuestatus` RPC and the CLI. One string so the wording never diverges.
const RESCUE_PENDING_WARNING: &str = "in-flight swap snapshot(s) found on the relays but NOT \
     auto-restored — run `restorefromrelay` ONLY if the machine that ran them is retired; \
     driving the same swap from two live machines can double-fund it and lose money";

/// Fetch our encrypted-to-self rescue snapshots from the relays and adopt any
/// swap we don't already have locally (#54). Our identity xonly and the adopt
/// are read/written under the registry lock; the relay fetch is lock-free in
/// between. Returns `(restored, seen)`. Errors only when the seed is
/// locked/unreadable or no relay transport is configured.
async fn restore_from_relay(app: &App) -> Result<(usize, usize)> {
    let blobs = fetch_rescue_blobs(app).await?;
    if blobs.is_empty() {
        return Ok((0, 0));
    }
    blocking_mut(app, move |e| e.rescue_from_blobs(&blobs)).await
}

/// Read-only twin of [`restore_from_relay`] (#54): count the snapshots that
/// WOULD be adopted — `(pending, seen)` — adopting nothing. The detection half
/// of the gated rescue; `rescuestatus` and the on-load/boot hooks use it.
async fn detect_rescue(app: &App) -> Result<(usize, usize)> {
    let blobs = fetch_rescue_blobs(app).await?;
    if blobs.is_empty() {
        return Ok((0, 0));
    }
    blocking(app, move |e| e.rescue_preview(&blobs)).await
}

async fn fetch_rescue_blobs(app: &App) -> Result<Vec<String>> {
    let Some(svc) = app.nostr.clone() else {
        bail!("nostr transport not configured — seed-only rescue needs at least one relay");
    };
    let me = blocking(app, |e| Ok(e.store.seed()?.identity_pubkey()?.to_string())).await?;
    Ok(svc.fetch_my_snapshots(&me).await)
}

// ---- JSON-RPC plumbing -------------------------------------------------

/// Positional (array) or named (object) params, bitcoin-cli style.
struct Params(Value);

impl Params {
    fn get(&self, i: usize, name: &str) -> Result<&Value> {
        match &self.0 {
            Value::Array(a) => a
                .get(i)
                .with_context(|| format!("missing param '{name}' (position {i})")),
            Value::Object(o) => o
                .get(name)
                .with_context(|| format!("missing param '{name}'")),
            Value::Null => bail!("missing param '{name}'"),
            _ => bail!("params must be an array or object"),
        }
    }
    fn str(&self, i: usize, name: &str) -> Result<String> {
        // Accept a JSON string, or render a scalar back to one — the CLI
        // JSON-parses each arg, so `sendtoaddress btc <addr> 0.5` arrives as a
        // number and must not be rejected for it (bitcoin-cli coerces too).
        match self.get(i, name)? {
            Value::String(s) => Ok(s.clone()),
            Value::Number(n) => Ok(n.to_string()),
            Value::Bool(b) => Ok(b.to_string()),
            _ => bail!("param '{name}' must be a string"),
        }
    }
    fn u32(&self, i: usize, name: &str) -> Result<u32> {
        let v = self.get(i, name)?;
        // Accept number or numeric string (bitcoin-cli sends strings).
        v.as_u64()
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            .with_context(|| format!("param '{name}' must be a u32"))
            .map(|n: u64| n as u32)
    }
    fn u64(&self, i: usize, name: &str) -> Result<u64> {
        let v = self.get(i, name)?;
        // Accept number or numeric string (bitcoin-cli sends strings).
        v.as_u64()
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            .with_context(|| format!("param '{name}' must be a u64"))
    }
    fn opt_u64(&self, i: usize, name: &str) -> Option<u64> {
        let v = self.get(i, name).ok()?;
        v.as_u64()
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    }
    fn opt_f64(&self, i: usize, name: &str) -> Option<f64> {
        let v = self.get(i, name).ok()?;
        v.as_f64()
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    }
    fn opt_str(&self, i: usize, name: &str) -> Option<String> {
        self.get(i, name)
            .ok()
            .and_then(|v| v.as_str())
            .map(str::to_string)
    }
    fn opt_bool(&self, i: usize, name: &str) -> Option<bool> {
        let v = self.get(i, name).ok()?;
        v.as_bool().or_else(|| match v.as_str()? {
            "true" | "1" => Some(true),
            "false" | "0" => Some(false),
            _ => None,
        })
    }
    fn bool(&self, i: usize, name: &str) -> Result<bool> {
        let v = self.get(i, name)?;
        // Accept a JSON bool or a "true"/"false"/"1"/"0" string (CLI sends strings).
        v.as_bool()
            .or_else(|| match v.as_str()? {
                "true" | "1" => Some(true),
                "false" | "0" => Some(false),
                _ => None,
            })
            .with_context(|| format!("param '{name}' must be a bool"))
    }
    fn envelope(&self, i: usize, name: &str) -> Result<Envelope> {
        serde_json::from_value(self.get(i, name)?.clone())
            .with_context(|| format!("param '{name}' is not an envelope"))
    }
}

/// Flattened JSON view of a fee-bump policy for `get/setfeepolicy` — one level so
/// the CLI/UI sets each field by a simple typed name (no nested objects). The
/// `step_pct` escalation knob was retired when the bump sites moved to the
/// market-tracking `target_feerate` strategy (fee-bump-design.md); it is no
/// longer exposed.
fn fee_policy_json(p: &libswap::FeeBumpPolicy) -> Value {
    json!({
        "max_feerate_sat_vb": p.max_feerate_sat_vb,
        "reservation_mult": p.funding.reservation_mult,
    })
}

/// Validate a coin id against the shipped registry, returning its canonical
/// (lowercase) id.
fn parse_coin(name: &str) -> Result<String> {
    let id = name.to_ascii_lowercase();
    if libswap::registry::get(&id).is_none() {
        bail!("unknown coin {name:?} (not in the shipped registry)");
    }
    Ok(id)
}

/// Parse a `--coin coin_id=url[,url...]` launch argument into
/// `(coin_id, backends)`. The coin must be shipped; the URL list non-empty.
fn parse_coin_arg(spec: &str) -> Result<(String, String)> {
    let (id, urls) = spec
        .split_once('=')
        .with_context(|| format!("--coin expects coin_id=urls, got {spec:?}"))?;
    let id = parse_coin(id)?;
    let urls = urls.trim();
    ensure!(!urls.is_empty(), "--coin {id}: empty backend URL list");
    Ok((id, urls.to_string()))
}

/// Assemble the per-coin backend map from the `--coin` flags — the single,
/// generic way every coin is attached (no per-coin special cases).
fn coins_from_args(args: &Args) -> Result<BTreeMap<String, String>> {
    let mut coins = BTreeMap::new();
    for spec in &args.coins {
        let (id, urls) = parse_coin_arg(spec)?;
        coins.insert(id, urls);
    }
    Ok(coins)
}

/// Parse a `--coin-confs coin_id=N` launch argument into `(coin_id, depth)`.
/// The coin must be shipped; the depth a positive integer.
fn parse_coin_confs_arg(spec: &str) -> Result<(String, u32)> {
    let (id, n) = spec
        .split_once('=')
        .with_context(|| format!("--coin-confs expects coin_id=N, got {spec:?}"))?;
    let id = parse_coin(id)?;
    let n: u32 = n
        .trim()
        .parse()
        .with_context(|| format!("--coin-confs {id}: N must be an integer"))?;
    ensure!(n >= 1, "--coin-confs {id}: depth must be ≥ 1");
    Ok((id, n))
}

/// Assemble the per-coin confirmation-depth map from `--coin-confs` flags.
fn coin_confs_from_args(args: &Args) -> Result<BTreeMap<String, u32>> {
    let mut map = BTreeMap::new();
    for spec in &args.coin_confs {
        let (id, n) = parse_coin_confs_arg(spec)?;
        map.insert(id, n);
    }
    Ok(map)
}

/// The public RPC catalog: (category, name, args, summary) per method, in the
/// order `help` prints them. Drives `help`, `listmethods` and the
/// unknown-method suggestion. KEEP IN SYNC with the `dispatch` match below —
/// the `catalog_matches_dispatch` test calls every cataloged name and fails on
/// one that falls through to "unknown method". The regtest-only
/// `_settestfeerate` hook is deliberately unlisted.
const METHODS: &[(&str, &str, &str, &str)] = &[
    (
        "control",
        "getinfo",
        "",
        "Daemon summary: version, protocol, network, identity, seed state, configured coins.",
    ),
    (
        "control",
        "help",
        "[method]",
        "List all methods by category, or show what one method does.",
    ),
    (
        "control",
        "listmethods",
        "",
        "Machine-readable array of all method names.",
    ),
    ("control", "stop", "", "Shut pactd down cleanly."),
    (
        "control",
        "tick",
        "",
        "One coordination + scheduler pass (relay sync, then auto-redeem/refund/fee-bump).",
    ),
    (
        "coins",
        "listcoins",
        "",
        "Shipped coins: which are configured, live connection status, tip height, confirmations.",
    ),
    (
        "coins",
        "serverstatus",
        "<coin_id>",
        "Passive health of the coin's Electrum servers (state, latency, last error) — display data, never probes.",
    ),
    (
        "coins",
        "listpairs",
        "",
        "Derived swap-pair availability for the current coin setup.",
    ),
    (
        "coins",
        "validatecoin",
        "<coin_id> <chain_data>",
        "Genesis-validate a proposed backend URL list for a coin without saving it.",
    ),
    (
        "coins",
        "estimateswapfees",
        "<give_coin> <get_coin>",
        "Fee preview for a prospective swap on the given pair.",
    ),
    (
        "wallet",
        "walletstatus",
        "",
        "Seed lifecycle: whether a seed exists / is encrypted / is locked.",
    ),
    (
        "wallet",
        "createseed",
        "[passphrase] [words]",
        "Create + persist a seed (12 or 24 words); the mnemonic is returned exactly once.",
    ),
    (
        "wallet",
        "generateseed",
        "[words]",
        "Generate a fresh mnemonic WITHOUT persisting it (onboarding show-then-confirm).",
    ),
    (
        "wallet",
        "importseed",
        "<mnemonic> [passphrase]",
        "Import an existing BIP39 mnemonic (optionally encrypted at rest).",
    ),
    (
        "wallet",
        "unlock",
        "<passphrase>",
        "Unlock an encrypted seed for this session (held in memory only).",
    ),
    (
        "wallet",
        "getbalance",
        "<coin>",
        "Wallet balance (sat) for a configured coin.",
    ),
    (
        "wallet",
        "getnewaddress",
        "<coin>",
        "Fresh receive address for a configured coin.",
    ),
    (
        "wallet",
        "sendtoaddress",
        "<coin> <address> <amount> [conf_target] [fee_rate]",
        "Send from the coin's wallet; amount in coin units (e.g. 0.5), or 'all' to sweep (fee off the amount).",
    ),
    (
        "wallet",
        "estimatesendfee",
        "<coin>",
        "Fee preview for the send form: slow/normal/fast sat/vB (null = no estimate) + the coin's floor.",
    ),
    (
        "wallet",
        "bumpfee",
        "<coin> <txid> <fee_rate>",
        "RBF-bump an unconfirmed wallet send to fee_rate sat/vB (nodeless coins; live-swap fundings refused).",
    ),
    (
        "wallet",
        "listtransactions",
        "<coin>",
        "Wallet activity of a nodeless coin, newest first.",
    ),
    (
        "wallet",
        "getfeepolicy",
        "",
        "The active merchant's fee-bump policy.",
    ),
    (
        "wallet",
        "setfeepolicy",
        "[max_feerate_sat_vb] [reservation_mult]",
        "Update fee-bump policy fields; only the fields supplied change.",
    ),
    (
        "wallet",
        "setwatchonly",
        "<on>",
        "Enter/leave watch-only mode (browse + withdraw own offers, never post/take/fund).",
    ),
    (
        "merchants",
        "createmerchant",
        "[label]",
        "Create a new merchant (its own seed + state under merchants/<id>/).",
    ),
    ("merchants", "listmerchants", "", "List all merchants."),
    (
        "merchants",
        "loadmerchant",
        "<id>",
        "Load a merchant as the active one.",
    ),
    (
        "merchants",
        "renamemerchant",
        "<id> <label>",
        "Relabel a merchant.",
    ),
    ("merchants", "unloadmerchant", "", "Unload the active merchant."),
    (
        "merchants",
        "getmerchantinfo",
        "[id]",
        "Details for one merchant (default: the active one).",
    ),
    ("swaps", "listswaps", "", "All v1 swap records."),
    (
        "swaps",
        "listadaptorswaps",
        "",
        "All v2 (adaptor) swap records.",
    ),
    (
        "swaps",
        "swapprogress",
        "",
        "Live per-swap progress: confirmation depth + latest scheduler action.",
    ),
    (
        "swaps",
        "listpendingtakes",
        "",
        "Outstanding takes still awaiting the maker's init.",
    ),
    (
        "swaps",
        "listmyoffers",
        "",
        "Our own board offers with lifecycle state and expiries.",
    ),
    ("swaps", "getswap", "<swap_id>", "One v1 swap record."),
    (
        "swaps",
        "dumpswap",
        "<swap_id>",
        "Dev-shareable dump of one swap: secrets-scrubbed record + its pactd log lines.",
    ),
    (
        "swaps",
        "offer",
        "<give> <get> <t1> <t2>",
        "Start a v1 swap as initiator (amounts as coin:value); returns the init envelope.",
    ),
    (
        "swaps",
        "acceptoffer",
        "<envelope>",
        "Accept a v1 init envelope; returns the accept envelope.",
    ),
    (
        "swaps",
        "recv",
        "<envelope>",
        "Ingest a counterparty v1 message (accept/funded/redeemed/abort).",
    ),
    (
        "swaps",
        "fund",
        "<swap_id>",
        "Fund our HTLC leg and notify the counterparty.",
    ),
    (
        "swaps",
        "redeem",
        "<swap_id>",
        "Redeem the counterparty HTLC (initiator: reveals the preimage).",
    ),
    (
        "swaps",
        "refund",
        "<swap_id>",
        "Broadcast the refund for our HTLC (valid once MTP >= T).",
    ),
    (
        "swaps",
        "abort",
        "<swap_id> [reason]",
        "Cancel a v1/v2 swap before funding, or drop a pending take (id = offer id).",
    ),
    (
        "adaptor (v2)",
        "adaptorinit",
        "<give> <get> <t1> <t2>",
        "Start a v2 adaptor swap as initiator; returns the init envelope.",
    ),
    (
        "adaptor (v2)",
        "adaptoraccept",
        "<envelope>",
        "Accept a v2 init envelope.",
    ),
    (
        "adaptor (v2)",
        "adaptorrecv",
        "<envelope>",
        "Ingest a counterparty v2 message.",
    ),
    (
        "adaptor (v2)",
        "adaptorfundingready",
        "<swap_id> <txid> <vout>",
        "Announce our funding outpoint for the v2 handshake.",
    ),
    (
        "adaptor (v2)",
        "adaptornonces",
        "<swap_id>",
        "Produce our musig2 nonces envelope.",
    ),
    (
        "adaptor (v2)",
        "adaptorsign",
        "<swap_id>",
        "Produce our partial signatures envelope.",
    ),
    (
        "adaptor (v2)",
        "adaptorassemble",
        "<swap_id>",
        "Assemble the counterparty's partials into complete redeem/refund txs.",
    ),
    (
        "adaptor (v2)",
        "adaptorfund",
        "<swap_id>",
        "Broadcast our v2 funding transaction.",
    ),
    (
        "adaptor (v2)",
        "adaptorredeem",
        "<swap_id>",
        "Broadcast the v2 redeem for the counterparty leg.",
    ),
    (
        "adaptor (v2)",
        "adaptorrefund",
        "<swap_id>",
        "Broadcast the v2 refund for our leg.",
    ),
    (
        "board",
        "boardlistoffers",
        "[board]",
        "Offers on a configured board (default: the first).",
    ),
    (
        "board",
        "boardpostoffer",
        "<give> <get> <t1_secs> <t2_secs> [protocol] [ttl_secs]",
        "Post an offer to every configured board.",
    ),
    ("board", "boardtake", "<offer_id>", "Take a board offer."),
    (
        "board",
        "boardrevoke",
        "<offer_id>",
        "Withdraw one of our board offers (terminal).",
    ),
    (
        "board",
        "revokeoffersforcoin",
        "<coin_id>",
        "Withdraw every live offer whose pair involves a coin (used when removing it).",
    ),
    (
        "board",
        "boardstatus",
        "",
        "Relay connectivity: one entry per configured Nostr relay.",
    ),
    (
        "board",
        "makeprivateoffer",
        "<give> <get> <t1_secs> <t2_secs> [protocol] [ttl_secs]",
        "Build + sign an off-market offer slip (never posted to a board).",
    ),
    (
        "board",
        "takeoffer",
        "<slip>",
        "Take a private offer from a pasted slip.",
    ),
    ("board", "listprivateoffers", "", "Our own private offer slips."),
    (
        "board",
        "cancelprivateoffer",
        "<offer_id>",
        "Cancel one of our private offer slips.",
    ),
    (
        "rescue",
        "rescuestatus",
        "",
        "Read-only: how many in-flight swaps `restorefromrelay` would recover.",
    ),
    (
        "rescue",
        "restorefromrelay",
        "<no args — read the warning first>",
        "Adopt in-flight swaps from our encrypted relay snapshots (seed-only rescue). ONLY once the machine that ran them is retired — two live drivers can double-fund a swap.",
    ),
];

/// Typed marker for JSON-RPC -32601 (method not found) — the one place the
/// transport layer distinguishes an error kind (issue #57 P1; broader typed
/// codes stay open as P2).
#[derive(Debug)]
struct MethodNotFound(String);

impl std::fmt::Display for MethodNotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for MethodNotFound {}

/// Build the "unknown method" error, suggesting the nearest cataloged name
/// when it is plausibly a typo (edit distance scaled to the input length).
fn method_not_found(method: &str) -> anyhow::Error {
    let nearest = METHODS
        .iter()
        .map(|(_, n, _, _)| *n)
        .min_by_key(|n| edit_distance(method, n));
    let hint = match nearest {
        Some(n) if edit_distance(method, n) <= 2.max(method.len() / 3) => {
            format!(" — did you mean '{n}'?")
        }
        _ => String::new(),
    };
    anyhow::Error::new(MethodNotFound(format!(
        "unknown method '{method}'{hint} (see 'help')"
    )))
}

/// Plain Levenshtein distance (two-row DP) for the did-you-mean suggestion.
fn edit_distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut row = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let sub = prev[j] + usize::from(ca != cb);
            row.push(sub.min(prev[j + 1] + 1).min(row[j] + 1));
        }
        prev = row;
    }
    prev[b.len()]
}

/// Render `help` / `help <method>` as plain text, bitcoin-cli style (the CLI
/// prints string results raw, so this reads like a man page, not JSON).
fn render_help(topic: Option<&str>) -> Result<String> {
    let Some(topic) = topic else {
        let mut out = String::new();
        let mut last_cat = "";
        for (cat, name, args, _) in METHODS {
            if *cat != last_cat {
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(&format!("== {cat} ==\n"));
                last_cat = cat;
            }
            out.push_str(name);
            if !args.is_empty() {
                out.push(' ');
                out.push_str(args);
            }
            out.push('\n');
        }
        out.push_str("\nhelp <method> explains one method; any method is callable as `pact-cli <method> [params...]`.");
        return Ok(out);
    };
    let topic = topic.to_ascii_lowercase();
    let (_, name, args, summary) = METHODS
        .iter()
        .find(|(_, n, _, _)| *n == topic)
        .ok_or_else(|| method_not_found(&topic))?;
    Ok(if args.is_empty() {
        format!("{name}\n\n{summary}")
    } else {
        format!("{name} {args}\n\n{summary}")
    })
}

/// Dispatch one JSON-RPC method. Result payloads match the old REST bodies
/// so clients only change transport, not shapes.
async fn dispatch(app: &App, method: &str, params: Value) -> Result<Value> {
    let p = Params(params);
    let net = app.network;
    match method {
        "help" => Ok(Value::String(render_help(
            p.opt_str(0, "method").as_deref(),
        )?)),
        "listmethods" => Ok(json!(METHODS
            .iter()
            .map(|(_, n, _, _)| *n)
            .collect::<Vec<_>>())),
        "getinfo" => {
            // Tolerate a missing/locked seed: a fresh merchant (first run) or
            // a locked encrypted one has no identity to show yet. The UI uses
            // seed_exists/locked to drive the wizard / unlock prompt.
            let (status, identity, coins, watch_only) = blocking(app, |e| {
                let status = e.store.wallet_status()?;
                let identity = if status.seed_exists && !status.locked {
                    e.store
                        .seed()
                        .ok()
                        .and_then(|s| s.identity_pubkey().ok())
                        .map(|p| p.to_string())
                } else {
                    None
                };
                Ok((status, identity, e.configured_coins(), e.watch_only))
            })
            .await?;
            Ok(json!({
                "name": "pactd",
                "version": env!("CARGO_PKG_VERSION"),
                "protocol": libswap::PROTOCOL_VERSION,
                // Wire-compatibility epochs this build speaks, per protocol
                // family (rc10). The UI badges offers whose signed `wire`
                // differs as un-takeable; the engine refuses them anyway.
                "wire_epochs": {
                    libswap::PROTOCOL_VERSION: libswap::WIRE_V1,
                    libswap::adaptor_swap::PROTOCOL_V2: libswap::WIRE_V2,
                },
                "network": format!("{net:?}").to_lowercase(),
                "identity": identity,
                "seed_exists": status.seed_exists,
                "encrypted": status.encrypted,
                "locked": status.locked,
                "coins": coins,
                "watch_only": watch_only,
            }))
        }
        "walletstatus" => {
            let status = blocking(app, |e| e.store.wallet_status()).await?;
            Ok(serde_json::to_value(status)?)
        }
        // The active merchant's fee-bump policy (per-merchant; pactd's store owns
        // it). Callable from the CLI like any other method — typed params, no JSON
        // blob.
        "getfeepolicy" => {
            let policy = blocking(app, |e| Ok(e.fee_bump)).await?;
            Ok(fee_policy_json(&policy))
        }
        // Update the active merchant's policy. Each field is optional; only the
        // fields supplied change. Validated server-side; persisted; applied live.
        "setfeepolicy" => {
            let max = p.opt_u64(0, "max_feerate_sat_vb");
            let reservation = p.opt_u64(1, "reservation_mult");
            let policy = blocking_mut(app, move |e| {
                let mut pol = e.fee_bump;
                if let Some(v) = max {
                    pol.max_feerate_sat_vb = v;
                }
                if let Some(v) = reservation {
                    pol.funding.reservation_mult = v;
                }
                e.set_fee_bump(pol)?;
                Ok(e.fee_bump)
            })
            .await?;
            Ok(fee_policy_json(&policy))
        }
        // Enter/leave watch-only mode for the active merchant (per-merchant, in
        // pactd's store; persisted, applied live). A watch-only session browses
        // the board and may withdraw its own offers, but never posts/takes/funds
        // and never manages offer liveness for another session. `getinfo` reports
        // the current value.
        "setwatchonly" => {
            let on = p.bool(0, "on")?;
            blocking_mut(app, move |e| e.set_watch_only(on)).await?;
            Ok(json!({ "watch_only": on }))
        }
        "listcoins" => {
            // Shipped registry + which are configured + a live connection
            // probe per configured coin (genesis check + tip). Unconfigured
            // coins are reported too, so the setup UI can offer them.
            let configured = blocking(app, |e| Ok(e.configured_coins())).await?;
            let mut coins = Vec::new();
            for def in libswap::registry::all().iter().copied() {
                // Skip coins not defined on the active network (a file coin may
                // ship e.g. regtest only).
                let Some(params) = def.params(net) else {
                    continue;
                };
                let is_conf = configured.iter().any(|c| c == def.id);
                let (status, tip) = if is_conf {
                    let id = def.id.to_string();
                    match blocking(app, move |e| e.probe_coin(net, &id)).await {
                        Ok(height) => ("ok".to_string(), Some(height)),
                        Err(err) => (format!("error: {err:#}"), None),
                    }
                } else {
                    ("unconfigured".to_string(), None)
                };
                // Effective + default confirmation depth (reorg-safety), so the
                // setup UI can show the value in force and its default.
                let coin_id = def.id.to_string();
                let (confirmations, default_confirmations) =
                    blocking(app, move |e| e.coin_confirmations_view(net, &coin_id))
                        .await
                        .unwrap_or((0, 0));
                // The Core wallet this coin's RPC is scoped to (parsed from the
                // configured URL); null when none is set (node default wallet).
                let wid = def.id.to_string();
                let wallet = blocking(app, move |e| Ok(e.coin_wallet(&wid)))
                    .await
                    .unwrap_or(None);
                // Nodeless (Electrum-only, pact-seed bdk wallet) — the UI keys
                // the send/receive/activity surface off this (epic #58).
                let nid = def.id.to_string();
                let nodeless = blocking(app, move |e| Ok(e.coin_nodeless(&nid)))
                    .await
                    .unwrap_or(false);
                // Two-dimensional health (issue #98): the quorum-based
                // `status` above stays green while a MINORITY of servers is
                // down — these fields let the UI show that degradation, and
                // a dead wallet-home server, instead of a false green.
                // Cheap in-memory registry read; never dials.
                let hid = def.id.to_string();
                let servers = if is_conf {
                    blocking(app, move |e| e.server_status(&hid))
                        .await
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };
                let servers_healthy = servers.iter().filter(|s| s.state == "healthy").count();
                let servers_down = servers.iter().filter(|s| s.state == "down").count();
                // The wallet HOME is the ELECTED server (#99), not simply
                // the first URL; before the first election (fresh boot) the
                // first URL is the presumptive home.
                let wallet_server_state = if nodeless {
                    servers
                        .iter()
                        .find(|s| s.role.as_deref() == Some("wallet"))
                        .or_else(|| servers.first())
                        .map(|s| s.state.clone())
                } else {
                    None
                };
                coins.push(json!({
                    "id": def.id,
                    "display_name": def.display_name,
                    "symbol": def.symbol,
                    "decimals": def.decimals,
                    "capabilities": def.capabilities,
                    "configured": is_conf,
                    "status": status,
                    "tip_height": tip,
                    "genesis_hash": params.genesis_hash,
                    "bech32_hrp": params.bech32_hrp,
                    "confirmations": confirmations,
                    "default_confirmations": default_confirmations,
                    "wallet": wallet,
                    "nodeless": nodeless,
                    "servers_total": servers.len(),
                    "servers_healthy": servers_healthy,
                    "servers_down": servers_down,
                    "wallet_server_state": wallet_server_state,
                    // Cache freshness (#99): how long ago the nodeless
                    // wallet cache was last confirmed against its server —
                    // the "balance as of" hint while the home is down.
                    "wallet_synced_secs_ago": if nodeless {
                        let sid = def.id.to_string();
                        blocking(app, move |e| Ok(e.wallet_sync_age_secs(&sid)))
                            .await
                            .unwrap_or(None)
                    } else {
                        None
                    },
                }));
            }
            Ok(json!({ "network": format!("{net:?}").to_lowercase(), "coins": coins }))
        }
        "serverstatus" => {
            // Passive Electrum-server health for one coin (issue #98): a
            // pure in-memory read of the health cells real traffic feeds —
            // this must NEVER dial or probe (the Network page polls it).
            let coin = parse_coin(&p.str(0, "coin_id")?)?;
            let servers = blocking(app, move |e| e.server_status(&coin)).await?;
            Ok(json!({ "servers": servers }))
        }
        "listpairs" => {
            // Derived availability for the current setup (no curated list).
            let configured = blocking(app, |e| Ok(e.configured_coins())).await?;
            let refs: Vec<&str> = configured.iter().map(String::as_str).collect();
            let pairs = libswap::registry::derive_pairs(&refs);
            Ok(json!({
                "network": format!("{net:?}").to_lowercase(),
                "pairs": serde_json::to_value(&pairs)?,
            }))
        }
        "validatecoin" => {
            // Genesis-hash check of a *proposed* backend before Satchel saves
            // it (spec §3.3). Builds an ephemeral backend; engine config is
            // untouched.
            let coin = parse_coin(&p.str(0, "coin_id")?)?;
            let chain_data = p.str(1, "chain_data")?;
            let genesis = libswap::registry::lookup(&coin, net).map(|p| p.genesis_hash);
            let tip = blocking(app, move |e| e.validate_coin(net, &coin, &chain_data)).await?;
            Ok(json!({ "ok": true, "tip_height": tip, "genesis_hash": genesis }))
        }
        "createseed" => {
            let passphrase = p.opt_str(0, "passphrase").filter(|s| !s.is_empty());
            let encrypted = passphrase.is_some();
            // Optional word count (12 default | 24) — phoenix parity.
            let words = p.opt_u64(1, "words").unwrap_or(12) as usize;
            let mnemonic = blocking_mut(app, move |e| {
                e.store.create_seed(passphrase.as_deref(), words)
            })
            .await?;
            kick_nostr(app);
            // The mnemonic is returned exactly once, for the user to back up.
            Ok(json!({ "mnemonic": mnemonic, "encrypted": encrypted }))
        }
        // Generate a fresh mnemonic WITHOUT persisting it — the onboarding flow
        // shows + confirms it, then commits via `importseed`. Read-only.
        "generateseed" => {
            // Optional word count (12 default | 24) — phoenix parity.
            let words = p.opt_u64(0, "words").unwrap_or(12) as usize;
            let mnemonic = blocking(app, move |e| e.store.generate_mnemonic(words)).await?;
            Ok(json!({ "mnemonic": mnemonic }))
        }
        "importseed" => {
            let mnemonic = p.str(0, "mnemonic")?;
            let passphrase = p.opt_str(1, "passphrase").filter(|s| !s.is_empty());
            let encrypted = passphrase.is_some();
            let phrase = blocking_mut(app, move |e| {
                e.store.import_seed(&mnemonic, passphrase.as_deref())
            })
            .await?;
            let identity =
                blocking(app, |e| Ok(e.store.seed()?.identity_pubkey()?.to_string())).await?;
            kick_nostr(app);
            // Echo the normalized phrase so the client can confirm, plus the
            // derived identity so the UI can show the new merchant at once.
            Ok(json!({ "mnemonic": phrase, "encrypted": encrypted, "identity": identity }))
        }
        "unlock" => {
            let passphrase = p.str(0, "passphrase")?;
            blocking_mut(app, move |e| e.store.unlock(&passphrase)).await?;
            let identity =
                blocking(app, |e| Ok(e.store.seed()?.identity_pubkey()?.to_string())).await?;
            kick_nostr(app);
            Ok(json!({ "unlocked": true, "identity": identity }))
        }
        // ---- merchants owned by pactd (C10) — the Bitcoin-Core wallet analog.
        // The RPC surface is merchant-scoped-ready (load/unload/info name an
        // explicit merchant); Phase 1 loads one active merchant at a time.
        "createmerchant" => {
            let label = p.str(0, "label").unwrap_or_default();
            let meta = blocking_registry(app, move |r| r.create(&label)).await?;
            Ok(json!({ "id": meta.id, "label": meta.label }))
        }
        "listmerchants" => Ok(blocking_registry(app, |r| Ok(r.list())).await?),
        "loadmerchant" => {
            let id = p.str(0, "id")?;
            let meta = blocking_registry(app, move |r| r.load(&id)).await?;
            kick_nostr(app);
            Ok(json!({ "id": meta.id, "label": meta.label }))
        }
        "renamemerchant" => {
            let id = p.str(0, "id")?;
            let label = p.str(1, "label")?;
            let meta = blocking_registry(app, move |r| r.set_label(&id, &label)).await?;
            Ok(json!({ "id": meta.id, "label": meta.label }))
        }
        "unloadmerchant" => {
            blocking_registry(app, |r| r.unload()).await?;
            Ok(json!({ "unloaded": true }))
        }
        "getmerchantinfo" => {
            let id = p.opt_str(0, "id");
            Ok(blocking_registry(app, move |r| r.info(id.as_deref())).await?)
        }
        "stop" => {
            // `stop skip_delist=true` (Satchel's config-change relaunch) suppresses
            // the shutdown soft de-list so surviving offers ride the relaunch (#97).
            let skip_delist = p.opt_bool(0, "skip_delist").unwrap_or(false);
            app.skip_delist_on_stop
                .store(skip_delist, std::sync::atomic::Ordering::SeqCst);
            app.shutdown.notify_one();
            Ok(json!("pactd stopping"))
        }
        "listswaps" => Ok(serde_json::to_value(
            blocking(app, |e| e.store.list()).await?,
        )?),
        // v2 (pact-htlc-v2) adaptor swaps live in their own table.
        "listadaptorswaps" => Ok(serde_json::to_value(
            blocking(app, |e| e.store.list_adaptor()).await?,
        )?),
        // Live per-swap progress (observability): confirmation depth + the latest
        // scheduler action, refreshed each tick and served from memory (no node
        // call per poll). The UI merges it into its swap rows by `swap_id`.
        "swapprogress" => Ok(serde_json::to_value(
            blocking(app, |e| Ok(e.swap_progress_snapshot())).await?,
        )?),
        // Outstanding takes awaiting the maker's init (the UI's "initiating"
        // pre-swaps — no swap record exists yet).
        "listpendingtakes" => Ok(serde_json::to_value(
            blocking(app, |e| e.list_pending_takes()).await?,
        )?),
        // The maker's OWN offers (offer-lifecycle): the My-offers view. Each row
        // carries the signed envelope (so the UI renders it like any offer card),
        // its lifecycle state, the rolling CURRENT expiry (last_refresh + relay
        // TTL, capped at final) and the maker-set FINAL expiry (created+valid_for).
        "listmyoffers" => {
            let rows = blocking(app, |e| e.store.my_offers_all()).await?;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let out: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|o| {
                    let env: serde_json::Value =
                        serde_json::from_str(&o.envelope).unwrap_or_else(|_| json!({}));
                    let final_expiry = if o.created != 0 && o.valid_for != 0 {
                        o.created + o.valid_for
                    } else {
                        0
                    };
                    let current_expiry = if o.last_refresh != 0 {
                        let roll = o.last_refresh + pact_nostr::RELAY_TTL_SECS;
                        if final_expiry != 0 {
                            roll.min(final_expiry)
                        } else {
                            roll
                        }
                    } else {
                        0
                    };
                    json!({
                        "offer_id": o.offer_id,
                        "offer": env,
                        "state": o.state,
                        "created": o.created,
                        "valid_for": o.valid_for,
                        "current_expiry": current_expiry,
                        "final_expiry": final_expiry,
                        "now": now,
                    })
                })
                .collect();
            Ok(json!(out))
        }
        "getswap" => {
            let id = p.str(0, "swap_id")?;
            Ok(serde_json::to_value(
                blocking(app, move |e| e.store.get(&id)).await?,
            )?)
        }
        // RC2 (#3b): a dev-shareable dump for ONE swap — its current record with
        // secrets scrubbed + the pactd log lines mentioning it. Safe to hand to
        // a dev (no seed/preimage/nonces). The UI's per-swap "Dump logs" button.
        "dumpswap" => {
            let id = p.str(0, "swap_id")?;
            let dump = blocking_registry(app, move |r| {
                let log_dir = r.data_dir().join("logs");
                let engine = r.active()?;
                // v1 record first (scrub its preimage); else the v2 adaptor
                // record (stores no secret — `t` is never persisted); else an
                // "initiating" pre-swap — dump the pending take (the signed
                // offer we took carries no secret either).
                let record = match engine.store.get(&id) {
                    Ok(rec) => scrub_secrets(serde_json::to_value(&rec)?),
                    Err(_) => match engine.store.get_adaptor(&id) {
                        Ok(rec) => serde_json::to_value(&rec)?,
                        Err(_) => {
                            let take = engine
                                .list_pending_takes()?
                                .into_iter()
                                .find(|t| t.offer_id == id)
                                .with_context(|| format!("unknown swap {id}"))?;
                            serde_json::to_value(&take)?
                        }
                    },
                };
                Ok(json!({
                    "swap_id": id,
                    "pactd_version": env!("CARGO_PKG_VERSION"),
                    "record": record,
                    "log": swap_log_lines(&log_dir, &id),
                }))
            })
            .await?;
            Ok(dump)
        }
        "offer" => {
            let give = parse_coin_amount(&p.str(0, "give")?)?;
            let get = parse_coin_amount(&p.str(1, "get")?)?;
            let (t1, t2) = (p.u32(2, "t1")?, p.u32(3, "t2")?);
            let (record, envelope) =
                blocking(app, move |e| e.offer(net, give, get, t1, t2, None, None)).await?;
            Ok(
                json!({ "record": serde_json::to_value(&record)?, "envelope": serde_json::to_value(&envelope)? }),
            )
        }
        "acceptoffer" => {
            let env = p.envelope(0, "envelope")?;
            let (record, reply) = blocking(app, move |e| e.accept(&env)).await?;
            Ok(
                json!({ "record": serde_json::to_value(&record)?, "envelope": serde_json::to_value(&reply)? }),
            )
        }
        "recv" => {
            let env = p.envelope(0, "envelope")?;
            let record = blocking(app, move |e| e.recv(&env)).await?;
            Ok(json!({ "record": serde_json::to_value(&record)? }))
        }
        // v2 (pact-htlc-v2) adaptor-swap lifecycle (spec v2 §7).
        "adaptorinit" => {
            let give = parse_coin_amount(&p.str(0, "give")?)?;
            let get = parse_coin_amount(&p.str(1, "get")?)?;
            let (t1, t2) = (p.u32(2, "t1")?, p.u32(3, "t2")?);
            let (record, envelope) =
                blocking(app, move |e| e.adaptor_init(net, give, get, t1, t2)).await?;
            Ok(
                json!({ "record": serde_json::to_value(&record)?, "envelope": serde_json::to_value(&envelope)? }),
            )
        }
        "adaptoraccept" => {
            let env = p.envelope(0, "envelope")?;
            let (record, reply) = blocking(app, move |e| e.adaptor_accept(&env)).await?;
            Ok(
                json!({ "record": serde_json::to_value(&record)?, "envelope": serde_json::to_value(&reply)? }),
            )
        }
        "adaptorrecv" => {
            let env = p.envelope(0, "envelope")?;
            let record = blocking(app, move |e| e.recv_adaptor(&env)).await?;
            Ok(json!({ "record": serde_json::to_value(&record)? }))
        }
        "adaptorfundingready" => {
            let (id, txid, vout) = (p.str(0, "swap_id")?, p.str(1, "txid")?, p.u32(2, "vout")?);
            let envelope =
                blocking(app, move |e| e.adaptor_funding_ready(&id, &txid, vout)).await?;
            Ok(json!({ "envelope": serde_json::to_value(&envelope)? }))
        }
        "adaptornonces" => {
            let id = p.str(0, "swap_id")?;
            let envelope = blocking(app, move |e| e.adaptor_nonces(&id)).await?;
            Ok(json!({ "envelope": serde_json::to_value(&envelope)? }))
        }
        "adaptorsign" => {
            let id = p.str(0, "swap_id")?;
            let envelope = blocking(app, move |e| e.adaptor_sign(&id)).await?;
            Ok(json!({ "envelope": serde_json::to_value(&envelope)? }))
        }
        "adaptorassemble" => {
            let id = p.str(0, "swap_id")?;
            let record = blocking(app, move |e| e.adaptor_assemble(&id)).await?;
            Ok(json!({ "record": serde_json::to_value(&record)? }))
        }
        "adaptorfund" => {
            let id = p.str(0, "swap_id")?;
            let envelope = blocking(app, move |e| e.adaptor_fund(&id)).await?;
            Ok(json!({ "envelope": serde_json::to_value(&envelope)? }))
        }
        "adaptorredeem" => {
            let id = p.str(0, "swap_id")?;
            let record = blocking(app, move |e| e.adaptor_redeem(&id)).await?;
            Ok(json!({ "record": serde_json::to_value(&record)? }))
        }
        "adaptorrefund" => {
            let id = p.str(0, "swap_id")?;
            let record = blocking(app, move |e| e.adaptor_refund(&id)).await?;
            Ok(json!({ "record": serde_json::to_value(&record)? }))
        }
        "fund" => {
            let id = p.str(0, "swap_id")?;
            // #5: relay the `funded` envelope (fund_and_notify), so a manual /
            // recovery fund notifies the maker like the auto-fund path does.
            let (record, envelope) = blocking(app, move |e| e.fund_and_notify(&id)).await?;
            Ok(
                json!({ "record": serde_json::to_value(&record)?, "envelope": serde_json::to_value(&envelope)? }),
            )
        }
        "redeem" => {
            let id = p.str(0, "swap_id")?;
            let record = blocking(app, move |e| e.redeem(&id)).await?;
            Ok(json!({ "record": serde_json::to_value(&record)? }))
        }
        "refund" => {
            let id = p.str(0, "swap_id")?;
            let record = blocking(app, move |e| e.refund(&id)).await?;
            Ok(json!({ "record": serde_json::to_value(&record)? }))
        }
        "abort" => {
            let id = p.str(0, "swap_id")?;
            let reason = p
                .opt_str(1, "reason")
                .unwrap_or_else(|| "user aborted".into());
            // One Cancel for every card kind the UI shows: a v1 record, a v2
            // adaptor record, or a still-unanswered pending take (there the
            // id is the offer id; cancel = drop it + tell the maker).
            let result = blocking(app, move |e| {
                if e.store.get(&id).is_ok() {
                    let record = e.abort(&id, &reason)?;
                    return Ok(json!({ "record": serde_json::to_value(&record)? }));
                }
                if e.store.get_adaptor(&id).is_ok() {
                    let record = e.adaptor_abort(&id, &reason)?;
                    return Ok(json!({ "record": serde_json::to_value(&record)? }));
                }
                e.cancel_pending_take(&id)?;
                Ok(json!({ "cancelled_pending_take": id }))
            })
            .await?;
            Ok(result)
        }
        "tick" => {
            // Mirror the scheduler: move Nostr mail/offers into the local
            // buffers (and publish our outbox) BEFORE the engine pass, so a
            // take/init that just arrived is dispatched by sync_board this tick.
            // Best-effort + a no-op when the Nostr transport isn't configured.
            kick_nostr(app);
            let events = blocking(app, |e| {
                let mut ev = e.sync_board();
                ev.extend(e.tick());
                Ok(ev)
            })
            .await?;
            Ok(json!({ "events": serde_json::to_value(&events)? }))
        }
        // Test-only (regtest): inject the market feerate (sat/vB) so the harness
        // can create a market-vs-broadcast gap for the fee-bump nurse — the
        // deterministic lever that replaces the now-removed settxfee. 0 clears it.
        // Honored by libswap::chain::fee_rate_sat_per_vb ONLY on regtest; the gate
        // here refuses it elsewhere so it can never perturb mainnet/testnet fees.
        "_settestfeerate" => {
            ensure!(
                net == Network::Regtest,
                "_settestfeerate is a regtest-only test hook"
            );
            let sat_vb = p.u32(0, "sat_vb")? as u64;
            libswap::chain::set_test_feerate(sat_vb);
            Ok(json!({ "test_feerate_sat_vb": sat_vb }))
        }
        "boardlistoffers" => {
            // Optional board selector: list from the given board (an HTTP
            // corkboard URL or "nostr"), else the first configured. Reads via the
            // engine's board set so it works under any transport (the old helper
            // was HTTP-only and errored with relays-only Nostr).
            let sel = p.opt_str(0, "board");
            let offers = blocking(app, move |e| e.list_board_offers(sel.as_deref())).await?;
            Ok(json!({ "offers": serde_json::to_value(&offers)? }))
        }
        // Nostr relay connectivity for the header indicator: one entry per
        // configured relay. Empty when the Nostr transport isn't configured.
        "boardstatus" => {
            let relays = match &app.nostr {
                Some(svc) => svc.relay_details().await,
                None => Vec::new(),
            };
            let relays: Vec<Value> = relays
                .into_iter()
                .map(|r| {
                    json!({
                        "url": r.url,
                        "connected": r.connected,
                        "status": r.status,
                        "latency_ms": r.latency_ms,
                        "connected_since": r.connected_since,
                        "attempts": r.attempts,
                        "success": r.success,
                        "bytes_sent": r.bytes_sent,
                        "bytes_received": r.bytes_received,
                    })
                })
                .collect();
            Ok(json!({ "relays": relays }))
        }
        // Seed-only rescue (#54): re-fetch our encrypted-to-self swap snapshots
        // from the relays and adopt any in-flight swap this machine is missing
        // (fresh install / wiped data dir, same seed). Idempotent — swaps we
        // already hold locally are left untouched. The scheduler then drives the
        // rescued swaps to completion/refund via chain-watch. This is the
        // EXPLICIT confirmation step: pactd itself only ever detects and warns
        // (see RESCUE_PENDING_WARNING) — call this only once the machine that
        // ran these swaps is retired.
        "restorefromrelay" => {
            let (restored, seen) = restore_from_relay(app).await?;
            Ok(json!({ "restored": restored, "seen": seen }))
        }
        // Read-only rescue detection (#54): how many relay snapshots WOULD be
        // adopted by `restorefromrelay`, without adopting any. A live relay
        // round each call — cheap enough for a user-invoked status check.
        "rescuestatus" => {
            let (pending, seen) = detect_rescue(app).await?;
            Ok(json!({
                "pending": pending,
                "seen": seen,
                "warning": if pending > 0 { Some(RESCUE_PENDING_WARNING) } else { None },
            }))
        }
        "boardpostoffer" => {
            let give = parse_coin_amount(&p.str(0, "give")?)?;
            let get = parse_coin_amount(&p.str(1, "get")?)?;
            let (t1s, t2s) = (p.u32(2, "t1_secs")?, p.u32(3, "t2_secs")?);
            // Optional protocol override (param 4): force "pact-htlc-v1"/"-v2";
            // omitted → default (Taproot pairs → v2 off-mainnet).
            let proto = p.opt_str(4, "protocol");
            // Optional offer TTL (param 5, seconds): the listing's validity →
            // NIP-40 expiry on Nostr; omitted → engine default. The Nostr
            // playground sets this short (~5 min) so stale test offers lapse.
            let ttl = p.opt_u64(5, "ttl_secs");
            let offer_id = blocking(app, move |e| {
                e.post_board_offer(net, give, get, t1s, t2s, ttl, proto.as_deref())
            })
            .await?;
            Ok(json!({ "offer_id": offer_id }))
        }
        "boardtake" => {
            let offer_id = p.str(0, "offer_id")?;
            blocking(app, move |e| e.take_board_offer(&offer_id)).await?;
            Ok(json!({ "taken": true }))
        }
        "boardrevoke" => {
            let offer_id = p.str(0, "offer_id")?;
            let oid = offer_id.clone();
            blocking(app, move |e| e.revoke_board_offer(&offer_id)).await?;
            tracing::info!(offer = %oid, "offer withdrawn (boardrevoke)");
            Ok(json!({ "revoked": true }))
        }
        "revokeoffersforcoin" => {
            // Reconfigure-time cleanup (#97): Satchel calls this before removing a
            // coin so offers whose pair involves it are withdrawn while pactd still
            // has it — the surviving offers then ride the skip-de-list relaunch.
            let coin_id = p.str(0, "coin_id")?;
            let cid = coin_id.clone();
            let revoked = blocking(app, move |e| e.revoke_offers_for_coin(&coin_id)).await?;
            if !revoked.is_empty() {
                tracing::info!(coin = %cid, count = revoked.len(), "revoked offers for removed coin");
            }
            Ok(json!({ "revoked": revoked }))
        }
        // ---- private (off-market) offers — the Pact handbook (private offers). The maker's
        // offer is built/signed/stored locally but NEVER posted to a board;
        // it travels to a friend as a slip string over their own chat. Mirrors
        // boardpostoffer/boardtake/boardrevoke; no board is touched.
        "makeprivateoffer" => {
            let give = parse_coin_amount(&p.str(0, "give")?)?;
            let get = parse_coin_amount(&p.str(1, "get")?)?;
            let (t1s, t2s) = (p.u32(2, "t1_secs")?, p.u32(3, "t2_secs")?);
            let proto = p.opt_str(4, "protocol");
            // Optional slip validity (param 5, seconds); omitted → engine default.
            let ttl = p.opt_u64(5, "ttl_secs");
            let slip = blocking(app, move |e| {
                e.make_private_offer(net, give, get, t1s, t2s, ttl, proto.as_deref())
            })
            .await?;
            Ok(json!({ "slip": slip }))
        }
        "takeoffer" => {
            // Take a private offer from a pasted slip. Decodes + verifies the
            // signed offer, then relays a `take` to the maker (same path as
            // boardtake, offer sourced from the slip instead of the board).
            let slip = p.str(0, "slip")?;
            blocking(app, move |e| e.take_offer_slip(&slip)).await?;
            Ok(json!({ "taken": true }))
        }
        "listprivateoffers" => {
            let offers = blocking(app, |e| e.list_private_offers()).await?;
            Ok(json!({ "offers": serde_json::to_value(&offers)? }))
        }
        "cancelprivateoffer" => {
            let offer_id = p.str(0, "offer_id")?;
            blocking(app, move |e| e.cancel_private_offer(&offer_id)).await?;
            Ok(json!({ "cancelled": true }))
        }
        "estimateswapfees" => {
            // Fee preview for a prospective swap (C3). `protocol`/`role` are
            // accepted for forward-compat but don't change today's HTLC legs;
            // legs are keyed off give/get (you fund what you give, redeem what
            // you get). `platform_fee_sat` is always 0 — Corkboard takes nothing.
            let give = parse_coin(&p.str(0, "give_coin")?)?;
            let get = parse_coin(&p.str(1, "get_coin")?)?;
            blocking(app, move |e| e.estimate_swap_fees(net, &give, &get)).await
        }
        "getbalance" => {
            let chain = p.str(0, "chain")?;
            let bal = blocking(app, move |e| e.wallet_balance(net, &parse_coin(&chain)?)).await?;
            Ok(json!({ "balance_sat": bal }))
        }
        "getnewaddress" => {
            let chain = p.str(0, "chain")?;
            let addr = blocking(app, move |e| e.wallet_address(net, &parse_coin(&chain)?)).await?;
            Ok(json!({ "address": addr }))
        }
        "sendtoaddress" => {
            let chain = p.str(0, "chain")?;
            let address = p.str(1, "address")?;
            let amount = p.str(2, "amount")?;
            // Fee: an explicit sat/vB rate (the form's Custom field — DECIMAL,
            // e.g. 1.08, carried internally as sat/kvB) wins over a block
            // target (a preset); neither = the 6-block Normal baseline.
            // Targets clamp to Core's estimatesmartfee range (1..=1008).
            let fee = match (p.opt_u64(3, "conf_target"), p.opt_f64(4, "fee_rate")) {
                (_, Some(rate)) if rate > 0.0 => {
                    SendFee::RatePerKvb((rate * 1000.0).round() as u64)
                }
                (Some(target), _) => SendFee::Target(target.clamp(1, 1008) as u16),
                _ => SendFee::Target(6),
            };
            let txid = blocking(app, move |e| {
                let coin = parse_coin(&chain)?;
                // "all" sweeps the wallet (send-everything, phoenix parity):
                // the fee comes out of the swept amount, the wallet ends
                // empty — no user-computed balance-minus-fee guesswork.
                if amount == "all" {
                    e.wallet_send_all(net, &coin, &address, fee)
                } else {
                    let (_, sat) = parse_coin_amount(&format!("{coin}:{amount}"))?;
                    e.wallet_send(net, &coin, &address, sat, fee)
                }
            })
            .await?;
            Ok(json!({ "txid": txid }))
        }
        "estimatesendfee" => {
            // Fee preview for the wallet send form: raw estimator answers for
            // the Slow/Normal/Fast presets (144/6/1 blocks, phoenix parity) —
            // null where the estimator has no data — plus the coin's feerate
            // floor for the Custom field.
            let chain = p.str(0, "chain")?;
            let est = blocking(app, move |e| {
                e.wallet_fee_estimates(net, &parse_coin(&chain)?)
            })
            .await?;
            Ok(serde_json::to_value(est)?)
        }
        "bumpfee" => {
            // RBF-bump an unconfirmed wallet send to `fee_rate` sat/vB (every
            // wallet send is broadcast BIP125-replaceable). The Activity
            // dialog's lever for nodeless coins; node-backed coins bump in
            // the node's own wallet.
            let chain = p.str(0, "chain")?;
            let txid = p.str(1, "txid")?;
            let fee_rate = p.u64(2, "fee_rate")?;
            let new_txid = blocking(app, move |e| {
                e.wallet_bumpfee(net, &parse_coin(&chain)?, &txid, fee_rate)
            })
            .await?;
            Ok(json!({ "txid": new_txid }))
        }
        "listtransactions" => {
            // Activity feed of the nodeless wallet (design doc §4): newest
            // first, each entry txid/direction/amount/fee/confirmations/time.
            // Core-backed coins refuse (their wallet stays read-only here).
            let chain = p.str(0, "chain")?;
            let txs = blocking(app, move |e| {
                e.wallet_transactions(net, &parse_coin(&chain)?)
            })
            .await?;
            Ok(json!({ "transactions": txs }))
        }
        other => Err(method_not_found(other)),
    }
}

/// Read-query board client with an optional selector: honor `sel` only if it is
/// one of the configured boards (so the UI can switch boards), else fall back to
/// the first. Untrusted/unknown URLs are ignored — never query an arbitrary host.
#[derive(serde::Deserialize)]
struct RpcRequest {
    #[serde(default)]
    id: Value,
    method: String,
    #[serde(default)]
    params: Value,
}

async fn rpc(State(app): State<App>, Json(req): Json<RpcRequest>) -> Json<Value> {
    match dispatch(&app, &req.method, req.params).await {
        Ok(result) => Json(json!({ "jsonrpc": "2.0", "id": req.id, "result": result })),
        Err(err) => {
            // JSON-RPC "method not found" gets its standard code; everything
            // else stays -1 for now (typed codes are #57 P2).
            let code = if err.is::<MethodNotFound>() {
                -32601
            } else {
                -1
            };
            Json(json!({
                "jsonrpc": "2.0", "id": req.id,
                "error": { "code": code, "message": format!("{err:#}") }
            }))
        }
    }
}

async fn health() -> &'static str {
    "ok"
}

// ---- auth (HTTP Basic: cookie and/or pact.conf) ------------------------

/// RFC 4648 base64 (used only to build expected Basic headers).
fn base64(input: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in input.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = u32::from(b[0]) << 16 | u32::from(b[1]) << 8 | u32::from(b[2]);
        out.push(A[(n >> 18 & 63) as usize] as char);
        out.push(A[(n >> 12 & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            A[(n >> 6 & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            A[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

fn constant_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

async fn auth(State(app): State<App>, req: Request, next: Next) -> Response {
    if req.uri().path() == "/health" {
        return next.run(req).await;
    }
    let provided = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    let ok = provided
        .map(|h| {
            app.auth_headers
                .iter()
                .any(|expected| constant_eq(h, expected))
        })
        .unwrap_or(false);
    if ok {
        next.run(req).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "auth required: cookie (.cookie) or rpcuser/rpcpassword (pact.conf)" })),
        )
            .into_response()
    }
}

fn read_conf(data_dir: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Ok(text) = std::fs::read_to_string(data_dir.join(CONF_FILE)) {
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                map.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }
    map
}

/// Write `.cookie` (`__cookie__:<hex>`) and return the `user:pass` string.
fn write_cookie(data_dir: &Path) -> Result<String> {
    use bitcoin::secp256k1::rand::RngCore;
    let mut bytes = [0u8; 32];
    bitcoin::secp256k1::rand::thread_rng().fill_bytes(&mut bytes);
    let creds = format!("__cookie__:{}", hex::encode(bytes));
    std::fs::write(data_dir.join(COOKIE_FILE), &creds).context("writing .cookie")?;
    Ok(creds)
}

/// Platform-default data dir (#57 P0), bitcoind-style, nested per network so
/// mainnet/testnet/regtest coexist: mainnet at the root, the test networks
/// beneath it. `network` must already be validated (see [`parse_network`]).
/// pact-cli mirrors this in its cookie autodiscovery — keep the two in sync.
fn default_data_dir(network: &str) -> Result<PathBuf> {
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("Pact"))
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support/Pact"))
    } else {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".pact"))
    }
    .context("no --data-dir and no platform default (HOME/APPDATA unset)")?;
    Ok(match network {
        "mainnet" => base,
        net => base.join(net),
    })
}

fn parse_network(name: &str) -> Result<Network> {
    match name {
        "regtest" => Ok(Network::Regtest),
        "testnet" => Ok(Network::Testnet),
        "mainnet" => Ok(Network::Mainnet),
        other => bail!("unknown network {other:?} (regtest | testnet)"),
    }
}

/// RC2: the pactd log lines that mention `swap_id`, across all rolling
/// `pactd.log*` files in `log_dir` (oldest first). The scheduler tags every
/// swap event with `swap=<id>`, so a substring match isolates that swap's
/// narration. Best-effort: unreadable files are skipped.
fn swap_log_lines(log_dir: &Path, swap_id: &str) -> Vec<String> {
    let mut files: Vec<PathBuf> = match std::fs::read_dir(log_dir) {
        Ok(entries) => entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("pactd.log"))
            })
            .collect(),
        Err(_) => return Vec::new(),
    };
    files.sort();
    let mut lines = Vec::new();
    for f in files {
        if let Ok(content) = std::fs::read_to_string(&f) {
            lines.extend(
                content
                    .lines()
                    .filter(|l| l.contains(swap_id))
                    .map(str::to_string),
            );
        }
    }
    lines
}

/// RC2: redact swap secrets from a serialized record before it leaves the
/// machine in a dev dump. The v1 `SwapRecord` carries the `preimage`; the v2
/// adaptor record stores no secret. Never touches the seed/nonces (not in the
/// record). Defensive: redacts any field named like a secret.
fn scrub_secrets(mut v: Value) -> Value {
    if let Some(obj) = v.as_object_mut() {
        for k in ["preimage", "secret", "seed", "mnemonic", "secnonce"] {
            if obj.contains_key(k) {
                obj.insert(k.to_string(), json!("[redacted]"));
            }
        }
    }
    v
}

#[tokio::main]
async fn main() -> Result<()> {
    // rustls 0.23 needs a process-wide CryptoProvider. nostr-sdk's TLS stack
    // (async-wsocket → tokio-rustls) links BOTH aws-lc-rs and ring, so rustls
    // won't auto-select one — and every wss:// handshake to a Nostr relay then
    // fails inside a background task (the panic goes to stderr, which managed
    // Satchel discards, so the log shows only downstream subscription timeouts
    // and the relays never reach Connected; ws:// works because it skips TLS).
    // Install the provider explicitly, before any relay connection is attempted.
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let args = Args::parse();
    let network = parse_network(&args.network)?;
    // Resolve the data dir (explicit flag or the platform default) and make
    // sure it exists before anything (logging, seed, cookie) writes into it.
    let data_dir = match &args.data_dir {
        Some(d) => d.clone(),
        None => default_data_dir(&args.network)?,
    };
    std::fs::create_dir_all(&data_dir)
        .with_context(|| format!("creating data dir {}", data_dir.display()))?;
    // Keep the file-writer guard alive for the whole process (flushes on drop).
    let _log_guard = init_logging(&data_dir);
    let passphrase = std::env::var("PACT_PASSPHRASE").ok();

    // Load extra coins from coins.toml (if any) BEFORE anything touches the
    // registry — `--coin` arg parsing validates ids against it. A bad file logs
    // and falls back to the built-ins rather than refusing to boot.
    if let Some(path) = &args.coins_file {
        match libswap::registry::init_from_path(path) {
            Ok(dropped) => {
                tracing::info!(coins_file = %path.display(), "loaded coin templates");
                for id in dropped {
                    tracing::warn!(
                        "coins.toml entry {id:?} dropped: collides with a built-in coin id"
                    );
                }
            }
            Err(err) => {
                tracing::error!("coins.toml load failed ({err:#}); using built-in coins only");
            }
        }
    }

    if args.auto_init && !data_dir.join(libswap::store::SEED_FILE).exists() {
        libswap::store::Store::init(&data_dir, passphrase.as_deref())?;
        tracing::info!(data_dir = %data_dir.display(), "first run: created seed + state db");
    }

    // C10: pactd owns the merchant registry. The `--data-dir` is the *parent*;
    // managed (Satchel) installs keep merchants under `merchants/<id>/`, while
    // the harness/`pact-cli`/`--auto-init` shape keeps the seed flat in the
    // root — detected here so that legacy path stays a single `default` merchant.
    let coins = coins_from_args(&args)?;
    let coin_confirmations = coin_confs_from_args(&args)?;
    let flat_seed_present = data_dir.join(libswap::store::SEED_FILE).exists();
    let engine_cfg = EngineConfig {
        coins,
        coin_confirmations,
        board_url: args.board_url.clone(),
        nostr_relays: args.nostr_relay.clone(),
        auto_fund: args.auto_fund,
        passphrase: passphrase.clone(),
    };
    let registry = MerchantRegistry::open(&data_dir, engine_cfg, flat_seed_present, args.merchants)
        .context("opening merchant registry (run with --auto-init or createmerchant first)")?;

    if args.once {
        // One scheduler pass over the active merchant (the flat/CLI seed).
        let engine = registry.active()?;
        let mut events = engine.sync_board();
        events.extend(engine.tick());
        println!("{}", serde_json::to_string_pretty(&events)?);
        std::process::exit(if events.iter().any(|e| e.action == "error") {
            1
        } else {
            0
        });
    }

    anyhow::ensure!(
        args.listen.ip().is_loopback(),
        "pactd binds loopback only (no auth across a network)"
    );

    // Auth material: cookie (always) + optional pact.conf credentials.
    let cookie = write_cookie(&data_dir)?;
    let conf = read_conf(&data_dir);
    let mut creds = vec![cookie];
    if let (Some(u), Some(pw)) = (conf.get("rpcuser"), conf.get("rpcpassword")) {
        creds.push(format!("{u}:{pw}"));
        tracing::info!("pact.conf rpcuser/rpcpassword accepted alongside the cookie");
    }
    let auth_headers: Vec<String> = creds
        .iter()
        .map(|c| format!("Basic {}", base64(c.as_bytes())))
        .collect();

    // One Nostr relay client per process (relays are shared config); the
    // per-merchant identity is supplied per pass. Connect up front so the pool
    // is warm, and carry it in `App` (for boardstatus + the on-login pass).
    let nostr = match args.nostr_relay.as_deref() {
        Some(relays) if !relays.trim().is_empty() => {
            match nostr_service::NostrService::connect(relays).await {
                Ok(svc) => {
                    tracing::info!(relays, "nostr transport enabled");
                    Some(Arc::new(svc))
                }
                Err(err) => {
                    tracing::error!("nostr: connect failed, transport disabled: {err:#}");
                    None
                }
            }
        }
        _ => None,
    };

    let app = App {
        registry: Arc::new(Mutex::new(registry)),
        network,
        auth_headers: Arc::new(auth_headers),
        shutdown: Arc::new(Notify::new()),
        skip_delist_on_stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        nostr,
    };

    if args.tick_secs > 0 {
        let scheduler = app.clone();
        let interval = Duration::from_secs(args.tick_secs);
        tokio::spawn(async move {
            // Re-advertise still-valid offers once, the first tick a merchant is
            // active — so offers soft-de-listed on the last clean close come back
            // immediately on restart (not after up to a full REFRESH_SECS).
            let mut readvertised = false;
            let mut rescue_checked = false;
            loop {
                // Poll-then-sleep: run a pass immediately (no cold-start gap for
                // an already-active merchant), then wait `interval` between
                // passes. The sleep is at the END so it ALWAYS runs — even when
                // no merchant is loaded — so an idle pactd never busy-loops.
                // PHASE 2: iterate every loaded merchant's engine here.
                let has_active = {
                    let reg = scheduler.registry.lock().expect("registry mutex poisoned");
                    reg.active_id().is_some()
                };
                if has_active {
                    // One-time on boot: re-advertise still-valid offers (queues
                    // re-listings) BEFORE the first nostr pass, so they publish in
                    // the same tick.
                    if !readvertised {
                        readvertised = true;
                        match blocking(&scheduler, |e| e.readvertise_offers()).await {
                            Ok(n) if n > 0 => {
                                tracing::info!(count = n, "re-advertised offers on boot")
                            }
                            Ok(_) => {}
                            Err(err) => tracing::warn!("re-advertise on boot failed: {err:#}"),
                        }
                    }
                    // Move Nostr mail/offers into the local buffers *before* the
                    // engine pass, so a take/init that just arrived is dispatched
                    // by sync_board in the same tick.
                    if let Some(svc) = &scheduler.nostr {
                        if let Err(err) = nostr_pass(&scheduler, svc).await {
                            tracing::error!("nostr pass failed: {err:#}");
                        }
                    }
                    // One-time on boot, once a merchant with a readable seed is
                    // active: DETECT any in-flight swap this machine is missing
                    // from our own encrypted relay snapshots (#54) and warn —
                    // never adopt silently (two live machines on one seed can
                    // double-fund a swap). Adoption stays behind the explicit
                    // `restorefromrelay` RPC / `pact-cli restore`. Covers the
                    // headless/CLI boot with no loadmerchant RPC; the UI path
                    // also detects via kick_nostr on load/unlock.
                    if !rescue_checked {
                        match detect_rescue(&scheduler).await {
                            Ok((n, _)) if n > 0 => {
                                rescue_checked = true;
                                tracing::warn!(count = n, "rescue: {}", RESCUE_PENDING_WARNING);
                            }
                            // Only latch once the seed was actually readable (a
                            // locked seed returns an error): keep retrying each
                            // tick until unlock makes detection possible.
                            Ok(_) => rescue_checked = true,
                            Err(err) => {
                                tracing::debug!("rescue: boot detection deferred: {err:#}")
                            }
                        }
                    }
                    match blocking(&scheduler, |e| {
                        let mut ev = e.sync_board();
                        ev.extend(e.tick());
                        ev.extend(e.refresh_offers()?); // roll relay TTLs, retire expired offers
                        Ok(ev)
                    })
                    .await
                    {
                        Ok(events) => {
                            for ev in events {
                                tracing::info!(swap = %ev.swap_id, action = %ev.action, detail = %ev.detail, "scheduler");
                            }
                        }
                        Err(err) => tracing::error!("scheduler pass failed: {err:#}"),
                    }
                }
                tokio::time::sleep(interval).await;
            }
        });
    }

    let router = Router::new()
        .route("/health", get(health))
        .route("/", post(rpc))
        .layer(axum::middleware::from_fn_with_state(app.clone(), auth))
        .with_state(app.clone());

    tracing::info!(listen = %args.listen, cookie = %data_dir.join(COOKIE_FILE).display(), "pactd JSON-RPC listening");
    let listener = tokio::net::TcpListener::bind(args.listen).await?;
    let shutdown = app.shutdown.clone();
    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = shutdown.notified() => {}
            }
        })
        .await?;

    // Offer-lifecycle revoke-on-close: on a clean shutdown (stop RPC or ctrl_c)
    // withdraw our still-live offers so the board stops advertising what we can no
    // longer honor. Best-effort; a crash skips this and the short relay TTL drops
    // the listings within RELAY_TTL_SECS. In C6 detach mode pactd keeps running
    // (Satchel sends no stop), so this only fires on a real stop.
    let has_active = {
        let reg = app.registry.lock().expect("registry mutex poisoned");
        reg.active_id().is_some()
    };
    let skip_delist = app
        .skip_delist_on_stop
        .load(std::sync::atomic::Ordering::SeqCst);
    if has_active && skip_delist {
        // Config-change relaunch (#97): DON'T de-list. Surviving offers keep their
        // relay listings across the ~2s restart; emitting the de-list kind-5 here
        // would get re-read on boot as a self-revocation and permanently withdraw
        // them. Offers on a removed pair were already revoked at reconfigure time.
        tracing::info!(
            "de-list-on-close skipped (config-change relaunch): offers keep their listings"
        );
    } else if has_active {
        // Soft de-list (NOT terminal): drop the relay listings so we don't
        // advertise while offline, but keep the offers live + unblocked so the
        // next startup re-advertises them (see readvertise on boot). The user's
        // explicit "withdraw & exit" goes through ExitGate → revoke_board_offer,
        // which IS terminal.
        match blocking(&app, |e| e.delist_live_offers()).await {
            Ok(n) if n > 0 => tracing::info!(count = n, "de-list-on-close: paused live offers"),
            Ok(_) => {}
            Err(err) => tracing::warn!("de-list-on-close failed: {err:#}"),
        }
    }

    // Cookie is per-run, like bitcoind: remove on clean shutdown.
    let _ = std::fs::remove_file(data_dir.join(COOKIE_FILE));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrub_secrets_redacts_preimage_keeps_rest() {
        // RC2 #3b: the dev dump must never carry the v1 preimage (or any
        // secret-named field), but everything else stays for diagnostics.
        let rec = json!({
            "swap_id": "abcd", "state": "completed", "preimage": "deadbeef",
            "final_txid": "1111", "amount_a": 50_000_000u64
        });
        let scrubbed = scrub_secrets(rec);
        assert_eq!(scrubbed["preimage"], json!("[redacted]"));
        assert_eq!(scrubbed["state"], json!("completed"));
        assert_eq!(scrubbed["final_txid"], json!("1111"));
        // A record without a preimage (e.g. a v2 adaptor record) is untouched.
        let v2 = json!({ "swap_id": "ef01", "state": "signed" });
        assert_eq!(scrub_secrets(v2.clone()), v2);
    }

    #[test]
    fn swap_log_lines_filters_by_id_across_files() {
        let dir = std::env::temp_dir().join(format!("pactd-dumptest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("pactd.log.2026-06-19"),
            "INFO scheduler swap=aaaa action=funded\nINFO scheduler swap=bbbb action=redeem\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("pactd.log.2026-06-20"),
            "INFO scheduler swap=aaaa action=completed\nINFO other unrelated line\n",
        )
        .unwrap();
        let lines = swap_log_lines(&dir, "aaaa");
        assert_eq!(lines.len(), 2, "two lines mention aaaa across both files");
        assert!(lines.iter().all(|l| l.contains("aaaa")));
        assert!(lines.iter().any(|l| l.contains("funded")));
        assert!(lines.iter().any(|l| l.contains("completed")));
        // Unknown id / missing dir → empty, no panic.
        assert!(swap_log_lines(&dir, "zzzz").is_empty());
        assert!(swap_log_lines(&dir.join("nope"), "aaaa").is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn edit_distance_basics() {
        assert_eq!(edit_distance("getbalance", "getbalance"), 0);
        assert_eq!(edit_distance("getblance", "getbalance"), 1);
        assert_eq!(edit_distance("", "abc"), 3);
        assert_eq!(edit_distance("abc", ""), 3);
    }

    #[test]
    fn unknown_method_suggests_nearest() {
        let msg = format!("{}", method_not_found("getblance"));
        assert!(msg.contains("did you mean 'getbalance'"), "{msg}");
        // Something far from every method gets no misleading suggestion.
        let msg = format!("{}", method_not_found("zzzzzzzzzzzzzzzz"));
        assert!(!msg.contains("did you mean"), "{msg}");
    }

    #[test]
    fn help_lists_and_details() {
        let all = render_help(None).unwrap();
        assert!(all.contains("== control ==") && all.contains("getbalance <coin>"));
        let one = render_help(Some("abort")).unwrap();
        assert!(one.starts_with("abort <swap_id> [reason]"), "{one}");
        // Case-insensitive lookup; unknown topic errors with the suggestion.
        assert!(render_help(Some("GETINFO")).is_ok());
        assert!(render_help(Some("nope")).is_err());
    }

    #[tokio::test]
    async fn catalog_matches_dispatch() {
        // Every cataloged method must be recognized by `dispatch` — a
        // fall-through to "unknown method" means METHODS drifted from the
        // match. An empty registry is enough: recognition happens before any
        // engine work (those calls fail later, with a different error).
        let dir = std::env::temp_dir().join(format!("pactd-catalog-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = EngineConfig {
            coins: BTreeMap::new(),
            coin_confirmations: BTreeMap::new(),
            board_url: None,
            nostr_relays: None,
            auto_fund: false,
            passphrase: None,
        };
        let registry = MerchantRegistry::open(&dir, cfg, false, false).unwrap();
        let app = App {
            registry: Arc::new(Mutex::new(registry)),
            network: Network::Regtest,
            auth_headers: Arc::new(Vec::new()),
            shutdown: Arc::new(Notify::new()),
            skip_delist_on_stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            nostr: None,
        };
        for (_, name, _, _) in METHODS {
            if let Err(e) = dispatch(&app, name, json!([])).await {
                assert!(
                    !format!("{e:#}").contains("unknown method"),
                    "'{name}' is cataloged but not dispatched"
                );
            }
        }
        // And a typo'd call is refused with the -32601 marker + suggestion.
        let err = dispatch(&app, "getblance", json!([])).await.unwrap_err();
        assert!(err.is::<MethodNotFound>());
        assert!(format!("{err}").contains("did you mean 'getbalance'"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn default_data_dir_nests_test_networks() {
        // mainnet at the root, test networks nested beneath it.
        let main = default_data_dir("mainnet").unwrap();
        let reg = default_data_dir("regtest").unwrap();
        assert!(reg.ends_with("regtest"));
        assert!(reg.starts_with(&main));
        assert!(default_data_dir("testnet").unwrap().ends_with("testnet"));
    }

    #[test]
    fn params_str_coerces_scalars() {
        // The CLI JSON-parses args, so `sendtoaddress btc <addr> 0.5` arrives
        // as a number — str() must render scalars back, not reject them.
        let p = Params(json!(["plain", 0.5, 12, true, [1]]));
        assert_eq!(p.str(0, "a").unwrap(), "plain");
        assert_eq!(p.str(1, "b").unwrap(), "0.5");
        assert_eq!(p.str(2, "c").unwrap(), "12");
        assert_eq!(p.str(3, "d").unwrap(), "true");
        assert!(p.str(4, "e").is_err());
    }

    #[test]
    fn coin_arg_parsing() {
        // coin_id=urls, case-normalized, multi-backend list preserved.
        assert_eq!(
            parse_coin_arg("btcx=http://u:p@host:1/wallet/x").unwrap(),
            ("btcx".to_string(), "http://u:p@host:1/wallet/x".to_string())
        );
        assert_eq!(
            parse_coin_arg("BTC=http://a:1,tcp://b:2").unwrap(),
            ("btc".to_string(), "http://a:1,tcp://b:2".to_string())
        );
        // Unknown coin, missing '=', and an empty URL list are all rejected.
        assert!(parse_coin_arg("doge=http://a:1").is_err());
        assert!(parse_coin_arg("btcx").is_err());
        assert!(parse_coin_arg("btcx=").is_err());
    }

    fn args_with(coins: Vec<&str>) -> Args {
        Args {
            data_dir: Some(PathBuf::from(".")),
            coins_file: None,
            coins: coins.into_iter().map(String::from).collect(),
            coin_confs: vec![],
            listen: "127.0.0.1:9737".parse().unwrap(),
            network: "regtest".into(),
            board_url: None,
            nostr_relay: None,
            auto_fund: false,
            tick_secs: 0,
            once: false,
            auto_init: false,
            merchants: false,
        }
    }

    #[test]
    fn coins_map_is_built_from_generic_coin_flags() {
        // Every coin is attached the same way, via repeated --coin flags.
        let coins =
            coins_from_args(&args_with(vec!["btcx=http://btcx:1", "btc=http://btc:1"])).unwrap();
        assert_eq!(coins.get("btcx").unwrap(), "http://btcx:1");
        assert_eq!(coins.get("btc").unwrap(), "http://btc:1");
        assert_eq!(coins.len(), 2);

        // A later --coin for the same id wins (last write).
        let dup =
            coins_from_args(&args_with(vec!["btcx=http://old:1", "btcx=http://new:9"])).unwrap();
        assert_eq!(dup.get("btcx").unwrap(), "http://new:9");
        assert_eq!(dup.len(), 1);
    }
}
