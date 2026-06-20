//! Satchel — the desktop face of Pact (the bitcoin-qt of this stack).
//!
//! Responsibilities, all of which keep swap logic out of the GUI:
//! 1. Own pactd's lifecycle (the phoenix-pocx NodeManager pattern):
//!    spawn it (managed), or adopt one already listening, or attach to an
//!    external one (`SATCHEL_PACTD_URL`); stop it gracefully via the RPC
//!    `stop` on exit.
//! 2. Hold the `.cookie` and proxy every UI call through the `pactd_rpc`
//!    command — so the webview never sees the cookie and there is no
//!    cross-origin/auth problem.
//! 3. Serve the bundled Pact UI (`ui/`).
//! 4. **Merchant manager** (C10): a *merchant* = one Pact seed = one trading
//!    identity = one data dir — but the registry is now owned by **pactd**, the
//!    Bitcoin-Core wallet analog. Satchel launches a single pactd at a parent
//!    data dir; it owns `merchants/<id>/` + a manifest, and switches the active
//!    merchant in-process (no relaunch) via the `createmerchant` / `listmerchants`
//!    / `loadmerchant` / `unloadmerchant` / `getmerchantinfo` RPCs. The old
//!    Satchel-side `merchants[]`/`active_merchant` registry is gone (it was the
//!    one place Satchel held state it shouldn't, and the two-source-of-truth
//!    desync bug). Node connections stay machine-level (shared across merchants);
//!    the active network is launch-determined (see `active_network`), one per
//!    install. Satchel NEVER persists a passphrase or seed and uses no OS
//!    keystore: an encrypted merchant is unlocked per session, in memory inside
//!    pactd, forgotten on exit.
//!
//! Config: `satchel.json` in the app config dir (chain backends, boards,
//! auto_fund, plus per-install UI prefs), created with defaults on first run.
//! Merchant ownership lives in pactd, not here.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde_json::json;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::Duration;
use tauri::Manager;

mod coins_file;
mod compose;

/// One configured coin (Phase C). Machine-level, shared across merchants: a
/// coin's chain-data backend + funding wallet is a property of this host, not
/// of a trading identity. `chain_data` is the comma-separated backend URL list
/// pactd turns into a `MultiBackend`; the first URL is the wallet-qualified
/// Core-RPC primary that also funds swaps.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct CoinConn {
    coin_id: String,
    /// The composed backend URL list pactd consumes (the single string passed on
    /// `--coin`). Derived from the structured fields below at save time, and
    /// recomposed at launch for cookie auth; kept as the source of truth so a
    /// legacy entry (structured fields absent) still works, and as the fallback
    /// if recomposition fails (e.g. the node's cookie isn't readable yet).
    chain_data: String,
    /// Funding wallet kind — only "core-rpc" for now (Ledger/PSBT later).
    #[serde(default = "default_funding")]
    funding_wallet: String,
    /// Confirmation depth (reorg-safety/finality) for this coin: how many
    /// confirmations before a funding/redeem is treated final. `None` means
    /// use pactd's network/spacing default. Passed to pactd as `--coin-confs`.
    #[serde(default)]
    confirmations: Option<u32>,

    // ---- structured connection (coin-setup v2) ----------------------------
    // All optional + `#[serde(default)]` so a pre-v2 satchel.json (only
    // `chain_data`) still deserializes. When `auth_method` is set, Satchel owns
    // composition of `chain_data` from these; when it is `None`, `chain_data` is
    // a raw/legacy string used verbatim.
    #[serde(default)]
    rpc_host: Option<String>,
    #[serde(default)]
    rpc_port: Option<u16>,
    /// "cookie" | "userpass".
    #[serde(default)]
    auth_method: Option<String>,
    #[serde(default)]
    rpc_user: Option<String>,
    #[serde(default)]
    rpc_password: Option<String>,
    /// Node data directory (for cookie discovery). OS tokens already expanded.
    #[serde(default)]
    datadir: Option<String>,
    /// `.cookie` location relative to `datadir` (network-default when absent).
    #[serde(default)]
    cookie_subpath: Option<String>,
    /// Wallet name for the wallet-qualified Core-RPC URL (`/wallet/<name>`).
    #[serde(default)]
    wallet: Option<String>,
    /// Extra read-only backends (e.g. `tcp://host:port` Electrum), appended
    /// after the primary Core-RPC URL.
    #[serde(default)]
    extra_backends: Vec<String>,
}

/// The structured connection payload the UI sends to `save_coin` /
/// `compose_coin_url`. Mirrors the [`CoinConn`] connection fields; Satchel turns
/// it into a `CoinConn` (composing `chain_data`).
#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct CoinConnInput {
    rpc_host: Option<String>,
    rpc_port: Option<u16>,
    auth_method: Option<String>,
    rpc_user: Option<String>,
    rpc_password: Option<String>,
    datadir: Option<String>,
    cookie_subpath: Option<String>,
    wallet: Option<String>,
    extra_backends: Vec<String>,
    funding_wallet: Option<String>,
    /// Expert/legacy escape hatch: a raw URL string that overrides composition.
    chain_data: Option<String>,
}

impl CoinConnInput {
    /// Build a [`CoinConn`] for `coin_id`, composing `chain_data` from the
    /// structured fields (unless a raw `chain_data` was supplied).
    fn into_conn(
        self,
        coin_id: String,
        network: &str,
        confirmations: Option<u32>,
    ) -> anyhow::Result<CoinConn> {
        let mut conn = CoinConn {
            coin_id,
            chain_data: String::new(),
            funding_wallet: self.funding_wallet.unwrap_or_else(default_funding),
            confirmations: confirmations.filter(|n| *n >= 1),
            rpc_host: self.rpc_host,
            rpc_port: self.rpc_port,
            auth_method: self.auth_method,
            rpc_user: self.rpc_user,
            rpc_password: self.rpc_password,
            datadir: self.datadir,
            cookie_subpath: self.cookie_subpath,
            wallet: self.wallet,
            extra_backends: self.extra_backends,
        };
        conn.chain_data = match self.chain_data {
            Some(raw) if !raw.trim().is_empty() => {
                conn.auth_method = None; // raw entry: don't recompose at launch
                raw
            }
            _ => compose::compose_chain_data(&conn, network)?,
        };
        Ok(conn)
    }
}

fn default_funding() -> String {
    "core-rpc".into()
}

/// Per-install UI preferences (UI-1). These used to live in the webview's
/// `localStorage`; they move into `satchel.json` so the UI persists nothing of
/// its own (the same "Satchel owns config, the webview is a thin client"
/// principle that C10 applies to merchants). Per-install, NOT per-merchant.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(default)]
struct UiPrefs {
    /// "dark" | "light" | "system" (MUI color scheme).
    theme: String,
    /// i18n language code (e.g. "en").
    language: String,
    /// Whether the collapsible left nav is expanded.
    nav_open: bool,
}

