# Fees, Fee-Bumping & Auto-Refund

Atomic swaps are fee-sensitive in a way ordinary payments are not: a redeem
that fails to confirm before a timelock matures can cost a party its funds (see
the chapter "Timelocks & Action Deadlines"). Pact therefore treats fee-bumping
and auto-refund as first-class, scheduler-driven mechanisms. This chapter
covers how v1 and v2 bump stuck spends — including the important v2 asymmetry
where one path can be bumped and one cannot — the funding-bump nurse, and how
the engine fires the timelock refund automatically and safely.

All the numeric fee parameters in this chapter are a **single configurable
policy** (`FeeBumpPolicy`, `crate::fee_policy`) rather than scattered constants;
the defaults below reproduce the historical behaviour. See "Configurable fee
policy" at the end of this chapter.

## The unified fee-bump strategy (market-tracking)

Every local fee-bump in Pact — the v1 redeem and refund, and the v2 single-key
refund — shares **one** strategy (`maybe_bump`, `engine.rs:3360`;
`adaptor_bump_refund`, `engine.rs:2339`; unified 2026-06-25). It replaces an
earlier market-**blind** escalator that simply added `step_pct` percent per tick
until it hit the ceiling. That escalator caused a real incident: a live mainnet
redeem ratcheted **159 → 358 sat/vB** into a roughly **1 sat/vB** market — a
"redeem fee storm" — because nothing tied the bump to what the market actually
was. The unified strategy fixes that by tracking the live estimate and gating on
block height and mempool state.

**Market-tracking, value-capped target** (`FeeBumpPolicy::target_feerate`,
`fee_policy.rs:160`):

```text
target = min(market,
             value_at_risk × FEE_CAP_PCT / vsize,
             max_feerate_sat_vb).max(1)
```

- `market` is the backend's live feerate estimate, so the bump **never bids
  above what the network is charging** — the storm is impossible by
  construction.
- `value_at_risk × FEE_CAP_PCT / vsize` is the **value-at-risk cap**: with
  `FEE_CAP_PCT = 100` (a hardcoded constant, `fee_policy.rs:28`, *not* a knob)
  the absolute fee can never exceed the amount being claimed. This is Eclair's
  rule — "it wouldn't make sense to pay more in fees than the amount we're
  trying to claim on-chain." It is a last-resort backstop, not the working term;
  the `market` term is what keeps real fees low.
- `max_feerate_sat_vb` (default 500) is the absolute local ceiling.
- The result is floored at 1 sat/vB.

Around that target, the loop applies three gates:

- **Block-driven cadence** (`last_action_height`, `store.rs:87`). The loop acts
  **at most once per block**; if the tip has not advanced since the last action
  it does nothing. This replaces the old 30-second scheduler tick that produced
  the per-tick churn behind the storm — a back-off when the chain is quiet.
- **BIP125 Rule-4 gate.** A replacement is broadcast only when `target` clears
  the current feerate by the node's **incremental-relay fee** (`getmempoolinfo`
  → `incremental_relay_feerate`, `chain.rs:389`). If the market has not risen —
  or we are already paying enough — there is nothing relayable to do, so the
  loop emits no replacement. When a bump *is* warranted, the replacement's
  **absolute** fee is also floored at `old_fee + incremental_relay_feerate ×
  vsize`: because `old_feerate` is a truncated `old_fee / vsize`, a target built
  purely from the feerate can otherwise land just below the node's absolute Rule-4
  minimum and be rejected (`-26`, "insufficient fee, rejecting replacement").
  Flooring the absolute fee guarantees that a bump the engine decides to make is
  one the node will accept.
- **Evicted-only rebroadcast** (`reanchor_if_evicted`, `engine.rs:3457`). When
  no bump is warranted, the engine re-broadcasts the *same* transaction (same
  txid — invisible to the wallet) **only if it actually fell out of the
  mempool**. In steady state the tx is present, so this is a silent no-op — the
  per-tick rebroadcast the old escalator emitted is gone.

A dust guard still applies: a target whose `new_fee` would push the swept output
below `DUST_LIMIT_SAT = 546` (`swap.rs:20`) is treated as "no bump" and falls
through to the evicted-only rebroadcast path rather than dusting the output.

## v1 fee-bumping (RBF)

