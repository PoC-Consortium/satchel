//! The trusted chain registry — chains are data, not a hardcoded enum.
//!
//! A [`ChainDef`] is the shipped, trusted definition of one coin: its stable
//! string `id` (which drives RPC routing, the wire `asset` field, and the
//! BIP32 coin-type), its per-network [`ChainParams`], and its capability
//! flags. The **pair resolver** derives which swap protocols two configured
//! coins can run from the *intersection* of their capabilities — there is no
//! curated pair list (SATCHEL_PLAN, "The chain model").
//!
//! Two coins (POCX, BTC) ship in-code and trusted as the always-present
//! built-ins. A `coins.toml` next to the executables can add **more** coins
//! purely as data (see [`coins_file`]): pactd calls
//! [`init_from_path`] at startup, which leaks the parsed defs into `'static`
//! and merges them with the built-ins. A file coin whose id collides with a
//! built-in is rejected, so the file can never redirect `btc`/`btcx`.

use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::path::Path;
use std::sync::OnceLock;

use crate::coins_file::{self, BuiltCoin, BuiltParams};
use crate::keys::{COIN_BTC, COIN_POCX};
use crate::params::{
    ChainParams, Network, BTC_MAINNET, BTC_REGTEST, BTC_TESTNET, POCX_MAINNET, POCX_REGTEST,
    POCX_TESTNET,
};

/// Consensus features a chain supports, consumed by the pair resolver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Capabilities {
    /// OP_CHECKLOCKTIMEVERIFY (BIP65) — the classic HTLC refund branch.
    pub cltv: bool,
    /// SegWit v0 (P2WSH) — the HTLC output type in pact-htlc-v1.
    pub segwit_v0: bool,
    /// Taproot (BIP341) — required by the (unbuilt) v2 adaptor/MuSig2 path.
    pub taproot: bool,
}

/// A swap protocol the engine can run for a pair of chains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    /// Classic CLTV hash-timelock swap (pact-htlc-v1) — the only built one.
    Htlc,
    /// Adaptor-signature / MuSig2 swap (v2) — capability-gated, NOT built.
    Adaptor,
}

/// Whether the v2 adaptor/MuSig2 engine path is built (libswap `musig`,
/// `taproot`, `adaptor_swap`). Now `true`: [`select_protocol`] may pick
/// `Adaptor` for taproot pairs that lack a classic-HTLC option.
pub const ADAPTOR_BUILT: bool = true;

/// Whether v2 adaptor swaps are permitted on **mainnet**. Now `true`: the
/// engineering blockers are closed — the cooperative redeem is CPFP-bumpable
/// (v2+, commit 340657d), the §7.4 action margins are enforced, and the swap
/// paths were audited in-session. v2+ runs on every network.
pub const ADAPTOR_MAINNET_ENABLED: bool = true;

/// The one remaining v2 gate: is an adaptor swap allowed on `network`?
/// Built everywhere; on mainnet additionally gated on the audit flag.
pub fn adaptor_allowed(network: Network) -> bool {
    ADAPTOR_BUILT && (network != Network::Mainnet || ADAPTOR_MAINNET_ENABLED)
}

/// A shipped, trusted chain definition (≈ today's `params.rs` consts, but as
/// data the engine and UI can enumerate).
pub struct ChainDef {
    /// Stable string id: drives RPC routing, the wire `asset` field, and the
    /// BIP32 coin-type. Lowercase.
    pub id: &'static str,
    pub display_name: &'static str,
    pub symbol: &'static str,
    pub decimals: u8,
    /// BIP32 coin-type for `coin(c)` (spec §4.1); SLIP-44 where it exists.
    pub bip32_coin_type: u32,
    /// Coin-level target block spacing (mirrors each network's `ChainParams`).
    pub target_spacing_secs: u32,
    pub capabilities: Capabilities,
    // Per-network params. `None` means the coin is not defined on that network
    // (a file coin may ship e.g. regtest only). Built-ins define all three.
    mainnet: Option<&'static ChainParams>,
    testnet: Option<&'static ChainParams>,
    regtest: Option<&'static ChainParams>,
}

