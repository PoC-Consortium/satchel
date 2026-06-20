# Network Gating, Reorgs & Safety

This chapter states, honestly and precisely, where Pact's swap protocols stand
on safety: which versions are permitted on which networks, how the engine
picks a protocol, how confirmation depth defends against reorgs, and what
residual risks remain. Pact is *alpha*; v1 (HTLC) and v2 (Taproot/MuSig2
adaptor) both run on mainnet, under external audit. The point of this chapter is
to make the safety posture legible, not to oversell it.

## Current gating state

Both protocols are enabled on every network in the current code:

| Protocol | Mainnet gate | State |
|---|---|---|
| v1 (`pact-htlc-v1`) | **none** | permitted on mainnet (the height/HTLC gate was lifted) |
| v2 (`pact-htlc-v2`) | `ADAPTOR_BUILT = true`, `ADAPTOR_MAINNET_ENABLED = true` | enabled on **every** network |

For v2, `ADAPTOR_BUILT = true` (`registry.rs:53`) and
`ADAPTOR_MAINNET_ENABLED = true` (`registry.rs:59`) together make
`adaptor_allowed` return `true` on every network; the test suite asserts
mainnet is allowed. v1 has no mainnet gate flag at all.

> **Warning** — Some older docs and a few stale source comments still describe
> v2 as "refused on mainnet" or "not built". Those are out of date. The shipped
> code runs v2 on every network. Treat the registry constants
> (`ADAPTOR_BUILT`, `ADAPTOR_MAINNET_ENABLED`) as the source of truth, and
> remember that *enabled* is not the same as *audited-complete*: v2 is live but
> still under external audit.

## Protocol selection prefers HTLC

When a pair could run either version, the engine **prefers v1 HTLC**
(`select_protocol`). It chooses v2 adaptor only for Taproot pairs that lack an
HTLC option:

- If both legs support CLTV + segwit (the v1 requirement), the engine selects
  **v1 HTLC**.
- v2 adaptor is selected only for taproot-capable pairs where HTLC is not
  available.

The shipped **BTCX ↔ BTC** pair therefore defaults to **v1 HTLC**. v2 is the
more advanced construction (better privacy, single-sig refund), but the engine
errs toward the older, more-exercised path unless v2 is the only option.

## Confirmation depth and reorg protection

A blockchain reorganization can un-confirm a transaction the engine already
acted on. Pact's defense is to withhold finality until a leg is buried deep
enough that a reorg is implausible (`default_confirmations`,
`engine.rs:388-394`):

| Chain class | Default `N` |
|---|---|
| Regtest | 1 |
| Fast chain (< 5-min spacing; BTCX ≈ 120s) | 10 |
| Slow chain (≥ 5-min spacing; BTC ≈ 600s) | 6 |

Two behaviors flow from this:

- **Auto-redeem / completion is withheld until N-deep.** The engine does not
  treat a swap as done — and keeps the refund armed — until the relevant spend
  is confirmed `N` blocks deep. A shallow reorg that un-confirms a redeem
  re-arms the refund rather than leaving a party exposed.
- **Reorg alert.** If a watched HTLC or leg output *vanishes* from the chain
  (a reorg dropped its funding or spend), the engine raises a reorg alert so
  the operator and the state machine can react rather than silently proceeding
  on a stale view.

Override `N` per coin via `Engine.coin_confirmations` (`satchel.json` →
`--coin-confs`), floored at `≥ 1`. Deeper is safer but slower.

> **Note** — Spec §7.3's suggested-minimums table lists `N_B = 3` for BTC, but
> the engine's heuristic `default_confirmations` returns **6** for any chain
> with ≥ 5-minute block spacing, including BTC. The engine default is more
> conservative than the spec's floor; the spec value is a *minimum*, not the
> shipped default.

## Residual risks, stated honestly

Pact's crypto core — the v1 witness construction, the v2 Taproot output and
tapleaf, the key derivation paths, and the adaptor reveal — matches the specs
and the pinned test vectors. The residual risks are operational, and worth
stating plainly:

- **v2's cooperative redeem is not RBF-bumpable.** Its fee is sealed into the
  pre-signed adaptor sighash. A redeem that misses its deadline can become a
  one-sided loss for the party that revealed. *Mitigation:* fee
  over-provisioning at init (live 6-block estimate × 3, clamped) plus a CPFP
  redeem-bump child that spends the redeem's own sweep output. The single-key
  refund path is always bumpable, so the funder is never the stuck party. See
  the chapter "Fees, Fee-Bumping & Auto-Refund".
- **Relay trust is liveness-only.** Pact never trusts a relay or a chain
  backend for *safety* — timelocks and on-chain enforcement protect funds
  regardless of what any relay says. A misbehaving or offline relay can only
  delay a swap (a liveness failure), never steal from it. The mitigation for a
  dead relay is the timelock refund.
- **Alpha status.** Both protocols are live and under external audit. Run
  amounts you can afford to have locked for the duration of a timelock, keep
  `pactd` running for the life of any funded swap, and prefer the **Medium** or
  **Long** timelock presets for real value (see the chapter "Timelocks &
  Action Deadlines").

> **Warning** — *Alpha; v1 (HTLC) and v2 (Taproot/MuSig2 adaptor) both run on
> mainnet, under external audit.* The atomicity guarantees are real and
> chain-enforced, but the software has not completed audit. Size your swaps
> accordingly.

## Summary

The safety model is: the chain enforces the deal, timelocks bound the worst
case, confirmation depth defends against reorgs, and relays are trusted only
for liveness. The two operational sharp edges — v2's unbumpable cooperative
redeem (mitigated by over-provisioning + CPFP) and the project's alpha,
under-audit status — are the ones to keep in mind when deciding how much value
to route through a swap.
