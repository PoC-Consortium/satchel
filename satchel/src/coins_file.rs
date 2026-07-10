//! Coin templates (`coins.toml`) — Satchel's reader.
//!
//! One `coins.toml` lives next to the executables and is read by BOTH pactd
//! (consensus params, to know what's tradable — see `libswap::coins_file`) and
//! Satchel (this module: per-coin **connection defaults** that pre-fill the
//! setup form, plus the coin **icon**). Keeping it one file means adding a new
//! coin — node connection defaults, icon, and engine consensus — is a single
//! edit with no recompile.
//!
//! Resolution order for the file (#160): first the sibling of the Satchel
//! executable (`<exe dir>/coins.toml` — the Windows/NSIS bundle layout), else
//! the Tauri **resource dir** (where `bundle.resources` land on Linux
//! .deb/AppImage — the exe is `usr/bin/satchel` but resources go to
//! `usr/lib/<app>/` — and in macOS `Resources/`), else `<config dir>/
//! coins.toml` (a user-editable copy). If none exists, the baked-in
//! [`DEFAULT_COINS_TOML`] is written to the config dir so the user can edit it.
//! A parse error logs and falls back to the baked-in default, never crashing
//! boot. The shipped copy (exe-sibling or resource dir) deliberately outranks
//! the config-dir copy so an upgrade's new defaults take effect; the config
//! copy is only authoritative where no shipped copy is found.

use serde::Deserialize;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The default templates shipped with Satchel (btcx + btc): connection presets
/// plus icon, and the engine consensus params (the latter dropped by pactd as
/// built-in collisions, but kept here as a worked example for adding a coin).
pub const DEFAULT_COINS_TOML: &str = include_str!("../coins.toml");

/// Parsed `coins.toml` from Satchel's point of view. Only the fields Satchel
/// needs are declared; the engine's `consensus` sub-tables are ignored by serde.
#[derive(Debug, Deserialize, Default)]
struct CoinFileDoc {
    #[serde(default, rename = "coin")]
    coins: Vec<CoinDoc>,
}

#[derive(Debug, Deserialize)]
struct CoinDoc {
    coin_id: String,
    display_name: String,
    symbol: String,
    #[serde(default)]
    decimals: Option<u8>,
    #[serde(default)]
    icon: Option<String>,
    #[serde(default)]
    mainnet: Option<NetDoc>,
    #[serde(default)]
    testnet: Option<NetDoc>,
    #[serde(default)]
    regtest: Option<NetDoc>,
}

#[derive(Debug, Deserialize)]
struct NetDoc {
    #[serde(default)]
    connection: Option<ConnDoc>,
}

/// Per-(coin, network) connection defaults that pre-fill the setup form.
#[derive(Debug, Deserialize, Clone, Default)]
struct ConnDoc {
    #[serde(default)]
    rpc_host: Option<String>,
    #[serde(default)]
    rpc_port: Option<u16>,
    #[serde(default)]
    auth_method: Option<String>,
    #[serde(default)]
    datadir: Option<String>,
    #[serde(default)]
    cookie_subpath: Option<String>,
    #[serde(default)]
    wallet: Option<String>,
    /// Default Electrum servers for the NODELESS mode (epic #58) — pre-fill
    /// the setup form's server list so the no-node path is one click.
    #[serde(default)]
    electrum: Vec<String>,
}

impl CoinDoc {
    fn net(&self, network: &str) -> Option<&NetDoc> {
        match network {
            "mainnet" => self.mainnet.as_ref(),
            "testnet" => self.testnet.as_ref(),
            "regtest" => self.regtest.as_ref(),
            _ => None,
        }
    }
}

/// Where Tauri unpacked the bundled resources (`coins.toml`, the coin icons).
/// Recorded once at app setup from `tauri::Manager::path().resource_dir()`;
/// unset in non-Tauri contexts (unit tests) or if the resolver fails, in which
/// case resolution simply skips this step.
static RESOURCE_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Record the Tauri resource dir for coins-file resolution (#160). Called once
/// during app setup, before anything reads templates or spawns pactd; a second
/// call is ignored (first write wins).
pub fn set_resource_dir(dir: PathBuf) {
    let _ = RESOURCE_DIR.set(dir);
}

/// Resolve the coins file path, materializing the baked-in default into the
/// config dir if no file exists anywhere. Always returns a path that exists.
pub fn resolve_coins_file(config_dir: &Path) -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    resolve_coins_file_in(
        exe_dir.as_deref(),
        RESOURCE_DIR.get().map(PathBuf::as_path),
        config_dir,
    )
}

