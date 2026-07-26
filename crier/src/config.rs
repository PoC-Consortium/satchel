//! crier.toml loading + validation. Fail-fast: unknown coin ids in `pairs`
//! are a startup error, not a silent skip.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

/// Satchel's recommended public relay set (mirrors `RECOMMENDED_NOSTR_RELAYS`
/// in `satchel/src/main.rs` — relays live in per-user `satchel.json`, not in
/// any shared config file, so crier carries its own copy).
pub const DEFAULT_RELAYS: &[&str] = &[
    "wss://relay.damus.io",
    "wss://nos.lol",
    "wss://relay.primal.net",
    "wss://nostr.mom",
    "wss://nostr-pub.wellorder.net",
    "wss://offchain.pub",
];

/// Display quote priority: the pair member appearing EARLIEST here becomes the
/// quote, so prices read as unit prices ("what 1 BTCX costs, in BTC"). This
/// deliberately inverts the Corkboard's `QUOTE_PRIORITY` orientation — see
/// PLAN.md §5.
const DISPLAY_QUOTE_PRIORITY: &[&str] = &["btc", "ltc", "doge", "btcx"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoinInfo {
    pub symbol: String,
    pub decimals: u32,
}

/// An unordered coin pair, stored normalized (lexicographic) so
/// `"btc/btcx"` and `"btcx/btc"` are the same pair.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pair {
    pub a: String,
    pub b: String,
}

impl Pair {
    pub fn parse(s: &str) -> Result<Self> {
        let (x, y) = s
            .split_once('/')
            .with_context(|| format!("pair '{s}' is not of the form coin/coin"))?;
        let (x, y) = (x.trim().to_lowercase(), y.trim().to_lowercase());
        if x.is_empty() || y.is_empty() || x == y {
            bail!("pair '{s}' must name two distinct coins");
        }
        Ok(if x < y {
            Pair { a: x, b: y }
        } else {
            Pair { a: y, b: x }
        })
    }

    pub fn from_assets(x: &str, y: &str) -> Self {
        let (x, y) = (x.to_lowercase(), y.to_lowercase());
        if x < y {
            Pair { a: x, b: y }
        } else {
            Pair { a: y, b: x }
        }
    }

