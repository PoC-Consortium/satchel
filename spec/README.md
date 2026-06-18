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
  adaptor swaps. Specifies only what changes from v1; route + rationale in
  [V2_ADAPTOR_SWAPS.md](../docs/V2_ADAPTOR_SWAPS.md). Active build, mainnet-gated.
- `vectors/htlc_v2.json` — v2 vectors (regenerate:
  `cargo run -p libswap --example gen-vectors-v2`; pinned by `tests/vectors_v2.rs`)

## Safety property to preserve

The protocol must never depend on Electrum servers being honest for
*safety* — a lying server can hide information and delay, but timelocks and
refunds must still protect funds.
