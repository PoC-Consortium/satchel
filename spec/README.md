# Swap Protocol Spec

The written atomic swap protocol: HTLC construction (CLTV-based), message
flow between counterparties, timelock rules (`T2 < T1`, hours not days),
refund procedure, and test vectors — written so third parties can implement
independently of Pact.

## Contents

- [`protocol.md`](protocol.md) — HTLC v1 spec (`pact-htlc-v1`): scripts, tx
  templates, key derivation paths, preimage rules, timelock rules, and the
  counterparty message handshake (§8 — a separate `messages.md` will only
  appear if the transport layer outgrows it)
- [`vectors/`](vectors/) — deterministic test vectors for PoCX↔BTC
  (regenerate: `cargo run -p libswap --example gen-vectors` in `pact/`;
  pinned by `pact/libswap/tests/vectors.rs`)
- [`protocol-v2.md`](protocol-v2.md) — v2 spec (`pact-htlc-v2`): Taproot/MuSig2
  adaptor swaps. Specifies only what changes from v1; route + rationale in the
  [Pact handbook](../docs/handbook-pact/). Live on mainnet (reviewed).
- `vectors/htlc_v2.json` — v2 vectors (regenerate:
  `cargo run -p libswap --example gen-vectors-v2`; pinned by `tests/vectors_v2.rs`)

## Naming & versioning

There are two protocol **families**, and — separately — each family carries a
**wire epoch**:

| Family (user-facing name) | Spec doc | Wire-id string | Current wire epoch |
|---|---|---|---|
| Standard (HTLC) | [`protocol.md`](protocol.md) ("v1") | `pact-htlc-v1` | 2 |
| Private (Taproot) | [`protocol-v2.md`](protocol-v2.md) ("v2") | `pact-htlc-v2` | 3 |

The "v1"/"v2" in the spec filenames and wire-id strings number the
*families* and never change. The wire epoch is a per-family flag-day counter
for message-format amendments (`WIRE_V1`/`WIRE_V2` and `wire_epoch()` in
`pact/libswap/src/lib.rs`), exposed by `pactd` as `getinfo.wire_epochs`;
both sides of a swap must speak the same epoch for a family — unequal epochs
are refused up-front (offers badge un-takeable, handshakes reject cleanly).
The version numbers Satchel displays — "Standard (HTLC) v2 · Private
(Taproot) v3" on the About page and Corkboard offer chips — are these wire
epochs, **not** the family numbers. The family names, not any number, are
the user-facing identifiers.

## Safety property to preserve

The protocol must never depend on Electrum servers being honest for
*safety* — a lying server can hide information and delay, but timelocks and
refunds must still protect funds.