impl Default for UiPrefs {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            language: "en".into(),
            nav_open: true,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(default)]
struct Config {
    pactd_path: String,
    /// Machine-level per-coin connections (shared across every merchant).
    coins: Vec<CoinConn>,
    board_urls: Vec<String>,
    /// Nostr relay URLs for the decentralized transport (docs/NOSTR_TRANSPORT.md).
    /// Prewired to [`RECOMMENDED_NOSTR_RELAYS`] on a fresh install — the
    /// container-level `#[serde(default)]` fills a satchel.json that omits the
    /// field from `Config::default`, so it lands on the default relay set. An
    /// explicit empty list the user saved is respected (transport off); the
    /// playground writes its own satchel.json, overriding this.
    nostr_relays: Vec<String>,
    listen: String,
    /// RC2: auto-fund both swap legs by default. Safe because offers are
    /// one-shot, so a maker's exposure is bounded by their posted book. Users
    /// can turn it off (→ manual funding + the funding-required alert) via the
    /// Settings toggle, which calls the `setautofund` RPC and persists here.
    auto_fund: bool,
    tick_secs: u64,
    /// Per-install UI preferences (UI-1), persisted here instead of localStorage.
    ui: UiPrefs,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pactd_path: "pactd".into(),
            coins: Vec::new(),
            board_urls: Vec::new(),
            nostr_relays: RECOMMENDED_NOSTR_RELAYS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            listen: default_listen().into(),
            auto_fund: true,
            tick_secs: 30,
            ui: UiPrefs::default(),
        }
    }
}

/// The active Bitcoin network for this Satchel install, selected once at
/// startup, Bitcoin-Core style: **mainnet by default**, with `-testnet` /
/// `-regtest` launch flags selecting a test network (a `SATCHEL_NETWORK` env
/// var is also honoured, for dev/playground launches where forwarding a flag
/// through `cargo tauri dev` is awkward). The network nests the data dir (see
/// [`network_subdir`]) so all three can run side by side.
///
/// Note: mainnet swaps are still hard-gated in libswap's engine admission
/// policy, so defaulting here to mainnet does not by itself enable trading.
fn active_network() -> &'static str {
    static NET: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();
    NET.get_or_init(|| {
        let norm = |s: &str| match s.trim().trim_start_matches('-') {
            "testnet" => Some("testnet"),
            "regtest" => Some("regtest"),
            "mainnet" => Some("mainnet"),
            _ => None,
        };
        // A launch flag wins; then the env var; else mainnet.
        std::env::args()
            .skip(1)
            .find_map(|a| norm(&a))
            .or_else(|| std::env::var("SATCHEL_NETWORK").ok().and_then(|v| norm(&v)))
            .unwrap_or("mainnet")
    })
}

/// Bitcoin-Core-style per-network data subdir: mainnet lives at the root,
/// test networks nest beneath it, so the three coexist without clobbering.
fn network_subdir(network: &str) -> Option<&'static str> {
    match network {
        // Bitcoin PoCX's testnet datadir is "testnet" (Bitcoin's is "testnet3");
        // Satchel is the PoCX app, so it follows the PoCX convention.
        "testnet" => Some("testnet"),
        "regtest" => Some("regtest"),
        _ => None, // mainnet → root
    }
}

/// The per-network config dir under the app's base config dir (where
/// satchel.json, the pactd data dir, and the running-pactd hand-off live).
fn net_config_dir(base: &Path) -> PathBuf {
    match network_subdir(active_network()) {
        Some(sub) => base.join(sub),
        None => base.to_path_buf(),
    }
}

/// Default managed-pactd listen address, offset per network (like Core's
/// 8332/18332/18443) so the three instances don't fight over one port.
fn default_listen() -> &'static str {
    match active_network() {
        "testnet" => "127.0.0.1:9738",
        "regtest" => "127.0.0.1:9739",
        _ => "127.0.0.1:9737",
    }
}

/// Mutable app state: the config (persisted to satchel.json on every change)
/// and where it lives. pactd's data dir (which owns merchants) hangs off it.
struct AppState {
    config: Mutex<Config>,
    config_dir: PathBuf,
}

/// What `pactd_rpc` needs to reach the single managed pactd. `auth` is empty
/// until pactd has come up and we've read its `.cookie`. The active merchant is
/// chosen *inside* pactd now (C10), so this no longer changes on a merchant
/// switch — only the cookie can change on a relaunch (config edit).
#[derive(Clone)]
struct RpcConn {
    url: String,
    auth: String,
}

struct RpcState(Mutex<RpcConn>);

/// The managed pactd child (None in external/adopt mode — not ours to kill).
struct ManagedPactd(Mutex<Option<Child>>);

/// C6: when true, the GUI is exiting but the managed pactd must be LEFT RUNNING
/// (detached) — so the `RunEvent::Exit` handler skips the graceful `stop`. Set by
/// `quit_app { keep_running: true }` after the runtime hand-off file is written.
struct DetachFlag(Mutex<bool>);

/// C6: the small runtime hand-off file (`running-pactd.json` under the config
/// dir, NOT satchel.json) that records a *detached* managed pactd so the next
/// launch can re-adopt it instead of spawning a second one. It is created when we
/// detach (keep-running on close) and removed once a launch has adopted it or
/// found it dead. Carries only the coordinates needed to re-attach: where it
/// listens, where its data dir + cookie live, and the pid (best-effort cleanup).
#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct RunningPactd {
    /// The `host:port` pactd is listening on (matches `Config.listen`).
    listen: String,
    /// pactd's parent data dir — where `.cookie` and `merchants/` live.
    data_dir: String,
    /// OS pid of the detached pactd (best-effort; used only for logging/cleanup).
    pid: u32,
}

fn running_pactd_path(config_dir: &Path) -> PathBuf {
    config_dir.join("running-pactd.json")
}

/// Read the detached-pactd hand-off file, if present and parseable.
fn read_running_pactd(config_dir: &Path) -> Option<RunningPactd> {
    let text = std::fs::read_to_string(running_pactd_path(config_dir)).ok()?;
    serde_json::from_str(&text).ok()
}

fn write_running_pactd(config_dir: &Path, rec: &RunningPactd) -> anyhow::Result<()> {
    std::fs::write(
        running_pactd_path(config_dir),
        serde_json::to_string_pretty(rec)?,
    )?;
    Ok(())
}

/// Drop the hand-off file (the detached pactd is gone, adopted, or stale).
fn clear_running_pactd(config_dir: &Path) {
    let _ = std::fs::remove_file(running_pactd_path(config_dir));
}