    /// Display orientation `(base, quote)` by `DISPLAY_QUOTE_PRIORITY`.
    pub fn orient(&self) -> (String, String) {
        let rank = |c: &str| {
            DISPLAY_QUOTE_PRIORITY
                .iter()
                .position(|p| *p == c)
                .unwrap_or(usize::MAX)
        };
        match rank(&self.a).cmp(&rank(&self.b)) {
            std::cmp::Ordering::Less => (self.b.clone(), self.a.clone()),
            std::cmp::Ordering::Greater => (self.a.clone(), self.b.clone()),
            // Neither coin ranked: deterministic fallback, quote = smaller id.
            std::cmp::Ordering::Equal => (self.b.clone(), self.a.clone()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiscordCfg {
    pub token: Option<String>,
    pub guild_id: u64,
    pub announce_channel_id: u64,
}

#[derive(Debug, Clone)]
pub struct TelegramCfg {
    pub token: Option<String>,
    /// Numeric chat/group id or "@channelname"; empty = no Telegram
    /// announcements (commands still work whenever a token is set).
    pub announce_chat_id: String,
}

#[derive(Debug, Clone)]
pub struct AnnounceCfg {
    pub debounce_secs: u64,
    pub min_interval_secs: u64,
    pub min_size_delta_pct: u64,
    pub initial_sync_secs: u64,
    pub pairs: Vec<Pair>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BtcUnit {
    Btc,
    Mbtc,
    Sat,
}

impl BtcUnit {
    pub fn scale(self) -> f64 {
        match self {
            BtcUnit::Btc => 1.0,
            BtcUnit::Mbtc => 1e3,
            BtcUnit::Sat => 1e8,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            BtcUnit::Btc => "BTC",
            BtcUnit::Mbtc => "mBTC",
            BtcUnit::Sat => "sat",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenderCfg {
    pub btc_unit: BtcUnit,
}

#[derive(Debug, Clone)]
pub struct CashCfg {
    /// Annotate BTC-quoted prices with a USD reference value.
    pub enabled: bool,
    pub refresh_secs: u64,
    /// Drop the USD annotations (rather than show a stale rate) once the last
    /// successful fetch is older than this.
    pub max_age_secs: u64,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub network: String,
    pub relays: Vec<String>,
    pub poll_secs: u64,
    pub state_file: PathBuf,
    pub coins: HashMap<String, CoinInfo>,
    pub discord: DiscordCfg,
    pub telegram: TelegramCfg,
    pub announce: AnnounceCfg,
    pub render: RenderCfg,
    pub cash: CashCfg,
}

// ---- raw (serde) shapes ----

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    network: Option<String>,
    relays: Option<Vec<String>>,
    poll_secs: Option<u64>,
    state_file: Option<String>,
    coins_file: Option<String>,
    #[serde(default)]
    discord: RawDiscord,
    #[serde(default)]
    telegram: RawTelegram,
    #[serde(default)]
    announce: RawAnnounce,
    #[serde(default)]
    render: RawRender,
    #[serde(default)]
    cash: RawCash,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDiscord {
    token: Option<String>,
    guild_id: Option<u64>,
    announce_channel_id: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTelegram {
    token: Option<String>,
    announce_chat_id: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAnnounce {
    debounce_secs: Option<u64>,
    min_interval_secs: Option<u64>,
    min_size_delta_pct: Option<u64>,
    initial_sync_secs: Option<u64>,
    pairs: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRender {
    btc_unit: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCash {
    enabled: Option<bool>,
    refresh_secs: Option<u64>,
    max_age_secs: Option<u64>,
}

fn builtin_coins() -> HashMap<String, CoinInfo> {
    let mut m = HashMap::new();
    for (id, symbol) in [
        ("btcx", "BTCX"),
        ("btc", "BTC"),
        ("ltc", "LTC"),
        ("doge", "DOGE"),
    ] {
        m.insert(
            id.to_string(),
            CoinInfo {
                symbol: symbol.to_string(),
                decimals: 8,
            },
        );
    }
    m
}

/// Merge symbols/decimals from a Satchel `coins.toml` (its `[[coin]]` blocks)
/// over the builtin table. Only `coin_id`/`symbol`/`decimals` are read.
fn merge_coins_file(coins: &mut HashMap<String, CoinInfo>, path: &Path) -> Result<()> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("read coins_file {}", path.display()))?;
    let value: toml::Value = text.parse().context("parse coins_file")?;
    let Some(entries) = value.get("coin").and_then(|v| v.as_array()) else {
        return Ok(());
    };
    for entry in entries {
        let Some(id) = entry.get("coin_id").and_then(|v| v.as_str()) else {
            continue;
        };
        let symbol = entry
            .get("symbol")
            .and_then(|v| v.as_str())
            .unwrap_or(id)
            .to_string();
        let decimals = entry
            .get("decimals")
            .and_then(|v| v.as_integer())
            .unwrap_or(8) as u32;
        coins.insert(id.to_lowercase(), CoinInfo { symbol, decimals });
    }
    Ok(())
}

impl Config {
    /// Load from `path` (missing file = all defaults) and the environment
    /// (`CRIER_DISCORD_TOKEN` overrides `discord.token`).
    pub fn load(path: &Path) -> Result<Config> {
        let raw: Raw = match std::fs::read_to_string(path) {
            Ok(text) => {
                toml::from_str(&text).with_context(|| format!("parse config {}", path.display()))?
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                tracing::warn!("config {} not found — using defaults", path.display());
                Raw::default()
            }
            Err(err) => return Err(err).context(format!("read config {}", path.display())),
        };

        let mut coins = builtin_coins();
        if let Some(cf) = &raw.coins_file {
            let cf_path = if Path::new(cf).is_absolute() {
                PathBuf::from(cf)
            } else {
                path.parent().unwrap_or(Path::new(".")).join(cf)
            };
            merge_coins_file(&mut coins, &cf_path)?;
        }

        let network = raw.network.unwrap_or_else(|| "mainnet".to_string());
        match network.as_str() {
            "mainnet" | "testnet" | "regtest" => {}
            other => bail!("unknown network '{other}' (mainnet|testnet|regtest)"),
        }

        let pairs = raw
            .announce
            .pairs
            .unwrap_or_else(|| vec!["btc/btcx".to_string()])
            .iter()
            .map(|s| Pair::parse(s))
            .collect::<Result<Vec<_>>>()?;
        for p in &pairs {
            for coin in [&p.a, &p.b] {
                if !coins.contains_key(coin) {
                    bail!(
                        "announce pair coin '{coin}' is not a known coin id \
                         (builtin: btcx, btc, ltc, doge — or add it via coins_file)"
                    );
                }
            }
        }

        let btc_unit = match raw.render.btc_unit.as_deref().unwrap_or("mbtc") {
            "btc" => BtcUnit::Btc,
            "mbtc" => BtcUnit::Mbtc,
            "sat" => BtcUnit::Sat,
            other => bail!("render.btc_unit '{other}' (btc|mbtc|sat)"),
        };

        let env_token = |name: &str| std::env::var(name).ok().filter(|t| !t.trim().is_empty());
        let discord_token = env_token("CRIER_DISCORD_TOKEN").or(raw.discord.token);
        let telegram_token = env_token("CRIER_TELEGRAM_TOKEN").or(raw.telegram.token);

        let state_file = raw
            .state_file
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("crier-state.json"));

        Ok(Config {
            network,
            relays: raw
                .relays
                .unwrap_or_else(|| DEFAULT_RELAYS.iter().map(|s| s.to_string()).collect()),
            poll_secs: raw.poll_secs.unwrap_or(30).max(5),
            state_file,
            coins,
            discord: DiscordCfg {
                token: discord_token,
                guild_id: raw.discord.guild_id.unwrap_or(0),
                announce_channel_id: raw.discord.announce_channel_id.unwrap_or(0),
            },
            telegram: TelegramCfg {
                token: telegram_token,
                announce_chat_id: raw.telegram.announce_chat_id.unwrap_or_default(),
            },
            announce: AnnounceCfg {
                debounce_secs: raw.announce.debounce_secs.unwrap_or(30),
                min_interval_secs: raw.announce.min_interval_secs.unwrap_or(60),
                min_size_delta_pct: raw.announce.min_size_delta_pct.unwrap_or(10),
                initial_sync_secs: raw.announce.initial_sync_secs.unwrap_or(60),
                pairs,
            },
            render: RenderCfg { btc_unit },
            cash: CashCfg {
                enabled: raw.cash.enabled.unwrap_or(true),
                refresh_secs: raw.cash.refresh_secs.unwrap_or(300).max(60),
                max_age_secs: raw.cash.max_age_secs.unwrap_or(1800),
            },
        })
    }

    pub fn coin(&self, id: &str) -> &CoinInfo {
        self.coins.get(id).expect("validated coin id")
    }
}

pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pair_is_unordered_and_orients_btc_as_quote() {
        let p1 = Pair::parse("btc/btcx").unwrap();
        let p2 = Pair::parse("BTCX/BTC").unwrap();
        assert_eq!(p1, p2);
        let (base, quote) = p1.orient();
        assert_eq!((base.as_str(), quote.as_str()), ("btcx", "btc"));
    }

    #[test]
    fn config_defaults_and_pair_validation() {
        let dir = std::env::temp_dir().join("crier-test-cfg");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("crier.toml");
        std::fs::write(
            &path,
            "network = \"regtest\"\n[announce]\npairs = [\"btc/btcx\"]\n",
        )
        .unwrap();
        let cfg = Config::load(&path).unwrap();
        assert_eq!(cfg.network, "regtest");
        assert_eq!(cfg.render.btc_unit, BtcUnit::Mbtc);
        assert_eq!(cfg.announce.pairs.len(), 1);
        assert!(cfg.relays.len() >= 5);

        std::fs::write(&path, "[announce]\npairs = [\"btc/nope\"]\n").unwrap();
        assert!(Config::load(&path).is_err());
    }
}