/// Testable core of [`resolve_coins_file`]: probe each candidate dir in order.
///
/// 1. `<exe dir>/coins.toml` — the Windows (NSIS) bundle layout, where
///    resources sit next to the executable. Kept first so Windows behavior is
///    unchanged (there the resource dir IS the exe dir anyway).
/// 2. `<resource dir>/coins.toml` — where Tauri puts `bundle.resources` on
///    Linux (.deb: `/usr/lib/<app>/`; AppImage: `<mount>/usr/lib/<app>/`) and
///    macOS (`Contents/Resources/`). Without this step the Linux lookup fell
///    through to a config-dir copy materialized by the FIRST ever install and
///    never updated — shadowing every upgrade's new defaults (#160).
/// 3. `<config dir>/coins.toml` — the user-editable copy, written from the
///    baked-in default when missing so there is always a file to edit and the
///    templates are never empty even if both shipped copies miss.
fn resolve_coins_file_in(
    exe_dir: Option<&Path>,
    resource_dir: Option<&Path>,
    config_dir: &Path,
) -> PathBuf {
    for dir in [exe_dir, resource_dir].into_iter().flatten() {
        let shipped = dir.join("coins.toml");
        if shipped.exists() {
            return shipped;
        }
    }
    let user_copy = config_dir.join("coins.toml");
    if !user_copy.exists() {
        // Best-effort: write the default so the user has something to edit. If
        // the write fails we still hand back the path; callers tolerate a
        // missing file by falling back to DEFAULT_COINS_TOML.
        let _ = std::fs::create_dir_all(config_dir);
        let _ = std::fs::write(&user_copy, DEFAULT_COINS_TOML);
    }
    user_copy
}

fn load_doc(config_dir: &Path) -> CoinFileDoc {
    let path = resolve_coins_file(config_dir);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|_| DEFAULT_COINS_TOML.to_string());
    match toml::from_str::<CoinFileDoc>(&text) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("satchel: coins.toml parse error ({err}); using built-in defaults");
            toml::from_str(DEFAULT_COINS_TOML).expect("baked-in coins.toml is valid")
        }
    }
}

/// Expand the OS tokens Satchel understands in a `datadir` value:
/// `~` → home dir, `%LOCALAPPDATA%` → local app data, `%APPDATA%` → config dir.
/// Anything else is left literal.
pub fn expand_datadir(raw: &str) -> String {
    let raw = raw.trim();
    // %NODEDIR%/<Name> → the Bitcoin-Core-family default data dir for a node
    // named <Name>, resolved per OS (this is where the node writes its .cookie):
    //   Windows → %LOCALAPPDATA%\<Name>   (modern Core + bitcoin-pocx)
    //   macOS   → ~/Library/Application Support/<Name>
    //   Linux   → ~/.<name lowercased>
    // Windows + macOS are both dirs::data_local_dir(); only Linux is the dotfile.
    // Mirrors phoenix-pocx's getDefaultDataDirectory.
    if let Some(rest) = raw.strip_prefix("%NODEDIR%") {
        let name = rest.trim().trim_start_matches(['/', '\\']);
        if cfg!(target_os = "linux") {
            if let Some(home) = dirs::home_dir() {
                return join_tail(&home, &format!(".{}", name.to_lowercase()));
            }
        } else if let Some(base) = dirs::data_local_dir() {
            return join_tail(&base, name);
        }
    }
    if let Some(rest) = raw.strip_prefix("~") {
        if let Some(home) = dirs::home_dir() {
            return join_tail(&home, rest);
        }
    }
    if let Some(rest) = raw.strip_prefix("%LOCALAPPDATA%") {
        if let Some(base) = dirs::data_local_dir() {
            return join_tail(&base, rest);
        }
    }
    if let Some(rest) = raw.strip_prefix("%APPDATA%") {
        if let Some(base) = dirs::config_dir() {
            return join_tail(&base, rest);
        }
    }
    raw.to_string()
}