fn config_path(config_dir: &Path) -> PathBuf {
    config_dir.join("satchel.json")
}

fn load_or_create_config(config_dir: &Path) -> anyhow::Result<Config> {
    std::fs::create_dir_all(config_dir)?;
    let path = config_path(config_dir);
    if let Ok(text) = std::fs::read_to_string(&path) {
        return Ok(serde_json::from_str(&text)?);
    }
    let config = Config::default();
    std::fs::write(&path, serde_json::to_string_pretty(&config)?)?;
    Ok(config)
}

fn save_config(config_dir: &Path, config: &Config) -> anyhow::Result<()> {
    std::fs::write(
        config_path(config_dir),
        serde_json::to_string_pretty(config)?,
    )?;
    Ok(())
}

/// The single pactd's parent data dir: `<config_dir>/pactd`. pactd owns the
/// `merchants/<id>/` subdirs + the manifest under it (C10) — Satchel no longer
/// computes per-merchant paths.
fn pactd_data_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("pactd")
}

fn resolve_pactd(configured: &str) -> PathBuf {
    if configured != "pactd" {
        return PathBuf::from(configured);
    }
    if let Ok(me) = std::env::current_exe() {
        let sibling = me.with_file_name(if cfg!(windows) { "pactd.exe" } else { "pactd" });
        if sibling.exists() {
            return sibling;
        }
    }
    PathBuf::from("pactd")
}

fn health_ok(listen: &str) -> bool {
    let Ok(mut stream) = TcpStream::connect(listen) else {
        return false;
    };
    let req = format!("GET /health HTTP/1.1\r\nHost: {listen}\r\nConnection: close\r\n\r\n");
    if stream.write_all(req.as_bytes()).is_err() {
        return false;
    }
    let mut resp = String::new();
    let _ = stream.read_to_string(&mut resp);
    resp.contains("200")
}

