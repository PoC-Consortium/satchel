//! The trusted chain registry — chains are data, not a hardcoded enum.
//!
//! A [`ChainDef`] is the shipped, trusted definition of one coin: its stable
//! string `id` (which drives RPC routing, the wire `asset` field, and the
//! BIP32 coin-type), its per-network [`ChainParams`], and its capability
//! flags. The **pair resolver** derives which swap protocols two configured
//! coins can run from the *intersection* of their capabilities — there is no
//! curated pair list (SATCHEL_PLAN, "The chain model").
//!
//! Phase A ships exactly two coins (POCX, BTC), in-code and trusted.
//! User-added coins (validated against the connected node) are later work.

use anyhow::{Context, Result};
use serde::Serialize;

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

/// Whether v2 adaptor swaps are permitted on **mainnet**. Stays `false`
/// until the M7 crypto audit signs off (V2_ADAPTOR_SWAPS.md "mainnet gate").
/// Regtest and testnet run freely; mainnet legs are refused until this flips.
pub const ADAPTOR_MAINNET_ENABLED: bool = false;

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
    mainnet: &'static ChainParams,
    testnet: &'static ChainParams,
    regtest: &'static ChainParams,
}

impl ChainDef {
    /// Resolved per-network params for this coin.
    pub fn params(&self, network: Network) -> &'static ChainParams {
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
    mainnet: &POCX_MAINNET,
    testnet: &POCX_TESTNET,
    regtest: &POCX_REGTEST,
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
    mainnet: &BTC_MAINNET,
    testnet: &BTC_TESTNET,
    regtest: &BTC_REGTEST,
};

/// The shipped registry. Order is display order.
pub const REGISTRY: &[&ChainDef] = &[&POCX, &BTC];

/// The [`ChainDef`] for a coin id, if shipped.
pub fn get(coin_id: &str) -> Option<&'static ChainDef> {
    REGISTRY.iter().copied().find(|c| c.id == coin_id)
}

/// Resolved per-network [`ChainParams`] for `(coin_id, network)`.
pub fn lookup(coin_id: &str, network: Network) -> Option<&'static ChainParams> {
    get(coin_id).map(|c| c.params(network))
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
    let coins = REGISTRY;
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

        // The shipped registry is exactly the two known coins.
        let ids: Vec<_> = REGISTRY.iter().map(|c| c.id).collect();
        assert_eq!(ids, vec!["btcx", "btc"]);
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
    fn adaptor_mainnet_gated_others_open() {
        // The one remaining v2 gate: regtest/testnet run, mainnet refused
        // until the audit flips ADAPTOR_MAINNET_ENABLED.
        assert!(adaptor_allowed(Network::Regtest));
        assert!(adaptor_allowed(Network::Testnet));
        assert_eq!(adaptor_allowed(Network::Mainnet), ADAPTOR_MAINNET_ENABLED);
        assert!(
            !ADAPTOR_MAINNET_ENABLED,
            "mainnet stays gated until M7 audit"
        );
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
