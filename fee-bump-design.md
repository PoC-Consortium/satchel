# Design — unified fee-bump strategy (v1 + v2)

**Status:** Proposal for review (companion to [`post_mortem.md`](post_mortem.md)).
**Author:** drafted by Claude (Opus 4.8) with Johnny, 2026-06-25.
**Scope:** align every fee-bump site in `pact/libswap/src/engine.rs` (v1 HTLC + v2
adaptor) onto one strategy, informed by LND's `LinearFeeFunction`, Eclair's
`ReplaceableTx*`, and eigenwallet's `estimate_fee`.

> Why: the post-mortem found our v1 redeem fee bumper escalates +50% every 30s tick,
> market-blind, uncapped except by a flat 500 sat/vB — ratcheting live mainnet redeems
> to 159 / 358 sat/vB into a ~1 sat/vB market. The same market-blind `escalate()` also
> drives the v1 refund and the v2 refund. Meanwhile the funding nurses and the v2 redeem
> are already market-aware and gated. This unifies all of them.

---

## 1. Current state — every bump site

| # | Site | `engine.rs` | Mechanism | Feerate logic today | Gated? | Verdict |
|---|---|---|---|---|---|---|
| 1 | v1 funding nurse | `maybe_bump_funding_v1` (3371) | RBF (`wallet_bumpfee`) | `min(market, ceiling, mult×old)` | ✅ `target≤old→no-op`; unconfirmed-only; deadline-gated | ✅ good |
| 2 | **v1 redeem + refund** | `maybe_bump` (3279) | RBF (rebuild) | **`old×1.5` geometric, market-blind** | ❌ | ❌ **bug** |
| 3 | v1 initial spend | `redeem`/`refund` (2666…) | one-shot | `estimatesmartfee 6`, floor 1000 sat | n/a | ⚠️ under-priced (A1) |
| 4 | v2 funding nurse | `maybe_bump_funding_v2` (2186) | CPFP via change | `min(market, ceiling, mult×old)` | ✅ + idempotent (skip if child exists) | ✅ good |
| 5 | v2 redeem initial | `adaptor_redeem_feerate` (563) | baked into adaptor sig at funding | `market × committed_mult(2)`, clamped to protocol bound | n/a | ✅ good |
| 6 | v2 redeem nurse | `adaptor_keep_moving`→`adaptor_cpfp_bump` (2096/2127) | CPFP child | child `min(market, ceiling)` | ✅ bump gated, **but rebroadcasts parent every tick** | ⚠️ partial |
| 7 | **v2 refund** | `adaptor_bump_refund` (2317) | RBF (rebuild) | **`old×1.5` geometric, market-blind** | ❌ | ❌ **bug** |

**Misalignment in one line:** the three RBF spend-bumpers that route through
`FeeBumpPolicy::escalate(old, step_pct, vsize)` — **v1 redeem, v1 refund, v2 refund** —
are market-blind geometric escalators with no "already enough" gate. Everything else
(funding nurses 1/4, v2 redeem 5/6) is already market-aware and gated. Two properties
are missing *everywhere*, even from the good sites:

- **A value-proportional fee cap** (LND budget / Eclair "funds at risk" / eigenwallet 20%). We have only an absolute 500 sat/vB.
- **Block-driven cadence**: every nurse fires on the 30s wall-clock tick, not per block.

---

## 2. Target strategy

One decision applies to **all six** nurses. The only per-site differences are the
*mechanism* (RBF vs CPFP — intrinsic to the tx type, not a choice) and the inputs
(which deadline, which value-at-risk).

### 2.1 The feerate target — a three-way `min`

```text
target_feerate(leg) =
    min(
        market_feerate(chain),                      // node estimator (existing fee_rate_sat_per_vb)
        value_at_risk(leg) * fee_cap_pct / vsize,   // cap at the value being claimed (default 100%)
        policy.max_feerate_sat_vb                   // existing absolute ceiling
    )
```

- `market_feerate` is the node's estimator at its standard conf target — the **existing
  `fee_rate_sat_per_vb`** the funding nurses already use — so it tracks the market *by
  construction* and can never bid 159 sat/vB into a 1 sat/vB market the way `escalate()`
  did. Using the same source the funding nurses use is what makes the claim legs price
  identically to funding (the v1/v2 alignment goal).
- **The value-at-risk cap is the key new guardrail** — Eclair's rule: *"it wouldn't make
  sense to pay more in fees than the amount we're trying to claim on-chain."* Above the
  value you're claiming you'd be paying to lose money, so `fee_abs ≤ value_at_risk` is the
  hard ceiling. `fee_cap_pct` defaults to **100%** (the full claim, like Eclair); set it
  lower (e.g. 50%) for a tighter sanity bound. On its own this caps both post-mortem
  incidents. *Distinct from `redeem.committed_mult` (= 2), which is the v2 **initial**
  feerate over-provision, not a cap — unchanged.*
- This **replaces `escalate()` entirely**; `redeem.step_pct` / `refund.step_pct` retire.