In v1 **both** the redeem and the refund are RBF-bumpable. Both spends signal
RBF (`nSequence = 0xFFFFFFFD`), and because the v1 keys sign deterministically
(ECDSA), the engine can re-sign a higher-fee replacement unilaterally
(`maybe_bump`). Both follow the **unified market-tracking strategy** above:
`target_feerate` toward the live market, at most once per block, only when the
target clears the BIP125 Rule-4 floor, otherwise a silent evicted-only
rebroadcast (`max_feerate_sat_vb = 500` by default).

## v2 fee-bumping: a split design

v2 is asymmetric, and the asymmetry is load-bearing (spec v2 §8):

| v2 spend | Bumpable? | Why |
|---|---|---|
| Single-key CLTV refund | **Yes**, RBF | single-key, deterministic re-sign |
| Cooperative MuSig2 key-path redeem | **No** | fee sealed into the pre-signed adaptor sighash |

### The refund is RBF-bumpable

The v2 single-key refund (`adaptor_bump_refund`, `engine.rs:2339`) bumps exactly
like v1: the same **unified market-tracking strategy** (block-driven cadence,
value-capped `target_feerate`, BIP125 Rule-4 gate, evicted-only rebroadcast),
with a deterministic single-key Schnorr re-sign and an RBF sequence. No
interactive ceremony is needed because the refund tapleaf is single-signature
(see the chapter "v2 Taproot/MuSig2 Adaptor Swaps").

### The cooperative redeem is NOT bumpable

The cooperative key-path redeem's fee is fixed at signing time: the fee is part
of the sighash the MuSig2 adaptor session signed, and re-signing would require
re-running the interactive ceremony. The engine cannot raise it after the fact.
Two mitigations make this safe in practice.

**(a) Commit the fee at init.** The adaptor redeem feerate is fixed *before*
signing (`engine.rs`, `adaptor_redeem_feerate`):

```text
adaptor_redeem_feerate = live_market_estimate × redeem.committed_mult(1)
                         clamped to [MIN_REDEEM_FEERATE = 1, MAX_REDEEM_FEERATE = 500] sat/vB
                         fallback 20 sat/vB if no backend is reachable
```

With the default `committed_mult = 1` the redeem commits at the **live market
rate, with no over-provision** — the CPFP child below is what lifts it if the
market climbs while it's pending, so padding the committed fee up front is no
longer needed (the multiplier was 2× before this, and 3× before CPFP existed).
Raise `committed_mult` above 1 to deliberately pre-pay a cushion. There is no
longer a regtest special-case: regtest has no fee history, so the estimate lands
on its ≈1 sat/vB floor (`MIN_REDEEM_FEERATE`) like any other quiet chain. Note
the clamp here is the **protocol** bound `MAX_REDEEM_FEERATE` (the value is
negotiated into the init message and validated by the counterparty), *not* the
local `max_feerate_sat_vb` bump ceiling.

**(b) The CPFP redeem-bump child (v2+).** Because the committed redeem is priced
at market (not padded), the CPFP child is the **primary** accelerator, not just a
fallback: if the market rises while the redeem is pending, the claimer drags it
through with a child-pays-for-parent transaction (`adaptor_cpfp_bump`,
`engine.rs:1942-1985`):

- The child spends the redeem's **own vout 0** — the claimer's wallet-owned
  sweep output — so it is self-funded and needs no extra inputs.
- The child signals RBF, so the *child* itself can be bumped further.
- Child vsize is `CPFP_CHILD_VSIZE = 150` (`engine.rs`).
- It emits `adaptor-cpfp` / `adaptor-rebroadcast` events.

This is a **plain CPFP** (no `submitpackage` / package relay): the parent
redeem stays relayable on its own, so a normal CPFP child suffices to drag it
through. Proven by `test_adaptor_redeem_cpfp` (and `..._ltc`, the first v2 swap
on litecoind).

> **Note** — The cooperative redeem is not RBF-bumpable, so it commits at the
> market rate up front and relies on a CPFP child to drag the parent through if
> conditions tighten before the deadline. The single-key refund path is always
> bumpable, so the *funder* is never stuck. See the chapter "Network Support,
> Reorgs & Safety".

## The funding-bump nurse