fn join_tail(base: &Path, tail: &str) -> String {
    let tail = tail.trim_start_matches(['/', '\\']);
    if tail.is_empty() {
        base.display().to_string()
    } else {
        base.join(tail).display().to_string()
    }
}

/// The default cookie sub-path for a network when a template omits it
/// (bitcoind layout). Templates normally set this explicitly — the Bitcoin
/// (`btc`) template carries "testnet3/.cookie", so this fallback follows the
/// Bitcoin PoCX convention ("testnet") a PoCX-family file-coin expects.
pub fn default_cookie_subpath(network: &str) -> &'static str {
    match network {
        "testnet" => "testnet/.cookie",
        "regtest" => "regtest/.cookie",
        _ => ".cookie",
    }
}

/// The coin templates for the current network, as JSON for the picker. Each
/// entry carries the connection defaults (datadir already OS-expanded) plus
/// whether an icon is available.
pub fn templates_json(config_dir: &Path, network: &str) -> serde_json::Value {
    let doc = load_doc(config_dir);
    let coins: Vec<serde_json::Value> = doc
        .coins
        .iter()
        .map(|c| {
            let conn = c
                .net(network)
                .and_then(|n| n.connection.clone())
                .unwrap_or_default();
            json!({
                "coin_id": c.coin_id,
                "display_name": c.display_name,
                "symbol": c.symbol,
                "decimals": c.decimals.unwrap_or(8),
                "has_icon": c.icon.is_some(),
                "defaults": {
                    "rpc_host": conn.rpc_host.unwrap_or_else(|| "127.0.0.1".into()),
                    "rpc_port": conn.rpc_port,
                    "auth_method": conn.auth_method.unwrap_or_else(|| "cookie".into()),
                    "datadir": conn.datadir.as_deref().map(expand_datadir).unwrap_or_default(),
                    "cookie_subpath": conn
                        .cookie_subpath
                        .unwrap_or_else(|| default_cookie_subpath(network).to_string()),
                    "wallet": conn.wallet.unwrap_or_default(),
                    "electrum": conn.electrum,
                },
            })
        })
        .collect();
    json!({ "network": network, "coins": coins })
}

/// The default Electrum server list shipped in `coins.toml` for `coin_id` on
/// `network` — empty when the coin declares none (e.g. `btcx` has no public
/// PoCX Electrum servers yet). Drives the coin-setup "reset to defaults" action
/// and the "new default servers available" reconcile prompt.
pub fn default_electrum(config_dir: &Path, coin_id: &str, network: &str) -> Vec<String> {
    let doc = load_doc(config_dir);
    doc.coins
        .iter()
        .find(|c| c.coin_id == coin_id)
        .and_then(|c| c.net(network))
        .and_then(|n| n.connection.as_ref())
        .map(|conn| conn.electrum.clone())
        .unwrap_or_default()
}