/// Spawn the single managed pactd at its parent data dir (C10). Deliberately
/// **no `--auto-init`**: merchants + seeds are created explicitly through the
/// wizard (`createmerchant` then `createseed`/`importseed`), so encryption is
/// always the user's choice. pactd reloads its previously-active merchant from
/// the manifest; an encrypted seed comes up *locked* until the UI unlocks it.
fn spawn_pactd(config: &Config, data_dir: &Path) -> anyhow::Result<Child> {
    let mut cmd = Command::new(resolve_pactd(&config.pactd_path));
    cmd.arg("--data-dir")
        .arg(data_dir)
        .arg("--listen")
        .arg(&config.listen)
        .arg("--network")
        .arg(active_network())
        // C10: opt into pactd's owned merchant registry (`merchants/<id>/`).
        // Satchel always wants this; the CLI/harness omit it for the flat layout.
        .arg("--merchants")
        .arg("--tick-secs")
        .arg(config.tick_secs.to_string());
    // Hand pactd the same coin templates Satchel's picker uses, so the engine
    // knows about any file-added coins. `<config_dir>/pactd` is `data_dir`, so
    // its parent is the config dir where the file is resolved.
    if let Some(config_dir) = data_dir.parent() {
        let coins_file = coins_file::resolve_coins_file(config_dir);
        cmd.arg("--coins-file").arg(&coins_file);
    }
    for coin in &config.coins {
        // Recompose cookie-auth URLs at launch (re-read the .cookie); fall back
        // to the stored string for raw/legacy entries or if the node is down.
        let chain_data = compose::effective_chain_data(coin, active_network());
        if !chain_data.trim().is_empty() {
            cmd.arg("--coin")
                .arg(format!("{}={}", coin.coin_id, chain_data));
            if let Some(n) = coin.confirmations {
                cmd.arg("--coin-confs")
                    .arg(format!("{}={}", coin.coin_id, n));
            }
        }
    }
    if !config.board_urls.is_empty() {
        cmd.arg("--board-url").arg(config.board_urls.join(","));
    }
    if !config.nostr_relays.is_empty() {
        cmd.arg("--nostr-relay").arg(config.nostr_relays.join(","));
    }
    if config.auto_fund {
        cmd.arg("--auto-fund");
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    Ok(cmd.spawn()?)
}

/// C6 re-adopt probe: is the detached pactd recorded in `running-pactd.json`
/// still genuinely alive and ours to talk to? Requires BOTH a healthy listen and
/// a working cookie (a stale file pointing at a dead or replaced daemon must fail
/// here so we clean it and spawn fresh). Returns the cookie on success.
fn probe_adoptable(listen: &str, data_dir: &Path) -> Option<String> {
    if !health_ok(listen) {
        return None;
    }
    let cookie = std::fs::read_to_string(data_dir.join(".cookie"))
        .ok()?
        .trim()
        .to_string();
    if cookie.is_empty() {
        return None;
    }
    // Confirm the cookie actually authenticates against THIS daemon (a benign,
    // always-available RPC) — proves it's our pactd, not a stranger on the port.
    pactd_call(&format!("http://{listen}"), &cookie, "getinfo", &json!([])).ok()?;
    Some(cookie)
}

fn wait_health(listen: &str, secs: u64) -> anyhow::Result<()> {
    let deadline = std::time::Instant::now() + Duration::from_secs(secs);
    while std::time::Instant::now() < deadline {
        if health_ok(listen) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    anyhow::bail!("pactd did not come up on {listen}")
}

/// Stop the managed pactd, if any (graceful RPC `stop`, then kill as a
/// fallback — the bitcoin-qt shutdown pattern). Safe to call when nothing is
/// running.
fn stop_managed(app: &tauri::AppHandle) {
    let managed = app.state::<ManagedPactd>();
    let child = managed.0.lock().unwrap().take();
    if let Some(mut child) = child {
        let conn = app.state::<RpcState>().0.lock().unwrap().clone();
        if !conn.auth.is_empty() {
            let _ = pactd_call(&conn.url, &conn.auth, "stop", &json!([]));
        }
        std::thread::sleep(Duration::from_millis(800));
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// (Re)launch the single managed pactd at its parent data dir, re-pointing the
/// RPC bridge at the fresh cookie. Used to apply a machine-level config change
/// (coins / board) — pactd reloads its active merchant from its own manifest,
/// so no merchant context is passed (C10: merchant selection is a pactd RPC,
/// not a relaunch). No-op in external/adopt mode (handled by the caller).
fn relaunch_pactd(app: &tauri::AppHandle) -> anyhow::Result<()> {
    let state = app.state::<AppState>();
    let (config, data_dir) = {
        let cfg = state.config.lock().unwrap();
        (cfg.clone(), pactd_data_dir(&state.config_dir))
    };
    std::fs::create_dir_all(&data_dir)?;

    stop_managed(app);
    let child = spawn_pactd(&config, &data_dir)?;
    wait_health(&config.listen, 30)?;
    let cookie = std::fs::read_to_string(data_dir.join(".cookie"))?
        .trim()
        .to_string();

    *app.state::<RpcState>().0.lock().unwrap() = RpcConn {
        url: format!("http://{}", config.listen),
        auth: cookie,
    };
    *app.state::<ManagedPactd>().0.lock().unwrap() = Some(child);
    Ok(())
}

// ---- Tauri commands ----------------------------------------------------
//
// C10: the merchant registry moved into pactd. The old `list_merchants` /
// `create_merchant` / `select_merchant` / `set_active_identity` Satchel
// commands are gone — the UI now calls the pactd RPCs (`listmerchants`,
// `createmerchant`, `loadmerchant`, `getmerchantinfo`) through `pactd_rpc`.
// Satchel keeps only daemon-level config commands (coins / board / UI prefs).

/// Read the per-install UI preferences (UI-1) from satchel.json. The webview no
/// longer keeps these in localStorage — Satchel owns all persisted state.
#[tauri::command]
fn get_ui_prefs(state: tauri::State<AppState>) -> serde_json::Value {
    let cfg = state.config.lock().unwrap();
    serde_json::to_value(&cfg.ui).unwrap_or_else(|_| json!({}))
}

/// Write the per-install UI preferences (UI-1). Each field is optional so the
/// UI can patch just the one that changed (theme / language / nav_open).
#[tauri::command]
fn set_ui_prefs(
    state: tauri::State<AppState>,
    theme: Option<String>,
    language: Option<String>,
    nav_open: Option<bool>,
) -> Result<(), String> {
    let mut cfg = state.config.lock().unwrap();
    if let Some(theme) = theme {
        cfg.ui.theme = theme;
    }
    if let Some(language) = language {
        cfg.ui.language = language;
    }
    if let Some(nav_open) = nav_open {
        cfg.ui.nav_open = nav_open;
    }
    save_config(&state.config_dir, &cfg).map_err(|e| format!("{e:#}"))
}

/// The machine-level coin connections, for the coin-setup UI. Reading from
/// satchel.json (the source of truth) — `listcoins` over pactd adds the live
/// connection status on top of this.
#[tauri::command]
fn list_coin_config(state: tauri::State<AppState>) -> serde_json::Value {
    let cfg = state.config.lock().unwrap();
    json!({
        "coins": cfg.coins,
        "network": active_network(),
        "board_urls": cfg.board_urls,
        "nostr_relays": cfg.nostr_relays,
    })
}

/// The default Nostr relay set, prewired into a fresh `Config` (and used to
/// fill a satchel.json that omits the field) — docs/NOSTR_TRANSPORT.md. These
/// six were verified by `tools/relay-prober` to accept AND retain our offer
/// (31510) + gift-wrap (1059) kinds with no auth/payment/PoW; the user can edit
/// or clear them in Settings. Re-run the prober before a release to confirm
/// they're still healthy.
const RECOMMENDED_NOSTR_RELAYS: [&str; 6] = [
    "wss://relay.damus.io",
    "wss://nos.lol",
    "wss://relay.primal.net",
    "wss://nostr.mom",
    "wss://nostr-pub.wellorder.net",
    "wss://offchain.pub",
];

/// Save the Nostr relay URL(s) and relaunch the active merchant so pactd
/// picks them up (passed at launch, like the board). An empty string
/// disables the Nostr transport. Comma-separated for multiple relays.
#[tauri::command]
async fn save_nostr_relays(app: tauri::AppHandle, urls: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<()> {
        {
            let state = app.state::<AppState>();
            let mut cfg = state.config.lock().unwrap();
            cfg.nostr_relays = urls
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect();
            save_config(&state.config_dir, &cfg)?;
        }
        apply_config_change(&app)
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
    .map_err(|e| format!("{e:#}"))
}

/// Save the machine-level noticeboard URL(s) and relaunch the active merchant
/// so pactd picks them up (the board is passed at launch, like coins). An empty
/// string clears the board. Comma-separated for multiple boards.
#[tauri::command]
async fn save_board(app: tauri::AppHandle, urls: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<()> {
        {
            let state = app.state::<AppState>();
            let mut cfg = state.config.lock().unwrap();
            cfg.board_urls = urls
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect();
            save_config(&state.config_dir, &cfg)?;
        }
        apply_config_change(&app)
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
    .map_err(|e| format!("{e:#}"))
}

/// Save (upsert) a coin's chain-data backend + funding wallet, then relaunch
/// the managed pactd so the new backend goes live (pactd reloads its active
/// merchant from its own manifest). The caller should have validated the
/// backend first (the `validatecoin` RPC). In external/adopt mode the config is
/// persisted but pactd isn't ours to relaunch.
#[tauri::command]
async fn save_coin(
    app: tauri::AppHandle,
    coin_id: String,
    conn: CoinConnInput,
    confirmations: Option<u32>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<()> {
        {
            let state = app.state::<AppState>();
            let mut cfg = state.config.lock().unwrap();
            let new = conn.into_conn(coin_id.clone(), active_network(), confirmations)?;
            match cfg.coins.iter_mut().find(|c| c.coin_id == coin_id) {
                Some(existing) => *existing = new,
                None => cfg.coins.push(new),
            }
            save_config(&state.config_dir, &cfg)?;
        }
        // Apply: a configured backend only reaches pactd at launch.
        apply_config_change(&app)
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
    .map_err(|e| format!("{e:#}"))
}

/// Compose (preview) the backend URL string from a structured connection,
/// without saving. The setup form passes the result to the `validatecoin` RPC
/// so validation hits the exact URL that will be saved — and cookie reading
/// stays in Rust, keeping secrets out of the webview.
#[tauri::command]
fn compose_coin_url(coin_id: String, conn: CoinConnInput) -> Result<String, String> {
    let conn = conn
        .into_conn(coin_id, active_network(), None)
        .map_err(|e| format!("{e:#}"))?;
    Ok(conn.chain_data)
}

/// The coin templates (connection defaults + icon availability) for the current
/// network, for the setup picker. Joined with the engine's `listcoins` in the UI
/// to mark each coin supported/unsupported and configured/live.
#[tauri::command]
fn list_coin_templates(state: tauri::State<AppState>) -> serde_json::Value {
    let config_dir = state.config_dir.clone();
    coins_file::templates_json(&config_dir, active_network())
}

/// A coin's icon (from the file next to `coins.toml`) as a `data:` URL, or null
/// when the coin has no icon / it can't be read (the UI falls back to a bundled
/// glyph for the built-ins).
#[tauri::command]
fn get_coin_icon(state: tauri::State<AppState>, coin_id: String) -> Option<String> {
    let config_dir = state.config_dir.clone();
    coins_file::icon_data_url(&config_dir, &coin_id)
}

/// Remove a coin's connection and relaunch the managed pactd without it.
#[tauri::command]
async fn remove_coin(app: tauri::AppHandle, coin_id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<()> {
        {
            let state = app.state::<AppState>();
            let mut cfg = state.config.lock().unwrap();
            cfg.coins.retain(|c| c.coin_id != coin_id);
            save_config(&state.config_dir, &cfg)?;
        }
        apply_config_change(&app)
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
    .map_err(|e| format!("{e:#}"))
}

/// Apply a machine-level config edit (coins / board) by relaunching the managed
/// pactd so it re-reads the new backends. In external/adopt mode pactd isn't
/// ours to restart, so this is a no-op there (the config is still persisted and
/// would apply if Satchel later launches a managed pactd).
fn apply_config_change(app: &tauri::AppHandle) -> anyhow::Result<()> {
    let managed = app.state::<ManagedPactd>().0.lock().unwrap().is_some();
    if managed {
        relaunch_pactd(app)?;
    }
    Ok(())
}

/// RFC 4648 base64 for the Basic auth header.
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

/// Blocking JSON-RPC call to pactd (std-only). `auth` is `user:pass`.
fn pactd_call(
    url: &str,
    auth: &str,
    method: &str,
    params: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| anyhow::anyhow!("pactd url must be http://"))?;
    let (hostport, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let (host, port) = hostport
        .rsplit_once(':')
        .ok_or_else(|| anyhow::anyhow!("pactd url needs a port"))?;
    let body = json!({ "jsonrpc": "2.0", "id": "satchel", "method": method, "params": params })
        .to_string();
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\nAuthorization: Basic {}\r\n\
         Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        base64(auth.as_bytes()),
        body.len()
    );
    let mut stream = TcpStream::connect((host, port.parse::<u16>()?))?;
    stream.write_all(request.as_bytes())?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw)?;
    let text = String::from_utf8_lossy(&raw);
    let (head, http_body) = text
        .split_once("\r\n\r\n")
        .ok_or_else(|| anyhow::anyhow!("malformed response"))?;
    let http_body = if head
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        dechunk(http_body)?
    } else {
        http_body.to_string()
    };
    let parsed: serde_json::Value = serde_json::from_str(http_body.trim())?;
    if let Some(err) = parsed.get("error").filter(|e| !e.is_null()) {
        anyhow::bail!("{}", err["message"].as_str().unwrap_or("RPC error"));
    }
    Ok(parsed["result"].clone())
}

fn dechunk(body: &str) -> anyhow::Result<String> {
    let mut out = String::new();
    let mut rest = body;
    loop {
        let (size_line, after) = rest
            .split_once("\r\n")
            .ok_or_else(|| anyhow::anyhow!("bad chunk"))?;
        let size = usize::from_str_radix(size_line.trim(), 16)?;
        if size == 0 {
            return Ok(out);
        }
        let chunk = after
            .get(..size)
            .ok_or_else(|| anyhow::anyhow!("truncated chunk"))?;
        out.push_str(chunk);
        rest = after
            .get(size..)
            .and_then(|r| r.strip_prefix("\r\n"))
            .ok_or_else(|| anyhow::anyhow!("bad terminator"))?;
    }
}

/// The one bridge the UI uses for pactd JSON-RPC — including the C10 merchant
/// RPCs (createmerchant/listmerchants/loadmerchant/getmerchantinfo). Runs the
/// blocking call off the UI thread; fails clearly if pactd isn't up yet.
#[tauri::command]
async fn pactd_rpc(
    app: tauri::AppHandle,
    method: String,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let conn = app.state::<RpcState>().0.lock().unwrap().clone();
    if conn.auth.is_empty() {
        return Err("pactd is not running yet — its connection isn't ready".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        pactd_call(&conn.url, &conn.auth, &method, &params)
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
    .map_err(|e| format!("{e:#}"))
}

/// RC2: set auto-fund and persist the choice. Applies LIVE via the
/// `setautofund` RPC (no pactd relaunch); also persists to the Satchel config so
/// it survives a restart (re-applied via the `--auto-fund` launch flag). If
/// pactd isn't up yet, the persisted flag takes effect on the next launch.
#[tauri::command]
async fn set_auto_fund(app: tauri::AppHandle, on: bool) -> Result<(), String> {
    let conn = app.state::<RpcState>().0.lock().unwrap().clone();
    tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<()> {
        {
            let state = app.state::<AppState>();
            let mut cfg = state.config.lock().unwrap();
            cfg.auto_fund = on;
            save_config(&state.config_dir, &cfg)?;
        }
        if !conn.auth.is_empty() {
            pactd_call(&conn.url, &conn.auth, "setautofund", &json!([on]))?;
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
    .map_err(|e| format!("{e:#}"))
}

/// C6: the exit-gate's terminal action. ExitGate has already run the 4-state
/// matrix (and any offer `boardrevoke`s) and now tells Satchel how to leave:
///
/// - `keep_running == false` → STOP the managed pactd gracefully (today's `stop`
///   RPC + kill fallback) and exit. The clean-exit / withdraw-&-exit branches and
///   the typed-confirm force-quit all funnel here.
/// - `keep_running == true` → DETACH: do NOT stop pactd. Persist a
///   `running-pactd.json` hand-off so the next launch re-adopts the still-alive
///   daemon, set the detach flag so `RunEvent::Exit` skips the graceful stop, then
///   exit the GUI leaving pactd watching its timelocks headless.
///
/// In external/adopt mode (`ManagedPactd` is None) pactd is never ours to stop, so
/// `keep_running` is moot — we just exit and leave it running, which is always
/// safe. `withdraw` is informational here (ExitGate performs the revokes, since it
/// holds the offer + identity context); it is logged so the intent is auditable.
#[tauri::command]
async fn quit_app(app: tauri::AppHandle, keep_running: bool, withdraw: bool) -> Result<(), String> {
    let is_managed = app.state::<ManagedPactd>().0.lock().unwrap().is_some();

    if is_managed && keep_running {
        // Detach: record the coordinates needed to re-adopt, then leave pactd up.
        let state = app.state::<AppState>();
        let (config_dir, listen, data_dir) = {
            let cfg = state.config.lock().unwrap();
            (
                state.config_dir.clone(),
                cfg.listen.clone(),
                pactd_data_dir(&state.config_dir),
            )
        };
        // pid is best-effort (used for logging/cleanup only); 0 if the child is
        // somehow gone — adoption keys on the health+cookie probe, not the pid.
        let pid = app
            .state::<ManagedPactd>()
            .0
            .lock()
            .unwrap()
            .as_ref()
            .map(|c| c.id())
            .unwrap_or(0);
        let rec = RunningPactd {
            listen,
            data_dir: data_dir.to_string_lossy().into_owned(),
            pid,
        };
        write_running_pactd(&config_dir, &rec).map_err(|e| format!("{e:#}"))?;
        // Skip the graceful stop on Exit, and drop our child handle so it is NOT
        // killed when the process tears down (it stays a live, re-adoptable pactd).
        *app.state::<DetachFlag>().0.lock().unwrap() = true;
        let _ = app.state::<ManagedPactd>().0.lock().unwrap().take();
        eprintln!(
            "satchel: detaching managed pactd (pid {pid}) — left running headless (withdraw={withdraw})"
        );
    } else if is_managed {
        // Stop & exit: a stopped daemon can't honor takes — only the no-live-swap
        // branches reach here (or a typed-confirm force-quit accepting the risk).
        stop_managed(&app);
        clear_running_pactd(&app.state::<AppState>().config_dir);
        eprintln!("satchel: stopped managed pactd on quit (withdraw={withdraw})");
    } else {
        // External/adopt mode: not ours to stop. Always safe to just leave.
        eprintln!("satchel: external/adopt pactd left running on quit");
    }

    app.exit(0);
    Ok(())
}

/// Open a URL in the user's default browser (for the update dialog's "release
/// page" link). Tauri's webview blocks external navigation, so the UI hands the
/// URL here. http(s) only.
#[tauri::command]
fn open_external(url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("only http(s) URLs may be opened".into());
    }
    #[cfg(windows)]
    let spawned = Command::new("cmd").args(["/C", "start", "", &url]).spawn();
    #[cfg(target_os = "macos")]
    let spawned = Command::new("open").arg(&url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let spawned = Command::new("xdg-open").arg(&url).spawn();
    spawned
        .map(|_| ())
        .map_err(|e| format!("failed to open browser: {e}"))
}

/// In-app update checking against GitHub releases — the Phoenix pattern (cloned
/// from phoenix-pocx `src-tauri/src/update.rs`). `check_app_update` hits the
/// canonical Satchel repo (`PoC-Consortium/satchel`); the UI polls it on
/// startup and every 6h and shows a left-menu badge when a newer release
/// exists. The `SemVer`/`is_newer_version` comparison (incl. pre-release rc
/// ordering) is lifted verbatim so the two wallets behave identically.
mod update {
    use serde::{Deserialize, Serialize};

    const RELEASES_LATEST: &str =
        "https://api.github.com/repos/PoC-Consortium/satchel/releases/latest";

    /// Update info returned to the UI (camelCase to match the TS interface).
    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdateInfo {
        pub available: bool,
        pub current_version: String,
        pub latest_version: Option<String>,
        pub release_url: Option<String>,
        pub release_notes: Option<String>,
        pub published_at: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    struct GitHubRelease {
        tag_name: String,
        html_url: String,
        body: Option<String>,
        published_at: Option<String>,
    }

    /// Current app version (= the satchel crate version).
    #[tauri::command]
    pub fn get_app_version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    /// Fetch the latest GitHub release and report whether it's newer. Any
    /// failure (no releases yet, offline, rate-limited) is an `Err` the UI
    /// treats as "no update" — never fatal.
    #[tauri::command]
    pub async fn check_app_update() -> Result<UpdateInfo, String> {
        let current = env!("CARGO_PKG_VERSION");
        let client = reqwest::Client::builder()
            .user_agent("Satchel-PoCX-Wallet")
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {e}"))?;
        let response = client
            .get(RELEASES_LATEST)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch release info: {e}"))?;
        if !response.status().is_success() {
            return Err(format!("GitHub API returned status: {}", response.status()));
        }
        let release: GitHubRelease = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse release info: {e}"))?;
        let latest_version = release.tag_name.trim_start_matches('v').to_string();
        let available = is_newer_version(&latest_version, current);
        Ok(UpdateInfo {
            available,
            current_version: current.to_string(),
            latest_version: Some(latest_version),
            release_url: Some(release.html_url),
            release_notes: release.body,
            published_at: release.published_at,
        })
    }

    /// Parsed semantic version with optional pre-release tag.
    #[derive(Debug, Clone)]
    struct SemVer {
        major: u32,
        minor: u32,
        patch: u32,
        prerelease: Option<String>,
    }

    impl SemVer {
        fn parse(version: &str) -> Option<Self> {
            let (version_part, prerelease) = match version.split_once('-') {
                Some((v, pre)) => (v, Some(pre.to_string())),
                None => (version, None),
            };
            let parts: Vec<&str> = version_part.split('.').collect();
            if parts.len() < 2 {
                return None;
            }
            Some(SemVer {
                major: parts.first()?.parse().ok()?,
                minor: parts.get(1)?.parse().ok()?,
                patch: parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0),
                prerelease,
            })
        }

        /// None (final release) > Some (pre-release).
        fn compare_prerelease(a: &Option<String>, b: &Option<String>) -> std::cmp::Ordering {
            use std::cmp::Ordering;
            match (a, b) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(a), Some(b)) => {
                    let extract_num = |s: &str| -> Option<u32> {
                        s.chars()
                            .rev()
                            .take_while(|c| c.is_ascii_digit())
                            .collect::<String>()
                            .chars()
                            .rev()
                            .collect::<String>()
                            .parse()
                            .ok()
                    };
                    let a_prefix = a.trim_end_matches(|c: char| c.is_ascii_digit());
                    let b_prefix = b.trim_end_matches(|c: char| c.is_ascii_digit());
                    if a_prefix == b_prefix {
                        extract_num(a)
                            .unwrap_or(0)
                            .cmp(&extract_num(b).unwrap_or(0))
                    } else {
                        a.cmp(b)
                    }
                }
            }
        }
    }

    /// True if `latest` is newer than `current`.
    fn is_newer_version(latest: &str, current: &str) -> bool {
        let latest_ver = match SemVer::parse(latest) {
            Some(v) => v,
            None => return false,
        };
        let current_ver = match SemVer::parse(current) {
            Some(v) => v,
            None => return true,
        };
        if latest_ver.major != current_ver.major {
            return latest_ver.major > current_ver.major;
        }
        if latest_ver.minor != current_ver.minor {
            return latest_ver.minor > current_ver.minor;
        }
        if latest_ver.patch != current_ver.patch {
            return latest_ver.patch > current_ver.patch;
        }
        SemVer::compare_prerelease(&latest_ver.prerelease, &current_ver.prerelease)
            == std::cmp::Ordering::Greater
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_version_comparison() {
            assert!(is_newer_version("2.0.1", "2.0.0"));
            assert!(is_newer_version("2.1.0", "2.0.0"));
            assert!(is_newer_version("3.0.0", "2.9.9"));
            assert!(!is_newer_version("2.0.0", "2.0.0"));
            assert!(!is_newer_version("1.9.9", "2.0.0"));
            assert!(is_newer_version("2.0.0", "2.0.0-rc7"));
            assert!(is_newer_version("2.0.0-rc8", "2.0.0-rc7"));
            assert!(is_newer_version("2.0.0-rc10", "2.0.0-rc9"));
            assert!(!is_newer_version("2.0.0-rc7", "2.0.0"));
            assert!(!is_newer_version("2.0.0-rc7", "2.0.0-rc8"));
            assert!(is_newer_version("2.0.1-rc1", "2.0.0"));
            assert!(!is_newer_version("2.0.0-rc1", "2.0.1"));
        }
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            pactd_rpc,
            set_auto_fund,
            quit_app,
            get_ui_prefs,
            set_ui_prefs,
            list_coin_config,
            list_coin_templates,
            compose_coin_url,
            get_coin_icon,
            save_coin,
            remove_coin,
            save_board,
            save_nostr_relays,
            open_external,
            update::get_app_version,
            update::check_app_update
        ])
        .setup(|app| {
            // Mode 1 — external: attach to a pactd someone else runs.
            //   SATCHEL_PACTD_URL=http://host:port  +  SATCHEL_PACTD_DATADIR=dir
            // (datadir is where we read .cookie; or SATCHEL_PACTD_COOKIE=user:pass)
            if let Ok(url) = std::env::var("SATCHEL_PACTD_URL") {
                let auth = resolve_external_auth()?;
                let config_dir = net_config_dir(&app.path().app_config_dir()?);
                app.manage(AppState {
                    config: Mutex::new(load_or_create_config(&config_dir)?),
                    config_dir,
                });
                app.manage(RpcState(Mutex::new(RpcConn {
                    url: url
                        .trim_end_matches("/ui")
                        .trim_end_matches('/')
                        .to_string(),
                    auth,
                })));
                app.manage(ManagedPactd(Mutex::new(None)));
                app.manage(DetachFlag(Mutex::new(false)));
                return Ok(());
            }

            // Modes 2/3 — managed or adopt. A single pactd runs at the parent
            // data dir and owns the merchant registry (C10); node connections
            // are machine-level. pactd boots seedless on a fresh install and
            // the UI's wizard calls createmerchant + createseed/importseed.
            let config_dir = net_config_dir(&app.path().app_config_dir()?);
            let config = load_or_create_config(&config_dir)?;
            let listen = config.listen.clone();
            let data_dir = pactd_data_dir(&config_dir);

            app.manage(AppState {
                config: Mutex::new(config.clone()),
                config_dir: config_dir.clone(),
            });
            // Start empty; filled in once pactd is up and we've read its cookie.
            app.manage(RpcState(Mutex::new(RpcConn {
                url: format!("http://{listen}"),
                auth: String::new(),
            })));
            app.manage(ManagedPactd(Mutex::new(None)));
            app.manage(DetachFlag(Mutex::new(false)));

            // C6 re-adopt: a previous session may have detached its managed pactd
            // (left it running headless to watch timelocks) and recorded it in
            // `running-pactd.json`. If that daemon is still alive at its listen and
            // its cookie still authenticates, ADOPT it — reuse it like external
            // mode — instead of spawning a second pactd on the same data dir. A
            // stale/dead record is cleaned and we fall through to the normal path.
            // (Guard: this runs BEFORE any spawn, and each branch is mutually
            // exclusive, so pactd is never launched twice.)
            if let Some(rec) = read_running_pactd(&config_dir) {
                if rec.listen == listen {
                    if let Some(cookie) = probe_adoptable(&listen, &data_dir) {
                        // Adopt the detached daemon. Leave ManagedPactd None: we
                        // did not spawn it this session, so it is treated like an
                        // adopted/external pactd until the user explicitly stops it.
                        *app.state::<RpcState>().0.lock().unwrap() = RpcConn {
                            url: format!("http://{listen}"),
                            auth: cookie,
                        };
                        return Ok(());
                    }
                }
                // Dead, stale, or different listen — drop the hand-off file.
                clear_running_pactd(&config_dir);
            }

            if health_ok(&listen) {
                // Adopt: a pactd is already listening (not ours to kill).
                if let Ok(cookie) = std::fs::read_to_string(data_dir.join(".cookie")) {
                    *app.state::<RpcState>().0.lock().unwrap() = RpcConn {
                        url: format!("http://{listen}"),
                        auth: cookie.trim().to_string(),
                    };
                }
            } else {
                // Managed: launch the one pactd. It reloads its active merchant
                // (if any) from its own manifest, or comes up merchant-less for
                // the first-run wizard.
                std::fs::create_dir_all(&data_dir)?;
                let child = spawn_pactd(&config, &data_dir)?;
                wait_health(&listen, 30)?;
                let cookie = std::fs::read_to_string(data_dir.join(".cookie"))?
                    .trim()
                    .to_string();
                *app.state::<RpcState>().0.lock().unwrap() = RpcConn {
                    url: format!("http://{listen}"),
                    auth: cookie,
                };
                *app.state::<ManagedPactd>().0.lock().unwrap() = Some(child);
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Satchel")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                // C6: skip the graceful stop when we deliberately detached the
                // managed pactd (`quit_app { keep_running: true }`) — it must stay
                // alive headless to keep watching timelocks. Otherwise stop it (the
                // window was closed/quit without keep-running, or no detach path
                // ran). External/adopt mode: ManagedPactd is None, so stop is a
                // no-op there regardless. DetachFlag is managed in every setup
                // branch, so `state` never panics here.
                let detached = *app.state::<DetachFlag>().0.lock().unwrap();
                if !detached {
                    stop_managed(app);
                }
            }
        });
}

/// External-mode auth: SATCHEL_PACTD_COOKIE (user:pass) or read .cookie
/// from SATCHEL_PACTD_DATADIR.
fn resolve_external_auth() -> anyhow::Result<String> {
    if let Ok(c) = std::env::var("SATCHEL_PACTD_COOKIE") {
        return Ok(c);
    }
    let dir = std::env::var("SATCHEL_PACTD_DATADIR").map_err(|_| {
        anyhow::anyhow!("external mode needs SATCHEL_PACTD_COOKIE or SATCHEL_PACTD_DATADIR")
    })?;
    Ok(std::fs::read_to_string(Path::new(&dir).join(".cookie"))?
        .trim()
        .to_string())
}

#[cfg(test)]
mod tests {
    // Tests build a Config::default() then tweak individual fields — clearer than
    // a full struct literal for "default except X"; clippy's field-reassign lint
    // doesn't add value here.
    #![allow(clippy::field_reassign_with_default)]
    use super::*;

    /// A structured coin connection for tests (userpass auth, no recompose
    /// needed since `into_conn` composes from these fields).
    fn test_conn(coin_id: &str, port: u16) -> CoinConn {
        CoinConnInput {
            rpc_host: Some("127.0.0.1".into()),
            rpc_port: Some(port),
            auth_method: Some("userpass".into()),
            rpc_user: Some("u".into()),
            rpc_password: Some("p".into()),
            wallet: Some(coin_id.into()),
            ..Default::default()
        }
        .into_conn(coin_id.into(), "regtest", None)
        .unwrap()
    }

    #[test]
    fn coin_config_round_trips_through_satchel_json() {
        // The per-coin connection config must survive a save/load cycle
        // unchanged — it is the source of truth Satchel passes to pactd.
        let mut cfg = Config::default();
        cfg.coins.push(test_conn("btcx", 19443));
        cfg.coins.push(test_conn("btc", 19543));

        let json = serde_json::to_string_pretty(&cfg).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.coins.len(), 2);
        assert_eq!(back.coins[0].coin_id, "btcx");
        // Structured fields + composed chain_data both round-trip.
        assert_eq!(back.coins[0].auth_method.as_deref(), Some("userpass"));
        assert_eq!(
            back.coins[0].chain_data,
            "http://u:p@127.0.0.1:19443/wallet/btcx"
        );
        assert_eq!(back.coins[1].coin_id, "btc");
        assert_eq!(back.coins[1].funding_wallet, "core-rpc");
    }

    #[test]
    fn legacy_chain_data_only_config_still_loads() {
        // A pre-v2 satchel.json has only `chain_data` (no structured fields).
        // It must deserialize cleanly with the new fields defaulting to None,
        // so an upgrade never bricks an existing install.
        let json = r#"{
            "coins": [
                { "coin_id": "btcx", "chain_data": "http://u:p@127.0.0.1:19443/wallet/pocx" }
            ]
        }"#;
        let cfg: Config = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.coins.len(), 1);
        let c = &cfg.coins[0];
        assert_eq!(c.chain_data, "http://u:p@127.0.0.1:19443/wallet/pocx");
        assert_eq!(c.funding_wallet, "core-rpc"); // serde default applied
        assert!(c.auth_method.is_none());
        // A legacy entry is used verbatim at launch (no recomposition).
        assert_eq!(compose::effective_chain_data(c, "regtest"), c.chain_data);
    }

    #[test]
    fn ui_prefs_round_trip_and_defaults() {
        // UI-1: theme/language/nav_open persist in satchel.json (not the
        // webview's localStorage). Defaults are sane on a fresh install.
        let fresh = Config::default();
        assert_eq!(fresh.ui.theme, "system");
        assert_eq!(fresh.ui.language, "en");
        assert!(fresh.ui.nav_open);

        let mut cfg = Config::default();
        cfg.ui.theme = "dark".into();
        cfg.ui.language = "en".into();
        cfg.ui.nav_open = false;
        let json = serde_json::to_string_pretty(&cfg).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.ui.theme, "dark");
        assert!(!back.ui.nav_open);

        // A satchel.json with no `ui` block loads the defaults (forward-compat).
        let legacy = r#"{ "network": "regtest" }"#;
        let cfg: Config = serde_json::from_str(legacy).unwrap();
        assert_eq!(cfg.ui.theme, "system");
        assert!(cfg.ui.nav_open);
    }

    #[test]
    fn legacy_merchant_keys_are_ignored_not_fatal() {
        // C10: the old Satchel-side registry (`merchants[]`/`active_merchant`)
        // moved into pactd. An old satchel.json that still carries those keys
        // must load cleanly — they're simply dropped (serde ignores unknown
        // fields), leaving Satchel stateless about merchants.
        let legacy = r#"{
            "network": "regtest",
            "merchants": [ { "id": "m1", "label": "Greedy", "identity": "ab12" } ],
            "active_merchant": "m1"
        }"#;
        let cfg: Config = serde_json::from_str(legacy).unwrap();
        // Defaults intact; nothing merchant-shaped (nor the removed `network`)
        // survived into the struct.
        assert_eq!(cfg.ui.theme, "system");
    }

    #[test]
    fn board_urls_round_trip_and_default_empty() {
        // Phase D: the noticeboard URL list is machine-level config Satchel
        // passes to pactd at launch; it must survive a save/load cycle and
        // default to empty (no board) on a fresh / board-less install.
        let fresh = Config::default();
        assert!(
            fresh.board_urls.is_empty(),
            "no board configured by default"
        );

        let mut cfg = Config::default();
        cfg.board_urls = vec![
            "http://board.one:8080".into(),
            "http://board.two:8080".into(),
        ];
        let json = serde_json::to_string_pretty(&cfg).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.board_urls, cfg.board_urls);
    }

    #[test]
    fn running_pactd_hand_off_round_trips_and_clears() {
        // C6: the detach hand-off file (`running-pactd.json`, separate from
        // satchel.json) must survive a write/read cycle so the next launch can
        // re-adopt the still-running pactd, and clear cleanly when it's stale.
        let dir = std::env::temp_dir().join(format!("satchel-c6-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        assert!(
            read_running_pactd(&dir).is_none(),
            "no hand-off file initially"
        );

        let rec = RunningPactd {
            listen: "127.0.0.1:9737".into(),
            data_dir: dir.join("pactd").to_string_lossy().into_owned(),
            pid: 4242,
        };
        write_running_pactd(&dir, &rec).unwrap();

        let back = read_running_pactd(&dir).expect("hand-off file reads back");
        assert_eq!(back.listen, "127.0.0.1:9737");
        assert_eq!(back.pid, 4242);
        assert_eq!(back.data_dir, rec.data_dir);

        clear_running_pactd(&dir);
        assert!(read_running_pactd(&dir).is_none(), "hand-off file cleared");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn funding_wallet_defaults_when_absent() {
        // Older/hand-edited entries without `funding_wallet` load as core-rpc.
        let json = r#"{ "coins": [ { "coin_id": "btcx", "chain_data": "http://x:1" } ] }"#;
        let cfg: Config = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.coins.len(), 1);
        assert_eq!(cfg.coins[0].funding_wallet, "core-rpc");
        // Unknown legacy fields (the old pocx_rpc, the removed network) are
        // ignored, not fatal.
        let legacy = r#"{ "pocx_rpc": "http://old:1", "network": "testnet" }"#;
        let cfg: Config = serde_json::from_str(legacy).unwrap();
        assert!(cfg.coins.is_empty());
    }
}