The redeem and refund spend an existing HTLC output, so their fees come *out of
that output*. The **funding/lock** is different: it is the only wallet-funded
action in a swap, and it had no bump at all — if the market rose above the rate
the lock went out at, it could sit unconfirmed. The funding nurse closes that
gap. It runs each scheduler tick while our own funding is **unconfirmed** and we
are still before that leg's fund-margin deadline, and chases the current market:

```text
target = min(market, max_feerate_sat_vb, funding.reservation_mult × old_feerate)
```

where `old_feerate = fee / vsize` of the broadcast funding (recomputed live, not
persisted). The `reservation_mult × old_feerate` bound keeps the bump within the
balance the pre-flight funds gate (`ensure_can_fund`) set aside as headroom.

This is **liveness, not safety**: a funding that can't keep up simply stalls and
the timelock refund returns the coin — never a loss. The two protocols bump
differently, and the asymmetry mirrors the redeem/refund split:

| | v1 funding | v2 funding |
|---|---|---|
| **Mechanism** | RBF (`bumpfee`) | CPFP-via-change |
| **Why** | the only outpoint-dependent downstream tx is the **single-key** refund — re-sign it locally against the new outpoint | the outpoint feeds the **2-of-2 MuSig2** adaptor sigs already exchanged; RBF would invalidate them, so spend the change output instead and keep the outpoint fixed |

- **v1 (`maybe_bump_funding_v1`).** RBF via the wallet, then re-locate the HTLC
  output, rebuild + re-sign the single-key CLTV refund against the new outpoint,
  and persist atomically. Safe for the counterparty: they detect the lock by
  **scriptPubKey, not txid** (`find_funding` → `scantxoutset`), so an RBF that
  keeps the HTLC output identical is invisible to them — and the nurse runs only
  while the funding is unconfirmed, before they have waited out the
  confirmations. The funding is broadcast explicitly BIP125-replaceable so
  `bumpfee` is accepted. A crash mid-bump self-heals: `find_funding` re-discovers
  the live outpoint on restart.
- **v2 (`maybe_bump_funding_v2`).** A CPFP child spends the funding's
  wallet-owned **change** output, leaving the funding outpoint — and therefore
  the exchanged adaptor sigs and the refund — untouched. A funding with no change
  output (exact-UTXO) cannot be CPFP'd → it stalls → refund (acceptable).

A recoverable `bumpfee`/sign failure (e.g. insufficient funds — the funds gate
is a soft pre-flight, not a lock — or a not-replaceable tx) is a graceful
skip event for that tick, never a crash: the funding stalls and refunds.

## Auto-refund

The refund is the safety net: if a counterparty disappears after a leg is
funded, the funder gets its coin back once the leg's timelock matures. It is
scheduler-driven and clock-based — the operator does nothing.

### v1 auto-refund

The v1 refund is **signed and persisted at funding time** (`engine.rs:2250-2266`).
The fully-signed refund transaction exists on disk the instant a leg is funded,
ready to broadcast even across a daemon restart.

It fires from `try_refund_due` (`engine.rs:2877-2908`), which broadcasts only
when **both** conditions hold:

```text
tip_median_time_min() ≥ locktime      (the least-advanced backend's MTP has reached T)
AND the HTLC output is still unspent
```

Using the **least-advanced** backend's MTP (`tip_median_time_min`) is
conservative: the engine waits until *every* watched chain agrees the timelock
has matured before refunding.

Several safety details:

- **M7 guard.** `refund()` refuses to broadcast a refund that would *race a
  counterparty redeem*. If the counterparty has already redeemed (or could),
  the engine does not fire a refund that the chain would reject or that could
  double-spend the wrong way.
- **`-27` is success.** A node returning `-27` ("transaction already in block
  chain" / already known) is treated as success, not an error — the refund (or
  an equivalent) is already on chain.
- **Armed until N-deep.** The refund stays armed until the redeem is confirmed
  `N` blocks deep (spec §9.5), so a shallow reorg that un-confirms a redeem
  re-arms the refund.

### v2 auto-refund

The v2 refund is **not** pre-signed at funding. It is re-derived from the seed
on each call (`adaptor_refund`, `engine.rs:1599-1660`): a single-key,
deterministic, unattended-safe Schnorr spend over the CLTV tapleaf. Readiness
is the same MTP test:

```text
tip_median_time_min() ≥ leg.locktime
```