impl ChainDef {
    /// Resolved per-network params for this coin, if it is defined on `network`.
    pub fn params(&self, network: Network) -> Option<&'static ChainParams> {
        match network {
            Network::Mainnet => self.mainnet,
            Network::Testnet => self.testnet,
            Network::Regtest => self.regtest,
        }
    }
}

pub const POCX: ChainDef = ChainDef {
    id: "btcx",
    display_name: "Bitcoin PoCX",
    symbol: "BTCX",
    decimals: 8,
    bip32_coin_type: COIN_POCX,
    target_spacing_secs: 120,
    // PoCX activated Taproot at genesis (ALWAYS_ACTIVE); CLTV + segwit v0
    // are standard.
    capabilities: Capabilities {
        cltv: true,
        segwit_v0: true,
        taproot: true,
    },
    mainnet: Some(&POCX_MAINNET),
    testnet: Some(&POCX_TESTNET),
    regtest: Some(&POCX_REGTEST),
};

pub const BTC: ChainDef = ChainDef {
    id: "btc",
    display_name: "Bitcoin",
    symbol: "BTC",
    decimals: 8,
    bip32_coin_type: COIN_BTC,
    target_spacing_secs: 600,
    capabilities: Capabilities {
        cltv: true,
        segwit_v0: true,
        taproot: true,
    },
    mainnet: Some(&BTC_MAINNET),
    testnet: Some(&BTC_TESTNET),
    regtest: Some(&BTC_REGTEST),
};

/// The always-present, trusted built-in coins. Order is display order.
fn builtins() -> Vec<&'static ChainDef> {
    vec![&POCX, &BTC]
}

/// The active registry: built-ins plus any coins loaded from `coins.toml`.
/// Lazily initialized to the built-ins if [`init_from_path`]/[`init_from_str`]
/// was never called (the CLI, the harness, and library tests run this way).
static REGISTRY: OnceLock<Vec<&'static ChainDef>> = OnceLock::new();

/// All registered coins (built-ins + file coins), in display order.
pub fn all() -> &'static [&'static ChainDef] {
    REGISTRY.get_or_init(builtins).as_slice()
}

/// Leak an owned [`BuiltParams`] into a `'static` [`ChainParams`]. Called once
/// per file coin/network at startup; the leak is intentional (registry lives
/// for the whole process).
fn leak_params(p: BuiltParams) -> &'static ChainParams {
    Box::leak(Box::new(ChainParams {
        coin_id: Box::leak(p.coin_id.into_boxed_str()),
        network: p.network,
        header_format: p.header_format,
        magic: p.magic,
        default_p2p_port: p.default_p2p_port,
        p2pkh_prefix: p.p2pkh_prefix,
        p2sh_prefix: p.p2sh_prefix,
        wif_prefix: p.wif_prefix,
        bech32_hrp: Box::leak(p.bech32_hrp.into_boxed_str()),
        genesis_hash: Box::leak(p.genesis_hash.into_boxed_str()),
        target_spacing_secs: p.target_spacing_secs,
        min_feerate_sat_vb: p.min_feerate_sat_vb,
    }))
}

/// Leak an owned [`BuiltCoin`] into a `'static` [`ChainDef`].
fn leak_def(c: BuiltCoin) -> &'static ChainDef {
    Box::leak(Box::new(ChainDef {
        id: Box::leak(c.id.into_boxed_str()),
        display_name: Box::leak(c.display_name.into_boxed_str()),
        symbol: Box::leak(c.symbol.into_boxed_str()),
        decimals: c.decimals,
        bip32_coin_type: c.bip32_coin_type,
        target_spacing_secs: c.target_spacing_secs,
        capabilities: c.capabilities,
        mainnet: c.mainnet.map(leak_params),
        testnet: c.testnet.map(leak_params),
        regtest: c.regtest.map(leak_params),
    }))
}

