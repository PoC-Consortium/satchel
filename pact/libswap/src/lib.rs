//! libswap — the Pact swap engine library.
//!
//! Implements `spec/protocol.md` (pact-htlc-v1): classic CLTV-based HTLC
//! atomic swaps between PoCX and BTC (and other Bitcoin-derived UTXO
//! chains).
//!
//! Layering:
//! - [`params`] — chain/network constants (read from bitcoin-pocx
//!   chainparams; never guessed) and address encoding.
//! - [`registry`] — the trusted chain registry: coins as data
//!   ([`ChainDef`](registry::ChainDef)) keyed by string id, with capability
//!   flags and the capability-derived pair resolver.
//! - [`keys`] — BIP32 derivation from the Pact seed: identity key, per-swap
//!   keys, deterministic preimages (spec §4).
//! - [`htlc`] — the v1 witness script and P2WSH output (spec §5).
//! - [`messages`] — signed handshake envelopes (spec §8).
//! - [`chain`] — chain-backend trait (Core RPC / Electrum), data treated as
//!   untrusted hints; safety never depends on backend honesty.
//! - [`swap`] — per-swap state machine and transaction building (spec §6,
//!   §7, §9). Stubbed in the scaffold.

pub mod adaptor_engine;
pub mod adaptor_swap;
pub mod board;
pub mod chain;
pub mod engine;
pub mod htlc;
pub mod keys;
pub mod messages;
pub mod musig;
pub mod nostr_board;
pub mod params;
pub mod registry;
pub mod rpc;
pub mod store;
pub mod swap;
pub mod taproot;

pub use pact_proto::PROTOCOL_VERSION;
