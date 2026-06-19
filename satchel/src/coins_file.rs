//! Coin templates (`coins.toml`) — Satchel's reader.
//!
//! One `coins.toml` lives next to the executables and is read by BOTH pactd
//! (consensus params, to know what's tradable — see `libswap::coins_file`) and
//! Satchel (this module: per-coin **connection defaults** that pre-fill the
//! setup form, plus the coin **icon**). Keeping it one file means adding a new
//! coin — node connection defaults, icon, and engine consensus — is a single
//! edit with no recompile.
//!
//! Resolution order for the file: first the sibling of the Satchel executable
//! (`<exe dir>/coins.toml`, the shipped bundle copy), else `<config dir>/
//! coins.toml` (a user-editable copy). If neither exists, the baked-in
//! [`DEFAULT_COINS_TOML`] is written to the config dir so the user can edit it.
//! A parse error logs and falls back to the baked-in default, never crashing
//! boot.

use serde::Deserialize;
use serde_json::json;
use std::path::{Path, PathBuf};

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

/// Resolve the coins file path, materializing the baked-in default into the
/// config dir if no file exists anywhere. Always returns a path that exists.
pub fn resolve_coins_file(config_dir: &Path) -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        let sibling = exe.with_file_name("coins.toml");
        if sibling.exists() {
            return sibling;
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
                },
            })
        })
        .collect();
    json!({ "network": network, "coins": coins })
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
}