/// Parse + validate + leak a `coins.toml` document into `'static` defs (no
/// global state touched — used directly by tests).
pub fn build_defs_from_str(toml_str: &str) -> Result<Vec<&'static ChainDef>> {
    Ok(coins_file::parse_and_validate(toml_str)?
        .into_iter()
        .map(leak_def)
        .collect())
}

/// Built-ins + `extra`, where a built-in id always wins (file coins that
/// collide with `btc`/`btcx` are dropped and reported). Order: built-ins first,
/// then added coins in file order.
fn merge(extra: Vec<&'static ChainDef>) -> (Vec<&'static ChainDef>, Vec<String>) {
    let mut coins = builtins();
    let mut dropped = Vec::new();
    for e in extra {
        if coins.iter().any(|c| c.id == e.id) {
            dropped.push(e.id.to_string());
        } else {
            coins.push(e);
        }
    }
    (coins, dropped)
}

/// Initialize the registry from a `coins.toml` string. Built-ins stay
/// authoritative. Returns the ids of file coins dropped for colliding with a
/// built-in (so the caller can warn). Errors if the registry was already
/// initialized (call once, at startup, before any [`get`]/[`all`]).
pub fn init_from_str(toml_str: &str) -> Result<Vec<String>> {
    let (coins, dropped) = merge(build_defs_from_str(toml_str)?);
    REGISTRY
        .set(coins)
        .map_err(|_| anyhow!("chain registry already initialized"))?;
    Ok(dropped)
}

/// Initialize the registry from a `coins.toml` file path. See [`init_from_str`].
pub fn init_from_path(path: &Path) -> Result<Vec<String>> {
    let s = std::fs::read_to_string(path)
        .with_context(|| format!("reading coins file {}", path.display()))?;
    init_from_str(&s)
}

/// The [`ChainDef`] for a coin id, if registered.
pub fn get(coin_id: &str) -> Option<&'static ChainDef> {
    all().iter().copied().find(|c| c.id == coin_id)
}

/// Resolved per-network [`ChainParams`] for `(coin_id, network)`. `None` if the
/// coin is unknown or not defined on that network.
pub fn lookup(coin_id: &str, network: Network) -> Option<&'static ChainParams> {
    get(coin_id).and_then(|c| c.params(network))
}

/// BIP32 coin-type for `coin_id` (spec §4.1 `coin(c)`).
pub fn bip32_coin_type(coin_id: &str) -> Result<u32> {
    get(coin_id)
        .map(|c| c.bip32_coin_type)
        .with_context(|| format!("unknown coin {coin_id:?} (not in the shipped registry)"))
}

/// Capability-derived protocols for a pair of capability sets — the
/// intersection rule. Pure: it reports what the *capabilities* allow and
/// ignores build flags (so taproot-only pairs report `Adaptor` even though
/// the engine cannot run it yet — see [`select_protocol`]).
///
/// - classic **HTLC** needs `cltv && segwit_v0` on both,
/// - **adaptor / MuSig2** needs `taproot` on both.
pub fn protocols_for(a: Capabilities, b: Capabilities) -> Vec<Protocol> {
    let mut out = Vec::new();
    if a.cltv && a.segwit_v0 && b.cltv && b.segwit_v0 {
        out.push(Protocol::Htlc);
    }
    if a.taproot && b.taproot {
        out.push(Protocol::Adaptor);
    }
    out
}

/// Capability-derived protocols between two *shipped* coins. Errors if
/// either coin is not in the registry.
pub fn protocols_for_pair(coin_a: &str, coin_b: &str) -> Result<Vec<Protocol>> {
    let a = get(coin_a).with_context(|| format!("unknown coin {coin_a:?}"))?;
    let b = get(coin_b).with_context(|| format!("unknown coin {coin_b:?}"))?;
    Ok(protocols_for(a.capabilities, b.capabilities))
}

