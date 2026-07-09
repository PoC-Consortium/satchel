//! The trusted chain registry + the swap-pair resolver.
//!
//! The registry itself — [`ChainDef`], [`Capabilities`], the built-ins and
//! the `coins.toml` merge — lives in the extracted `params-btcx` crate and
//! is re-exported here unchanged. What stays local is the **pair
//! resolver**: it derives which swap *protocols* two configured coins can
//! run from the *intersection* of their capabilities — there is no curated
//! pair list (SATCHEL_PLAN, "The chain model"). Protocol selection is swap
//! engine policy (build flags, mainnet gates), not chain data, so it does
//! not belong in the params crate.

use anyhow::{Context, Result};
use serde::Serialize;

use crate::params::Network;

// The chain registry proper (params-btcx): ChainDef/Capabilities, the
// BTCX/BTC built-ins, coins.toml loading, and the id/network lookups.
pub use params_btcx::registry::{
    all, bip32_coin_type, build_defs_from_str, get, init_from_path, init_from_str, lookup,
    Capabilities, ChainDef, BTC, BTCX,
};

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

        // The shipped pair BTCX<->BTC supports HTLC.
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
            select_protocol(BTCX.capabilities, BTC.capabilities),
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
        assert_eq!(none.len(), 1, "exactly one unordered pair (btcx, btc)");
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
