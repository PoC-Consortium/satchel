//! pact-proto — the Pact wire protocol, chain-agnostic.
//!
//! The shared foundation for every Pact component: the engine (`libswap`/
//! `pactd`), the noticeboard (`corkboard`), and any third-party client.
//! It contains only the wire format and the cryptography that the spec
//! (`spec/protocol.md`) defines — no swap logic, no chain parameters, no
//! daemon. Chain identity never appears here (envelope bodies are opaque
//! JSON), which is what keeps the rest of the system coin-agnostic.
//!
//! - [`crypto`] — tagged hashes, swap-id, preimage hash (spec §4).
//! - [`envelope`] — signed message envelopes + canonical JSON (spec §8).
//! - [`seal`] — relay blob encryption: ECDH + ChaCha20-Poly1305 (spec §10).
//! - [`slip`] — private-offer slip codec: base64url(canonical offer envelope)
//!   for out-of-band (chat) delivery (spec/protocol.md §10).

pub mod crypto;
pub mod envelope;
pub mod seal;
pub mod slip;

/// Protocol version string covering the v1 wire format.
pub const PROTOCOL_VERSION: &str = "pact-htlc-v1";
