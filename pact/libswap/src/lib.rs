//! libswap — the Pact swap engine library.
//!
//! Implements `spec/protocol.md` (pact-htlc-v1): classic CLTV-based HTLC
//! atomic swaps between PoCX and BTC (and other Bitcoin-derived UTXO
//! chains).
//!
//! Layering:
//! - [`params`] — chain/network constants (read from bitcoin-pocx
//!   chainparams; never guessed) and address encoding. Lives in the
//!   extracted `params-btcx` crate; re-exported here.
//! - [`registry`] — the trusted chain registry: coins as data
//!   ([`ChainDef`](registry::ChainDef)) keyed by string id, with capability
//!   flags (from `params-btcx`) and the capability-derived pair resolver
//!   (local — it is swap-protocol policy, not chain data).
//! - [`keys`] — BIP32 derivation from the Pact seed: identity key, per-swap
//!   keys, deterministic preimages (spec §4). Built on `keys-btcx`.
//! - [`htlc`] — the v1 witness script and P2WSH output (spec §5).
//! - [`messages`] — signed handshake envelopes (spec §8).
//! - [`chain`] — chain-backend trait (Core RPC / Electrum), data treated as
//!   untrusted hints; safety never depends on backend honesty. The Electrum
//!   transport itself lives in the extracted `electrum-btcx` crate.
//! - [`swap`] — per-swap state machine and transaction building (spec §6,
//!   §7, §9). Stubbed in the scaffold.

pub mod adaptor_engine;
pub mod adaptor_swap;
pub mod board;
pub mod chain;
pub mod engine;
pub mod fee_policy;
pub mod htlc;
pub mod keys;
pub mod messages;
pub mod musig;
pub mod nostr_board;
pub mod registry;
pub mod rpc;
pub mod store;
pub mod swap;
pub mod taproot;
pub mod wallet_bdk;

// Extracted to the btcx crates (github.com/PoC-Consortium/btcx); re-exported
// under their old module paths so `crate::params::…` / `crate::server_health::…`
// callers are unchanged.
pub use electrum_btcx::server_health;
pub use params_btcx::coins_file;
pub use params_btcx::params;

pub use fee_policy::FeeBumpPolicy;
pub use pact_proto::PROTOCOL_VERSION;

// ---- protocol wire-compatibility epochs (rc10) -----------------------------
// Swap protocols demand byte-identical transaction construction on both
// sides, so nearly every amendment is a hard break. Each protocol family
// therefore carries a single monotonic WIRE EPOCH: equal epochs can trade,
// anything else is refused up-front (offers badge un-takeable, takes and
// handshakes reject cleanly) instead of failing deep inside the handshake.
// A missing `wire` field on the wire parses as 1 — the pre-rc10 era.

/// v1 (classic HTLC) wire epoch — bumped 1→2 in the rc12 recut: confirmation
/// depths became per-side (the taker no longer adopts the maker's `n_a`/`n_b`
/// from the init; each side derives its own from local config and the accept
/// gained advisory `n_a`/`n_b` fields for the display exchange).
pub const WIRE_V1: u32 = 2;
/// v2 (Taproot/MuSig2 adaptor) wire epoch — bumped 1→2 in rc10 (the
/// co-signed redeem's input sequence, part of the shared MuSig2 sighash,
/// became non-replaceable, see `taproot::build_keypath_redeem`); bumped 2→3
/// in the rc12 recut (init/accept exchange advisory per-side `n_a`/`n_b` so
/// the confirmation-wait display is exact; the depths themselves stay local).
pub const WIRE_V2: u32 = 3;

/// The wire epoch THIS build speaks for `protocol`. Unknown protocol names
/// map to 1 — every path that consumes an offer/handshake validates the
/// protocol name itself before (or right after) consulting the epoch.
pub fn wire_epoch(protocol: &str) -> u32 {
    if protocol == adaptor_swap::PROTOCOL_V2 {
        WIRE_V2
    } else {
        WIRE_V1
    }
}
