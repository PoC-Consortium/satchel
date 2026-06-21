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
    /// Data directory (seed, SQLite state, .cookie, pact.conf).
    #[arg(long)]
    data_dir: PathBuf,
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
    });
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
        Ok(self
            .get(i, name)?
            .as_str()
            .with_context(|| format!("param '{name}' must be a string"))?
            .to_string())
    }
    fn u32(&self, i: usize, name: &str) -> Result<u32> {
        let v = self.get(i, name)?;
        // Accept number or numeric string (bitcoin-cli sends strings).
        v.as_u64()
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            .with_context(|| format!("param '{name}' must be a u32"))
            .map(|n: u64| n as u32)
    }
    fn opt_u64(&self, i: usize, name: &str) -> Option<u64> {
        let v = self.get(i, name).ok()?;
        v.as_u64()
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    }
    fn opt_str(&self, i: usize, name: &str) -> Option<String> {
        self.get(i, name)
            .ok()
            .and_then(|v| v.as_str())
            .map(str::to_string)
    }
    fn envelope(&self, i: usize, name: &str) -> Result<Envelope> {
        serde_json::from_value(self.get(i, name)?.clone())
            .with_context(|| format!("param '{name}' is not an envelope"))
    }
}

/// Flattened JSON view of a fee-bump policy for `get/setfeepolicy` — one level so
/// the CLI/UI sets each field by a simple typed name (no nested objects). The two
/// `step_pct` fields are kept in sync (single knob, decision #3), so the redeem
/// value represents both.
fn fee_policy_json(p: &libswap::FeeBumpPolicy) -> Value {
    json!({
        "max_feerate_sat_vb": p.max_feerate_sat_vb,
        "min_fee_sat": p.min_fee_sat,
        "reservation_mult": p.funding.reservation_mult,
        "committed_mult": p.redeem.committed_mult,
        "step_pct": p.redeem.step_pct,
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

/// Dispatch one JSON-RPC method. Result payloads match the old REST bodies
/// so clients only change transport, not shapes.
async fn dispatch(app: &App, method: &str, params: Value) -> Result<Value> {
    let p = Params(params);
    let net = app.network;
    match method {
        "getinfo" => {
            // Tolerate a missing/locked seed: a fresh merchant (first run) or
            // a locked encrypted one has no identity to show yet. The UI uses
            // seed_exists/locked to drive the wizard / unlock prompt.
            let (status, identity, coins) = blocking(app, |e| {
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
                Ok((status, identity, e.configured_coins()))
            })
            .await?;
            Ok(json!({
                "name": "pactd",
                "version": env!("CARGO_PKG_VERSION"),
                "protocol": libswap::PROTOCOL_VERSION,
                "network": format!("{net:?}").to_lowercase(),
                "identity": identity,
                "seed_exists": status.seed_exists,
                "encrypted": status.encrypted,
                "locked": status.locked,
                "coins": coins,
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
        // fields supplied change. `step_pct` sets both the redeem and refund step
        // (single knob). Validated server-side; persisted; applied live.
        "setfeepolicy" => {
            let max = p.opt_u64(0, "max_feerate_sat_vb");
            let min = p.opt_u64(1, "min_fee_sat");
            let reservation = p.opt_u64(2, "reservation_mult");
            let committed = p.opt_u64(3, "committed_mult");
            let step = p.opt_u64(4, "step_pct");
            let policy = blocking_mut(app, move |e| {
                let mut pol = e.fee_bump;
                if let Some(v) = max {
                    pol.max_feerate_sat_vb = v;
                }
                if let Some(v) = min {
                    pol.min_fee_sat = v;
                }
                if let Some(v) = reservation {
                    pol.funding.reservation_mult = v;
                }
                if let Some(v) = committed {
                    pol.redeem.committed_mult = v;
                }
                if let Some(v) = step {
                    pol.redeem.step_pct = v;
                    pol.refund.step_pct = v;
                }
                e.set_fee_bump(pol)?;
                Ok(e.fee_bump)
            })
            .await?;
            Ok(fee_policy_json(&policy))
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
                }));
            }
            Ok(json!({ "network": format!("{net:?}").to_lowercase(), "coins": coins }))
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
            let mnemonic =
                blocking_mut(app, move |e| e.store.create_seed(passphrase.as_deref())).await?;
            kick_nostr(app);
            // The mnemonic is returned exactly once, for the user to back up.
            Ok(json!({ "mnemonic": mnemonic, "encrypted": encrypted }))
        }
        // Generate a fresh mnemonic WITHOUT persisting it — the onboarding flow
        // shows + confirms it, then commits via `importseed`. Read-only.
        "generateseed" => {
            let mnemonic = blocking(app, |e| e.store.generate_mnemonic()).await?;
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
        "unloadmerchant" => {
            blocking_registry(app, |r| r.unload()).await?;
            Ok(json!({ "unloaded": true }))
        }
        "getmerchantinfo" => {
            let id = p.opt_str(0, "id");
            Ok(blocking_registry(app, move |r| r.info(id.as_deref())).await?)
        }
        "stop" => {
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
                // record (stores no secret — `t` is never persisted).
                let record = match engine.store.get(&id) {
                    Ok(rec) => scrub_secrets(serde_json::to_value(&rec)?),
                    Err(_) => serde_json::to_value(engine.store.get_adaptor(&id)?)?,
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
            let (record, envelope) = blocking(app, move |e| e.fund(&id)).await?;
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
            let record = blocking(app, move |e| e.abort(&id, &reason)).await?;
            Ok(json!({ "record": serde_json::to_value(&record)? }))
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
                Some(svc) => svc.relay_status().await,
                None => Vec::new(),
            };
            let relays: Vec<Value> = relays
                .into_iter()
                .map(|(url, connected)| json!({ "url": url, "connected": connected }))
                .collect();
            Ok(json!({ "relays": relays }))
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
            blocking(app, move |e| e.revoke_board_offer(&offer_id)).await?;
            Ok(json!({ "revoked": true }))
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
            let txid = blocking(app, move |e| {
                let coin = parse_coin(&chain)?;
                let (_, sat) = parse_coin_amount(&format!("{coin}:{amount}"))?;
                e.wallet_send(net, &coin, &address, sat)
            })
            .await?;
            Ok(json!({ "txid": txid }))
        }
        other => bail!("unknown method '{other}'"),
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
        Err(err) => Json(json!({
            "jsonrpc": "2.0", "id": req.id,
            "error": { "code": -1, "message": format!("{err:#}") }
        })),
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
    let args = Args::parse();
    // Keep the file-writer guard alive for the whole process (flushes on drop).
    let _log_guard = init_logging(&args.data_dir);
    let network = parse_network(&args.network)?;
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

    if args.auto_init && !args.data_dir.join(libswap::store::SEED_FILE).exists() {
        libswap::store::Store::init(&args.data_dir, passphrase.as_deref())?;
        tracing::info!(data_dir = %args.data_dir.display(), "first run: created seed + state db");
    }

    // C10: pactd owns the merchant registry. The `--data-dir` is the *parent*;
    // managed (Satchel) installs keep merchants under `merchants/<id>/`, while
    // the harness/`pact-cli`/`--auto-init` shape keeps the seed flat in the
    // root — detected here so that legacy path stays a single `default` merchant.
    let coins = coins_from_args(&args)?;
    let coin_confirmations = coin_confs_from_args(&args)?;
    let flat_seed_present = args.data_dir.join(libswap::store::SEED_FILE).exists();
    let engine_cfg = EngineConfig {
        coins,
        coin_confirmations,
        board_url: args.board_url.clone(),
        nostr_relays: args.nostr_relay.clone(),
        auto_fund: args.auto_fund,
        passphrase: passphrase.clone(),
    };
    let registry = MerchantRegistry::open(
        &args.data_dir,
        engine_cfg,
        flat_seed_present,
        args.merchants,
    )
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
    let cookie = write_cookie(&args.data_dir)?;
    let conf = read_conf(&args.data_dir);
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
        nostr,
    };

    if args.tick_secs > 0 {
        let scheduler = app.clone();
        let interval = Duration::from_secs(args.tick_secs);
        tokio::spawn(async move {
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
                    // Move Nostr mail/offers into the local buffers *before* the
                    // engine pass, so a take/init that just arrived is dispatched
                    // by sync_board in the same tick.
                    if let Some(svc) = &scheduler.nostr {
                        if let Err(err) = nostr_pass(&scheduler, svc).await {
                            tracing::error!("nostr pass failed: {err:#}");
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

    tracing::info!(listen = %args.listen, cookie = %args.data_dir.join(COOKIE_FILE).display(), "pactd JSON-RPC listening");
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
    if has_active {
        match blocking(&app, |e| e.revoke_live_offers()).await {
            Ok(n) if n > 0 => tracing::info!(count = n, "revoke-on-close: withdrew live offers"),
            Ok(_) => {}
            Err(err) => tracing::warn!("revoke-on-close failed: {err:#}"),
        }
    }

    // Cookie is per-run, like bitcoind: remove on clean shutdown.
    let _ = std::fs::remove_file(args.data_dir.join(COOKIE_FILE));
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
            data_dir: PathBuf::from("."),
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
