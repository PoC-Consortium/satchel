//! Typed message bodies — spec §8.
//!
//! The signed envelope, canonical JSON and BIP340 sign/verify live in
//! `pact-proto` (chain-agnostic) and are re-exported here so existing
//! `crate::messages::{Envelope, sign, verify, ...}` callers are unchanged.
//! This module keeps the *typed* bodies. `ChainRef` now names chains by
//! string `coin_id` (keyed into [`crate::registry`]); it serializes the id
//! under the wire key `asset` so pact-htlc-v1 bytes are unchanged.

use serde::{Deserialize, Serialize};

pub use pact_proto::envelope::{canonical_json, sign, signing_digest, verify, Envelope};

use crate::params::Network;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChainRef {
    /// Stable coin id ("btcx", "btc") keyed into the registry. Serialized as
    /// `asset` so pact-htlc-v1 wire bytes are unchanged — the value (the
    /// lowercase coin id) is exactly what the old `Asset` enum encoded to.
    #[serde(rename = "asset")]
    pub coin_id: String,
    pub network: Network,
}

/// Body of `init` (spec §8.3). All amounts are integer base units.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InitBody {
    pub protocol: String,
    pub chain_a: ChainRef,
    pub chain_b: ChainRef,
    pub amount_a: u64,
    pub amount_b: u64,
    pub hash_h: String,
    pub t1: u32,
    pub t2: u32,
    pub n_a: u32,
    pub n_b: u32,
    pub alice_refund_pubkey_a: String,
    pub alice_redeem_pubkey_b: String,
    /// Board offer this init fulfils, echoed back so the taker can match it
    /// to the exact pending take — required to run >1 swap with the same
    /// maker concurrently (C11). Optional for wire compat: direct (boardless)
    /// inits and pre-C11 makers omit it, and the taker falls back to matching
    /// by maker identity. `skip_serializing_if` keeps absent bodies signing /
    /// verifying byte-identically to the pre-C11 wire format.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offer_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AcceptBody {
    pub bob_redeem_pubkey_a: String,
    pub bob_refund_pubkey_b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FundedBody {
    /// "a" or "b".
    pub chain: String,
    pub txid: String,
    pub vout: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RedeemedBody {
    pub preimage: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AbortBody {
    pub reason: String,
}

// ---- v2 (pact-htlc-v2) message bodies (spec v2 §7) ----
// The signed envelope is shared with v1; only the bodies differ. `protocol`
// in InitV2Body is "pact-htlc-v2" — a party that doesn't recognise it aborts.

/// `init` (Alice → Bob): swap terms + Alice's keys + the adaptor point `T`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InitV2Body {
    pub protocol: String,
    pub chain_a: ChainRef,
    pub chain_b: ChainRef,
    pub amount_a: u64,
    pub amount_b: u64,
    pub t1: u32,
    pub t2: u32,
    /// Full (33-byte compressed) MuSig2 signer keys, hex.
    pub alice_swap_a: String,
    pub alice_swap_b: String,
    /// x-only refund key for the leg Alice funds (A).
    pub alice_refund_a: String,
    /// Adaptor point `T = t·G`, compressed hex.
    pub adaptor_point: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offer_id: Option<String>,
}

/// `accept` (Bob → Alice): Bob's keys — both parties can now build both legs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AcceptV2Body {
    pub bob_swap_a: String,
    pub bob_swap_b: String,
    pub bob_refund_b: String,
}

/// `funding_ready` (each → other): the funding output, built but not yet
/// broadcast, so both redeem txs are determined (spec v2 §7).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FundingReadyV2Body {
    /// "a" or "b".
    pub chain: String,
    pub txid: String,
    pub vout: u32,
}

/// `nonces` (each → other): public nonces for both MuSig2 adaptor sessions,
/// hex-encoded.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NoncesV2Body {
    pub redeem_a_pubnonce: String,
    pub redeem_b_pubnonce: String,
}

/// `partial_sigs` (each → other): partial adaptor signatures for both sessions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PartialSigsV2Body {
    pub redeem_a_partial: String,
    pub redeem_b_partial: String,
}