/// The icon for a coin as a `data:` URL (read from the file next to
/// `coins.toml`), or `None` if the coin has no icon or it can't be read.
pub fn icon_data_url(config_dir: &Path, coin_id: &str) -> Option<String> {
    let doc = load_doc(config_dir);
    let icon = doc
        .coins
        .iter()
        .find(|c| c.coin_id == coin_id)
        .and_then(|c| c.icon.clone())?;
    let base = resolve_coins_file(config_dir);
    let icon_path = base.with_file_name(&icon);
    let bytes = std::fs::read(&icon_path).ok()?;
    let mime = match icon_path.extension().and_then(|e| e.to_str()) {
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    };
    Some(format!("data:{mime};base64,{}", crate::base64(&bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Self-cleaning unique temp dir (no tempfile dep for one test).
    struct TempDir(PathBuf);
    impl TempDir {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "satchel-coins-test-{tag}-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// #160 — the full resolution chain: exe-sibling → resource dir →
    /// config-dir copy (materialized from the baked-in default when missing).
    #[test]
    fn resolution_chain_prefers_shipped_then_materializes_default() {
        let exe = TempDir::new("exe");
        let res = TempDir::new("res");
        let cfg = TempDir::new("cfg");

        // Nothing anywhere → the baked-in default is materialized into the
        // config dir, and it parses with working (non-empty) Electrum fleets,
        // so the wizard is never empty even if every shipped lookup misses.
        let got = resolve_coins_file_in(Some(exe.path()), Some(res.path()), cfg.path());
        assert_eq!(got, cfg.path().join("coins.toml"));
        assert!(got.exists());
        let doc: CoinFileDoc = toml::from_str(&std::fs::read_to_string(&got).unwrap()).unwrap();
        let btc = doc.coins.iter().find(|c| c.coin_id == "btc").unwrap();
        let conn = btc.mainnet.as_ref().unwrap().connection.as_ref().unwrap();
        assert!(
            !conn.electrum.is_empty(),
            "baked-in default must carry Electrum fleets"
        );

        // A shipped copy in the resource dir (the Linux .deb/AppImage layout)
        // outranks the (now stale) config-dir copy — the #160 fix.
        std::fs::write(res.path().join("coins.toml"), "# resource copy").unwrap();
        let got = resolve_coins_file_in(Some(exe.path()), Some(res.path()), cfg.path());
        assert_eq!(got, res.path().join("coins.toml"));

        // The exe-sibling (Windows layout) outranks both — unchanged behavior.
        std::fs::write(exe.path().join("coins.toml"), "# exe copy").unwrap();
        let got = resolve_coins_file_in(Some(exe.path()), Some(res.path()), cfg.path());
        assert_eq!(got, exe.path().join("coins.toml"));

        // No resource dir recorded (unit tests / resolver failure) → the chain
        // still works, skipping that step.
        let got = resolve_coins_file_in(None, None, cfg.path());
        assert_eq!(got, cfg.path().join("coins.toml"));
    }

    /// An existing config-dir copy is used as-is (user-editable), never
    /// overwritten by the materialization step.
    #[test]
    fn config_dir_copy_is_not_clobbered() {
        let cfg = TempDir::new("cfg-keep");
        let user_copy = cfg.path().join("coins.toml");
        std::fs::write(&user_copy, "# user edit").unwrap();
        let got = resolve_coins_file_in(None, None, cfg.path());
        assert_eq!(got, user_copy);
        assert_eq!(std::fs::read_to_string(&user_copy).unwrap(), "# user edit");
    }

    #[test]
    fn default_toml_parses_and_has_both_coins() {
        let doc: CoinFileDoc = toml::from_str(DEFAULT_COINS_TOML).unwrap();
        let ids: Vec<_> = doc.coins.iter().map(|c| c.coin_id.as_str()).collect();
        assert!(ids.contains(&"btcx") && ids.contains(&"btc"));
        // Regtest connection defaults match the playground node ports.
        let btcx = doc.coins.iter().find(|c| c.coin_id == "btcx").unwrap();
        let rt = btcx.regtest.as_ref().unwrap().connection.as_ref().unwrap();
        assert_eq!(rt.rpc_port, Some(19443));
    }

    #[test]
    fn expands_home_token() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(
            expand_datadir("~/.bitcoin"),
            home.join(".bitcoin").display().to_string()
        );
        assert_eq!(expand_datadir("/abs/path"), "/abs/path");
    }

    #[test]
    fn nodedir_token_resolves_per_os() {
        // %NODEDIR%/<Name> → the node's default data dir for this OS: the
        // dotted-lowercase home dir on Linux, the platform data-local dir
        // (%LOCALAPPDATA% / ~/Library/Application Support) elsewhere.
        let got = expand_datadir("%NODEDIR%/Bitcoin-PoCX");
        let want = if cfg!(target_os = "linux") {
            dirs::home_dir().unwrap().join(".bitcoin-pocx")
        } else {
            dirs::data_local_dir().unwrap().join("Bitcoin-PoCX")
        };
        assert_eq!(got, want.display().to_string());
    }
}