### 2.2 The deadline gate — derived from the §7.4 margins (unchanged on/off behaviour)

§7.4 timelocks are **time-based**: `T1`/`T2` are unix timestamps and each leg-action has a
margin `M` from `action_margins(net)` (mainnet: fund `3h`, reveal `2h`, redeem-A `1h`;
regtest `0`). The existing on/off gate is `action_safe(now, M, T)` ≡ `now + M < T`, and it
stays exactly as today: it decides **whether** to nurse, not **how hard**. Past the cutoff
the nurse stops and the leg stalls → refund. The feerate itself is set purely by §2.1's
market estimator + value cap; we deliberately do **not** scale the feerate by deadline
proximity (that keeps the claim legs identical in shape to the funding nurses, which also
only gate on/off — the v1/v2 alignment goal). Deadline-proximity urgency scaling is a
possible future refinement, out of scope here.

### 2.3 The decision loop — shared by all six nurses

```text
on tick, for a swap leg with an unconfirmed funding/spend tx:

  0. if tip_height == rec.last_action_height:        return no-op    # back out: same block
  1. confs = tx_confirmations(tx)
     if confs ≥ policy_confs:                         advance state;  return   # policy confs everywhere → closes A3
     if confs ≥ 1:                                    return no-op    # mined but shallow: can't RBF a mined tx; just wait for depth
  2. now = deadline_clock(net, local_now, chain_mtp)
     if !action_safe(now, M, T):                     return no-op    # past §7.4 cutoff → stall→refund
  3. target = target_feerate(leg)                                    # §2.1
  4. if target > current_feerate:                                    # bump warranted
        build RBF replacement (or CPFP child) at `target`
        broadcast
        rec.last_action_height = tip_height                          # record the block we acted in
        return bump event
  5. else:                                                           # nothing to bump
        if tx is NOT in the mempool (evicted):  rebroadcast the SAME tx (same txid)
        else:                                    return no-op        # steady state = silent
```

Properties this gives, mapped to the three design questions:

1. **Keep the nurse** — yes; the nurse *pattern* is correct (every reference + our own
   funding nurses confirm it). Only its pricing changes: `escalate()` → `target_feerate()`.
2. **Keep the tick, back out on same height** — step 0. `last_action_height` (new field on
   `SwapRecord` and `AdaptorSwapRecord`) converts the 30s poll into **block-driven** action
   while keeping the existing tick loop. This alone turns the post-mortem's 8–10-tx storm
   into ≤1 action per block.
3. **Rebroadcast when nothing changed?** — **No new txid; re-anchor the same tx only if it
   was evicted.**
   - A *new higher-fee replacement* (new txid) happens only at step 4, only when a bump is
     warranted. This is what spammed Bitcoin-QT; it is now block-gated and market-gated.
   - *Re-broadcasting the identical tx* (same txid) is invisible to the wallet — Core won't
     create a new entry for an already-known tx — and only recovers from mempool eviction,
     so we do it **only when the tx is actually missing** (step 5), not every tick
     (eigenwallet's `ensure_broadcasted` pattern). **Steady state ⇒ the nurse broadcasts
     nothing**, eliminating the storm rather than slowing it.

### 2.4 Per-site application (mechanism unchanged; inputs)

| Site | Mechanism | deadline `T` | margin `M` | `value_at_risk` |
|---|---|---|---|---|
| v1 funding | RBF | own refund timelock (T1/T2) | `fund` (3h) | amount locked |
| v1 redeem | RBF | **T2** | `reveal` (2h) | `amount_b` (claim) |
| v1 refund | RBF | own refund timelock | leg margin | `amount_a` (recover) |
| v2 funding | CPFP (change) | own refund timelock | `fund` (3h) | amount locked |
| v2 redeem | CPFP (child) | T2 | `reveal` (2h) | `amount_b` |
| v2 refund | RBF | own refund timelock | leg margin | amount recovered |

Initial broadcasts: v2 redeem already prices well via `committed_mult` (§5). **Apply the
same to the v1 initial redeem/refund** (fixes A1): price the first broadcast with
`target_feerate()` instead of `estimatesmartfee 6` floored at 1000, so the nurse is a rare
safety net, not the primary confirmation mechanism. Keep a *dynamic* min-relay floor
(`max(estimate, live mempool-min, broadcast-min)`), not the flat 1000-sat constant.

**v2 funding must guarantee a CPFP anchor (A8).** v2 funding can't RBF (the txid is locked by
the exchanged MuSig2 adaptor sigs), so its only bump path is CPFP on a change output — which
today depends on the node's coin selection and may not exist (exact-UTXO funding →
unbumpable → stall → forced refund). Build the v2 funding ourselves (`fundrawtransaction`,
explicit feerate) with a **guaranteed wallet-owned change/anchor output ≥ a CPFP-viable
minimum**, so `maybe_bump_funding_v2` always has something to spend. This makes v2 funding
bumpability deterministic and lets us set the funding feerate directly (subsumes A7 for v2).
v1 funding is exempt — it can always RBF regardless of change.

---

## 3. Policy / type changes