/// The protocol the engine prefers for a pair, honoring build flags: HTLC
/// stays the default when both legs support it (v1 unchanged); otherwise
/// adaptor *if built* ([`ADAPTOR_BUILT`]). Network-independent — the mainnet
/// gate is applied separately (see [`adaptor_allowed`] /
/// `engine::ensure_pair_supported`). `None` means not swappable.
pub fn select_protocol(a: Capabilities, b: Capabilities) -> Option<Protocol> {
    let available = protocols_for(a, b);
    if available.contains(&Protocol::Htlc) {
        return Some(Protocol::Htlc);
    }
    if ADAPTOR_BUILT && available.contains(&Protocol::Adaptor) {
        return Some(Protocol::Adaptor);
    }
    None
}

/// Derived availability of one shipped pair for the current setup — what the
/// `listpairs` RPC and the coin-setup UI render. Pairs are never curated:
/// `protocols` falls straight out of the capability intersection, `available`
/// folds in both "is it actually built" ([`select_protocol`]) and "are both
/// legs configured with a backend".
#[derive(Debug, Clone, Serialize)]
pub struct PairInfo {
    pub coin_a: String,
    pub coin_b: String,
    /// Capability-derived protocols (ignores build flags) — the honest menu.
    pub protocols: Vec<Protocol>,
    /// The protocol the engine would actually run, honoring build flags.
    pub selectable: Option<Protocol>,
    /// Both legs have a configured chain-data backend in this setup.
    pub both_configured: bool,
    /// Tradable right now: both configured AND a built protocol resolves.
    pub available: bool,
}

