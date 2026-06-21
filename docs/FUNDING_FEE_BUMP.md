# Fee-bump policy — unify, parameterize, and expose

**Status:** PENDING (follow-up). Mostly liveness; **not** release-blocking.

**Goal:** the engine bumps stuck transactions in three different places with
three different hardcoded policies. Unify them into **one parameterized policy**.
Different *strategies* per transaction type (funding vs redeem vs refund) are
fine and expected — but the parameters must be **consistent, configurable, and
have sensible defaults**, surfaced all the way up to a **Satchel settings page**.

This started as the "funding fee-bump nurse" (the one transaction with no bump
at all — see [§ Funding nurse](#funding-nurse-the-concrete-gap)); pinning that
down surfaced the broader inconsistency below.

## Current state (the sprawl)

Every swap broadcasts three kinds of fee-paying transaction. Their bump policies
are all **hardcoded** consts scattered across `engine.rs` / `swap.rs`, and use
**three different models**:

| Path | Strategy | Policy (hardcoded) | Feerate ceiling |
| --- | --- | --- | --- |
| v1 redeem | RBF | `+50%/tick` (`old_fee + max(old_fee/2, 1000)`) | none — until output dusts |
| v1 refund | RBF | `+50%/tick` | none — until dust |
| v2 refund | RBF | `+50%/tick` (`adaptor_bump_refund`) | none — until dust |
| v2 redeem (committed) | fixed at init | `2×` live estimate (`ADAPTOR_REDEEM_FEERATE_MULT`) | clamp 500 |
| v2 redeem CPFP | child → market | target = current market (`fee_rate_sat_per_vb`, 1×) | output size; 500 |
| **funding** | **none today** | — (proposed: RBF v1 / CPFP v2) | — |

The three models:

1. **+50%-per-tick RBF escalation** — v1 redeem, v1 refund, v2 refund. A relative
   step with **no feerate ceiling** (bounded only by the output dusting), floor
   `MIN_SPEND_FEE_SAT = 1000`.
2. **Target current market (1×)** — the v2 redeem CPFP child.
3. **Multiple-of-init** — the v2 committed redeem (`2×`), and the proposed
   funding bump (`3×` today), both capped at 500.

Problems:

- **No single ceiling model.** The 500 sat/vB cap (`MAX_REDEEM_FEERATE`) applies
  to the committed redeem + CPFP, but **not** to the RBF escalators (they have no
  feerate cap at all).
- **Drifted multiples.** Funding is `3×` while the redeem it was meant to match
  is now `2×` (they were both `3×` until the redeem was lowered). The funding
  *gate* (`ensure_can_fund`) reserves the same multiple the bump will chase, so
  the two must move together.
- **Magic numbers.** The `+50%` escalation is an inline `/2`, not a named const.
- **Nothing is configurable.** All compile-time consts; no runtime knob, nothing
  surfaced to the user.

## Target: one parameterized policy

Keep a distinct strategy per transaction type — the strategy is *intrinsic*
(you cannot, e.g., RBF a v2 redeem) and so is **not** a user choice. What becomes
configurable is the **numeric policy**, with one shared shape and sensible
defaults.

### Proposed parameters (defaults shown)

```toml
[fee_bump]
max_feerate_sat_vb = 500    # absolute ceiling, EVERY path (incl. the RBF escalators)
min_fee_sat        = 1000   # floor for any single-tx bump

[fee_bump.funding]          # the lock — RBF (v1) / CPFP-via-change (v2)
ceiling_mult = 2            # chase up to min(ceiling_mult × initial feerate, max)

[fee_bump.redeem]           # claim — v1 RBF escalation; v2 committed-at-init + CPFP
committed_mult = 2          # v2: over-provision baked into the adaptor sig (the
                            #     UNATTENDED floor; CPFP chases market beyond it)
step_pct       = 50         # v1: +N% per scheduler tick

[fee_bump.refund]           # RBF escalation, both protocols
step_pct = 50               # +N% per scheduler tick
```

Decisions folded in:

- **One ceiling for all paths:** `max_feerate_sat_vb` now also caps the RBF
  escalators (today uncapped). Belt-and-suspenders against a runaway estimate.
- **Align the multiples:** funding defaults to `2×`, matching the redeem
  `committed_mult` (reconciles the `3×` drift; lowering the bump ceiling also
  lowers the funds-gate reservation — they share the constant).
- **Name the escalation:** the inline `+50%` becomes `step_pct`.
- **Regtest** keeps its fixed low feerate (`REGTEST_REDEEM_FEERATE = 2`) so the
  deterministic e2e is unchanged — a network special case, not a user knob.

### Plumbing

1. **libswap:** replace the scattered consts with a `FeeBumpPolicy` struct on
   `EngineConfig`; the bump sites read it instead of consts. Defaults match the
   table above, so behaviour is unchanged until someone overrides.
2. **pactd:** accept the policy (config section / flags), pass it into the engine
   — same path as the other `EngineConfig` fields.
3. **Satchel:** persist it in `satchel.json` and pass it to pactd on launch (like
   the board URLs / relays); expose it on the **Settings → Fees** page.

### Satchel settings page

A new **Fees** section under Settings (advanced / collapsible — most users never
touch it):

- Absolute max feerate (sat/vB) — the safety ceiling.
- Funding bump aggressiveness (`ceiling_mult`).
- Redeem over-provision (`committed_mult`) — with a note that higher = safer when
  unattended, lower = cheaper.
- RBF escalation step (`step_pct`).

All default to the values above; a "Reset to defaults" affordance. Copy must make
clear these are **safety/cost trade-offs**, not required setup.

## Funding nurse (the concrete gap)

The transaction that has **no** bump at all today — the **funding/lock** — is the
first consumer of the unified policy. It is unbumped on all four combinations:
`{v1, v2} × {maker leg, taker leg}`. It is also the *only* wallet-funded action
in a swap (redeem/refund/CPFP fees come out of the output being spent, never
spendable balance), which is why the pre-flight funds gate reserves headroom only
for funding (`ensure_can_fund`).

**Liveness, not safety.** A stuck funding → stall → refund, never a loss: the
maker funds leg A first and the taker waits for `n_a` confirmations before
committing leg B, so a stalled funding has no counterparty exposure; the refund
timelock returns the funds. So this raises swap **completion rate**, it does not
fix a fund-safety hole — hence not release-blocking.

### Spike before vs after broadcast

- **Before broadcast** — already handled. `sendtoaddress` / `wallet_send`
  re-estimate at broadcast, so the lock is paid at the current rate (from the
  wallet); the only failure is insufficient balance, which the funds gate covers.
- **After broadcast** — the gap. The funding went out at feerate `X`, the market
  moved above `X`, and nothing bumps it.

### Per-protocol strategy

| | v1 funding | v2 funding |
| --- | --- | --- |
| **Bump** | **RBF** (basic) | **CPFP-via-change** (forced) |
| Why | the only outpoint-dependent downstream tx is the **single-key refund** | downstream is the **2-of-2 MuSig2 cooperative-redeem adaptor sigs** |

- **v1 → RBF.** Drive via the wallet's `bumpfee` (the funding is wallet-owned).
  RBF changes the funding `txid → outpoint`, so the refund (signed at funding
  time, spec §6.3) must be rebuilt + re-signed against the new outpoint — but that
  is **purely local, no counterparty**: the v1 refund is a **single-key** spend
  (funder's own key on the CLTV branch; `build_refund_tx` witness
  `[sig, pubkey, <empty>, witness_script]`). RBF is preferred because it is the
  **basic, universally-relayed** mechanism — no package-relay or change-output
  dependency.
- **v2 → CPFP-via-change.** A child spends the funding's **change output**,
  keeping the funding outpoint **unchanged** — mandatory, because that outpoint
  feeds the 2-of-2 adaptor sigs already exchanged with the counterparty; RBF
  would invalidate them and re-doing a MuSig2 round **requires the counterparty**.
  Mirrors the redeem-side `adaptor_cpfp_bump`. Edge case: an exact-UTXO funding
  with no change output can't be CPFP'd → stall → refund (acceptable).

> The asymmetry in one line: v1's outpoint-dependent downstream is **single-key**
> (re-sign locally) → RBF; v2's is **2-of-2** (needs the counterparty) → CPFP.

### On deadline-miss

Keep trying at the ceiling and let it stall → refund. No proactive early-abort
(it would only improve speed; the timelock already protects the funds).

## Implementation sketch

1. **Policy struct** (`FeeBumpPolicy` on `EngineConfig`) replacing the scattered
   consts; every bump site reads it. Defaults = current behaviour (after the
   `2×`/ceiling reconciliation above).
2. **Backend:** a method to fetch a wallet tx's fee + vsize + its wallet-owned
   (change) output (`gettransaction` + decode; identify the non-HTLC output by
   script). The initial funding feerate is then recomputable at bump time
   (`fee / vsize`) — **no new persisted record field needed**.
3. **v1 funding nurse** in `tick_one`: unconfirmed funding, below target, before
   the fund-margin deadline → `bumpfee`, re-locate the HTLC output, rebuild /
   re-sign / persist the refund, update the stored funding `txid:vout`.
4. **v2 funding nurse** in `adaptor_tick_one`: the symmetric case → build +
   `wallet_sign_send` a CPFP child on the funding's change output.
5. **pactd + Satchel** plumbing and the Settings → Fees page.

## Verification

The funding nurse can't be meaningfully unit-tested — it needs a **regtest
fee-spike simulation** (broadcast a funding at a low rate, raise the mempool
floor, watch the bump land before the deadline). Build alongside the local
release / regtest iteration loop. The policy refactor itself (consts →
`FeeBumpPolicy` with identical defaults) is covered by the existing suite.