`FeeBumpPolicy` (`fee_policy.rs`):

- **Add** `fee_cap_pct: u64` (default `100`) — the value-at-risk cap (fee never exceeds this
  % of the amount being claimed/recovered; Eclair's model). `redeem.committed_mult` (= 2)
  is a separate, unchanged knob (v2 *initial* feerate over-provision, not a cap).
- **Retire** `redeem.step_pct` and `refund.step_pct` and `FeeBumpPolicy::escalate()` — no
  longer used once §2.1 lands.
- **Keep** `max_feerate_sat_vb` (absolute ceiling), `min_fee_sat`, `funding.reservation_mult`,
  `redeem.committed_mult` (v2 initial pricing).

Records: **add** `last_action_height: u32` to `SwapRecord` and `AdaptorSwapRecord` (per the
no-backward-compat principle: required field, no migration default).

Backend (`chain.rs`): **add** `is_in_mempool(txid) -> bool` (or reuse `tx_confirmations`
returning a mempool/absent signal) for step 5's evicted-only rebroadcast, and
`incremental_relay_feerate()` from
`getmempoolinfo.incrementalrelayfee` (closes **A4**: the new RBF replacement must beat the old
fee by the node's incremental relay fee, not the hardcoded 1 sat/vB that `escalate()` assumed,
or replacements get rejected on nodes with a higher `incrementalrelayfee`). Consider
median-not-MAX across heterogeneous estimators **only because** the new value cap now bounds
the downside (revises post-mortem A2: "cap-then-MAX is fine; MAX-without-cap is the hazard").

---

## 4. What changes vs today

- **Add** `target_feerate()` + the `fee_cap_pct` value-at-risk cap.
- **Replace** `escalate()` in `maybe_bump` (v1 redeem/refund) and `adaptor_bump_refund`
  (v2 refund) with `target_feerate()` + the §2.3 loop.
- **Add** `last_action_height` + same-height backout (step 0) to all six nurses.
- **Change** rebroadcast to evicted-only (touches `maybe_bump`, `adaptor_keep_moving`, both
  refund bumpers — the v2 redeem stops re-broadcasting its parent every tick).
- **Fix A1**: v1 initial redeem/refund priced via `target_feerate()`; dynamic min-relay floor.
- **Unchanged**: both funding nurses are already conformant — they only *gain* the value cap
  and the same-height backout for free; v2 redeem `committed_mult` initial pricing stays.

End state: one strategy, six call sites, aligned; the funding nurses define the shape and
the three buggy RBF bumpers converge onto it plus a value cap.

---

## 5. Coverage of post-mortem findings

| Finding | Status in this design |
|---|---|
| Root cause (market-blind escalator) | ✅ §2 — `target_feerate()` + decision loop, all six sites |
| A1 (under-priced initial spend) | ✅ §2.4 + §4 — price v1 initial like v2's `committed_mult`; dynamic min-relay floor |
| A2 (MAX across estimators) | ✅ §3 — value cap makes MAX safe; median optional |
| A3 (1-conf vs `n_b` policy) | ✅ §2.3 step 1 — `policy_confs` uniformly on every stop/advance gate |
| A4 (hardcoded incremental-relay floor) | ✅ §3 — source `incrementalrelayfee` from the node for the new RBF path |
| A5 (rebuild without re-checking HTLC unspent) | ◐ partial — §2.3 step 5 only rebuilds on eviction (no per-tick churn); add an explicit "input still unspent?" guard before step 4 rebuild |
| A6 (CPFP 1-out fee accounting) | ✓ non-issue — redeem is single-output by design; the assumption is correct |
| A7 (fund reservation vs `sendtoaddress` fee) | ◐ subsumed by A8 for v2; v1 funding-pricing tidy-up left as its own track |
| **A8 (v2 funding bumpability not guaranteed)** | ✅ §2.4 — build v2 funding ourselves with a guaranteed CPFP anchor so it can always be bumped (it can't RBF) |

A5 is the only partial: the evicted-only rebroadcast removes the churn, but if the
counterparty has spent the HTLC (refund races, or a confirmed replacement), step 4 should
re-check the input is still unspent before rebuilding rather than broadcast a guaranteed
reject. Cheap to add (`get_txout` on the input). A6 is a non-issue (single-output redeem by
design); A7 (funding fee chosen by the node) is left to its own track.

## 6. Review checklist / open questions

- [ ] **`fee_cap_pct` = 100%** (cap at the full value being claimed, Eclair model) — confirm,
  or set a tighter sanity fraction (e.g. 50%)? The absolute `max_feerate_sat_vb` and a
  dynamic min-relay floor still apply underneath either way.
- [ ] **Refund gate anchor** — confirm the refund leg's `M`/`T` mapping (its own timelock) is
  the right anchor for the on/off `action_safe` gate.
- [ ] **A2 (MAX across estimators)** — adopt median, or keep MAX now that the relative cap
  bounds it?
- [ ] **Eviction check** — cheapest reliable signal for "tx no longer in mempool" across the
  Core-RPC + Electrum backends.