/// Derived pair availability over every unordered pair of *shipped* coins.
/// `configured` is the set of coin ids that currently have a backend, so the
/// UI can show both the tradable pairs and the "configure the other leg" ones.
pub fn derive_pairs(configured: &[&str]) -> Vec<PairInfo> {
    let coins = all();
    let mut out = Vec::new();
    for i in 0..coins.len() {
        for j in (i + 1)..coins.len() {
            let (a, b) = (coins[i], coins[j]);
            let protocols = protocols_for(a.capabilities, b.capabilities);
            let selectable = select_protocol(a.capabilities, b.capabilities);
            let both_configured = configured.contains(&a.id) && configured.contains(&b.id);
            out.push(PairInfo {
                coin_a: a.id.to_string(),
                coin_b: b.id.to_string(),
                protocols,
                selectable,
                both_configured,
                available: both_configured && selectable.is_some(),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    // These tests assert build-flag constants (ADAPTOR_BUILT /
    // ADAPTOR_MAINNET_ENABLED) on purpose — that's the point of the test, so
    // clippy's "assertion on a constant" lint doesn't apply.
    #![allow(clippy::assertions_on_constants)]
    use super::*;

    #[test]
    fn registry_lookup() {
        assert_eq!(get("btcx").unwrap().id, "btcx");
        assert_eq!(get("btc").unwrap().symbol, "BTC");
        assert_eq!(get("btcx").unwrap().display_name, "Bitcoin PoCX");
        assert_eq!(get("btcx").unwrap().symbol, "BTCX");
        assert!(get("doge").is_none());
        assert!(get("BTCX").is_none(), "ids are case-sensitive, lowercase");

        // Per-network params resolve through the registry.
        assert_eq!(
            lookup("btcx", Network::Regtest).unwrap().genesis_hash,
            POCX_REGTEST.genesis_hash
        );
        assert_eq!(
            lookup("btc", Network::Mainnet).unwrap().bech32_hrp,
            BTC_MAINNET.bech32_hrp
        );
        assert!(lookup("doge", Network::Regtest).is_none());

        // BIP32 coin-types match the keys-module constants (spec §4.1).
        assert_eq!(bip32_coin_type("btcx").unwrap(), COIN_POCX);
        assert_eq!(bip32_coin_type("btc").unwrap(), COIN_BTC);
        assert!(bip32_coin_type("doge").is_err());

        // With no coins.toml loaded, the registry is exactly the two built-ins.
        let ids: Vec<_> = all().iter().map(|c| c.id).collect();
        assert_eq!(ids, vec!["btcx", "btc"]);
    }

    const DOGE_TOML: &str = r#"
[[coin]]
coin_id = "doge"
display_name = "Dogecoin"
symbol = "DOGE"
decimals = 8
bip32_coin_type = 3
target_spacing_secs = 60
capabilities = { cltv = true, segwit_v0 = true, taproot = false }
  [coin.regtest]
  consensus = { header_format = "bitcoin", magic = "fabfb5da", default_p2p_port = 18444, p2pkh_prefix = 111, p2sh_prefix = 196, wif_prefix = 239, bech32_hrp = "dcrt", genesis_hash = "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206" }
"#;

    // Exercises the parse → leak → merge path without touching the process-wide
    // OnceLock (so it can't contaminate other tests that read `all()`).
    #[test]
    fn file_coin_merges_and_resolves() {
        let defs = build_defs_from_str(DOGE_TOML).unwrap();
        let (merged, dropped) = merge(defs);
        assert!(dropped.is_empty());
        let ids: Vec<_> = merged.iter().map(|c| c.id).collect();
        assert_eq!(ids, vec!["btcx", "btc", "doge"]);

        let doge = merged.iter().copied().find(|c| c.id == "doge").unwrap();
        assert_eq!(doge.bip32_coin_type, 3);
        assert!(!doge.capabilities.taproot);
        // regtest-only: mainnet/testnet absent, regtest resolves.
        assert!(doge.params(Network::Mainnet).is_none());
        let rt = doge.params(Network::Regtest).unwrap();
        assert_eq!(rt.bech32_hrp, "dcrt");
        assert_eq!(rt.coin_id, "doge");

        // A capability-derived pair btcx<->doge is HTLC (doge lacks taproot).
        assert_eq!(
            protocols_for(POCX.capabilities, doge.capabilities),
            vec![Protocol::Htlc]
        );
    }

    // A file coin that reuses a built-in id is dropped — the file can never
    // redirect btc/btcx to different consensus params.
    #[test]
    fn builtin_id_collision_is_dropped() {
        let evil = DOGE_TOML
            .replace("coin_id = \"doge\"", "coin_id = \"btc\"")
            .replace("bech32_hrp = \"dcrt\"", "bech32_hrp = \"evil\"");
        let defs = build_defs_from_str(&evil).unwrap();
        let (merged, dropped) = merge(defs);
        assert_eq!(dropped, vec!["btc".to_string()]);
        // The surviving `btc` is still the trusted built-in.
        let btc = merged.iter().copied().find(|c| c.id == "btc").unwrap();
        assert_eq!(btc.params(Network::Mainnet).unwrap().bech32_hrp, "bc");
    }

    #[test]
    fn capability_flags() {
        // Both shipped coins are full UTXO chains with Taproot.
        for coin in [&POCX, &BTC] {
            let c = coin.capabilities;
            assert!(c.cltv && c.segwit_v0 && c.taproot, "{} caps", coin.id);
        }
    }

    #[test]
    fn pair_resolver_yields_protocols_from_capabilities() {
        let full = Capabilities {
            cltv: true,
            segwit_v0: true,
            taproot: true,
        };
        let no_taproot = Capabilities {
            cltv: true,
            segwit_v0: true,
            taproot: false,
        };
        let no_segwit = Capabilities {
            cltv: true,
            segwit_v0: false,
            taproot: true,
        };
        let taproot_only = Capabilities {
            cltv: false,
            segwit_v0: false,
            taproot: true,
        };

        // Full caps on both: HTLC and adaptor both *capability-available*.
        assert_eq!(
            protocols_for(full, full),
            vec![Protocol::Htlc, Protocol::Adaptor]
        );
        // No taproot on one leg: HTLC only.
        assert_eq!(protocols_for(full, no_taproot), vec![Protocol::Htlc]);
        assert_eq!(protocols_for(no_taproot, no_taproot), vec![Protocol::Htlc]);
        // Missing segwit on one leg kills HTLC; taproot may still allow adaptor.
        assert_eq!(protocols_for(full, no_segwit), vec![Protocol::Adaptor]);
        // A pure-taproot pair reports adaptor only (no UTXO HTLC).
        assert_eq!(
            protocols_for(taproot_only, taproot_only),
            vec![Protocol::Adaptor]
        );
        // Nothing in common -> no protocols.
        let none = Capabilities {
            cltv: false,
            segwit_v0: false,
            taproot: false,
        };
        assert!(protocols_for(none, full).is_empty());

        // The shipped pair POCX<->BTC supports HTLC.
        assert!(protocols_for_pair("btcx", "btc")
            .unwrap()
            .contains(&Protocol::Htlc));
        assert!(protocols_for_pair("btcx", "doge").is_err());
    }

    #[test]
    fn select_protocol_prefers_htlc_adaptor_now_built() {
        let full = Capabilities {
            cltv: true,
            segwit_v0: true,
            taproot: true,
        };
        let taproot_only = Capabilities {
            cltv: false,
            segwit_v0: false,
            taproot: true,
        };
        let no_segwit = Capabilities {
            cltv: true,
            segwit_v0: false,
            taproot: true,
        };

        // HTLC stays the default for any cltv+segwit pair (v1 unchanged).
        assert_eq!(select_protocol(full, full), Some(Protocol::Htlc));
        // Adaptor is now BUILT: a taproot pair with no HTLC option selects it.
        assert!(ADAPTOR_BUILT);
        assert_eq!(
            select_protocol(taproot_only, taproot_only),
            Some(Protocol::Adaptor)
        );
        assert_eq!(select_protocol(no_segwit, full), Some(Protocol::Adaptor));

        // The shipped pair still selects HTLC by default.
        assert_eq!(
            select_protocol(POCX.capabilities, BTC.capabilities),
            Some(Protocol::Htlc)
        );
    }

    #[test]
    fn adaptor_allowed_on_every_network() {
        // v2+ is enabled everywhere now that the redeem is CPFP-bumpable and the
        // §7.4 margins are enforced (ADAPTOR_MAINNET_ENABLED flipped true).
        assert!(adaptor_allowed(Network::Regtest));
        assert!(adaptor_allowed(Network::Testnet));
        assert!(adaptor_allowed(Network::Mainnet));
        assert!(ADAPTOR_MAINNET_ENABLED);
    }

    #[test]
    fn derive_pairs_reflects_configuration() {
        // Nothing configured: the one shipped pair exists but is not tradable.
        let none = derive_pairs(&[]);
        assert_eq!(none.len(), 1, "exactly one unordered pair (pocx, btc)");
        let p = &none[0];
        assert_eq!((p.coin_a.as_str(), p.coin_b.as_str()), ("btcx", "btc"));
        assert_eq!(p.protocols, vec![Protocol::Htlc, Protocol::Adaptor]);
        assert_eq!(p.selectable, Some(Protocol::Htlc));
        assert!(!p.both_configured && !p.available);

        // Only one leg configured: still not tradable.
        let one = derive_pairs(&["btcx"]);
        assert!(!one[0].both_configured && !one[0].available);

        // Both legs configured: HTLC is selectable, so the pair is available.
        let both = derive_pairs(&["btcx", "btc"]);
        assert!(both[0].both_configured && both[0].available);
        assert_eq!(both[0].selectable, Some(Protocol::Htlc));

        // Configuration order / unknown extras don't change the derivation.
        let messy = derive_pairs(&["btc", "doge", "btcx"]);
        assert!(messy[0].both_configured && messy[0].available);
    }

    #[test]
    fn protocol_serializes_lowercase() {
        // The wire/JSON form the UI consumes is the lowercase name.
        assert_eq!(serde_json::to_string(&Protocol::Htlc).unwrap(), "\"htlc\"");
        assert_eq!(
            serde_json::to_string(&Protocol::Adaptor).unwrap(),
            "\"adaptor\""
        );
    }
}