Re-deriving from the seed (rather than persisting a signed tx) is possible
because the refund key is a deterministic seed branch and the refund path needs
no MuSig2 nonce — so even a daemon with an empty state DB can refund. This is
the design asymmetry to remember: **v1 pre-signs the refund; v2 re-derives it.**

> **Note** — Both versions refund off the chain clock (MTP), never off local
> wall-clock alone. A timelock is "mature" only when the watched chains' median
> time past actually reaches it.

## Confirmation depth as the reorg-finality knob

How many confirmations the engine waits for before treating a leg as final is
the per-coin reorg-finality knob (`default_confirmations`,
`engine.rs:388-394`):

| Chain class | Default confirmations |
|---|---|
| Regtest | 1 |
| Fast chain (< 5-min block spacing; BTCX ≈ 120s) | 10 |
| Slow chain (≥ 5-min spacing; BTC ≈ 600s) | 6 |

Override per coin via `Engine.coin_confirmations` (`satchel.json` →
`--coin-confs`), floored at `≥ 1`. Deeper confirmations mean stronger reorg
protection at the cost of a slower swap; this is the dial an operator turns to
trade finality against speed. See the chapter "Network Gating, Reorgs &
Safety".

## Configurable fee policy

The numeric fee parameters are one struct, `FeeBumpPolicy` (`crate::fee_policy`),
owned per-merchant by pactd's store and surfaced as typed RPC:

- `getfeepolicy` / `setfeepolicy` — read and update over JSON-RPC (each field a
  plain typed param, no JSON blob), callable from the CLI like any other method.
  A change is validated, applied to the live engine, and persisted, so it
  survives a restart with no Satchel involved.
- **Satchel → Settings → Fees** edits the **active merchant's** policy over the
  same RPC (applied live, no relaunch). It exposes three knobs
  (`max_feerate_sat_vb`, `reservation_mult`, `committed_mult`); every fee is
  market-derived, so there is no minimum-fee floor to show. The retired
  `step_pct` knob was removed from this page when the bump strategy was unified.

Changes take effect on the **next** bump; swaps already funded keep the
`committed_mult` and gate reservation they were funded under (both fixed at
funding time).

| Field | Default | Meaning |
|---|---|---|
| `max_feerate_sat_vb` | 500 sat/vB | ceiling for every local bump; also the hard system max (the estimator is clamped to 500), settable `1..=500` |
| `funding.reservation_mult` | 3× | funds-gate headroom + funding-nurse bound (`× old_feerate`) |
| `redeem.committed_mult` | 1× | v2 committed-redeem multiplier over live market (1 = commit at market, no padding; CPFP lifts it if the market climbs) |

> **Note** — `min_fee_sat` (the old 1000-sat floor) and `redeem.step_pct` /
> `refund.step_pct` (default 50) are **retired**. The floor and the per-tick
> percentage step both drove the old market-blind escalator that the unified
> market-tracking strategy replaced. They survive in the `FeeBumpPolicy` struct
> only for serde back-compat (an old stored policy that still sets them
> deserializes cleanly) and are **not** exposed by `getfeepolicy` /
> `setfeepolicy` or the Satchel Fees page. Setting them has no effect on bump
> behaviour.

Other fee-related constants remain fixed (not policy):

| Constant | Value | Meaning |
|---|---|---|
| `FEE_CAP_PCT` | 100% | value-at-risk cap in `target_feerate`: a bump's fee never exceeds the amount being claimed (Eclair's rule). Hardcoded (`fee_policy.rs:28`), not a knob |
| `DUST_LIMIT_SAT` | 546 sat | swept output must stay above this |
| `MAX_REDEEM_FEERATE` | 500 sat/vB | **protocol** bound on the negotiated redeem feerate (distinct from `max_feerate_sat_vb`) |
| `CPFP_CHILD_VSIZE` | 150 vB | the CPFP redeem/funding-bump child |
| `FUNDING_VSIZE_EST` | 250 vB | sizing estimate for the funds-gate reservation |

The unified RBF strategy (v1 redeem/refund, v2 refund) and the v2
commit-at-market-plus-CPFP strategy are two answers to the same question — *how do
I make sure a time-critical spend confirms?* — chosen because an RBF-able spend
can re-sign freely toward the market, while v2's cooperative redeem cannot and
must commit its fee up front.
