# Funding fee-bump nurse — design & follow-up TODO

**Status:** PENDING (follow-up). Liveness-only, **not** release-blocking.
**Scope:** the one remaining fee-spike gap in the swap engine.

## The gap

Every swap broadcasts three kinds of fee-paying transaction. After a careful
pass over both protocols, exactly **one** of them has no fee-bump strategy:

| Transaction | v1 (HTLC) | v2 (Taproot/adaptor) |
| --- | --- | --- |
| **Funding / lock** | ❌ never bumped | ❌ never bumped |
| Redeem / claim | ✅ RBF (`maybe_bump`) | ✅ CPFP (`adaptor_cpfp_bump`) |
| Refund | ✅ RBF (`maybe_bump`) | ✅ RBF (`adaptor_bump_refund`) |

The **funding/lock transaction is the only never-bumped class** — and it's
unbumped on all four combinations: `{v1, v2} × {maker leg, taker leg}`.
Everything downstream (redeem, refund) is already nursed by the scheduler.

The funding tx is also the *only* wallet-funded action in a swap: redeem,
refund, and the CPFP child all take their fee from the on-chain output being
spent, never from spendable balance. Funding draws the wallet. (This is why the
pre-flight funds gate reserves headroom only for funding — see
`ensure_can_fund`.)

### Why it's not release-blocking

A stuck funding is a **liveness** problem, never a safety one:

- The maker funds leg A first. If it stalls, the taker (who waits for leg A to
  reach `n_a` confirmations before funding leg B) simply never commits — no
  counterparty exposure — and the maker refunds after `T1`.
- The taker funds leg B only after leg A is confirmed. If leg B stalls, the
  maker waits, then both refund (`T1` / `T2`).

In every case the refund timelock protects the funds. Worst case is
**stall → refund**, never a loss. An unconfirmed funding eventually confirms or
is evicted from the mempool (returning the coins). So this is a swap
**completion-rate** improvement, not a fund-safety fix.

## What "spike before vs after broadcast" actually does

- **Before broadcast** (spike between negotiation and funding): already handled.
  `sendtoaddress` / `wallet_send` re-estimate the fee at broadcast, so the lock
  is simply paid at the current market rate (from the wallet). The only failure
  mode is insufficient balance, which the funds gate covers.
- **After broadcast** (spike between funding and confirmation): **this is the
  gap.** The funding was sent at feerate `X`; the market moved above `X`; nothing
  bumps it.

## The design

Bump the funding **per protocol** — basic where we can, advanced only where the
cryptography forces it:

### v1 funding → RBF

- Drive via the wallet's `bumpfee` RPC (the funding is wallet-owned, so this is
  a single call), or an equivalent manual BIP125 replacement.
- RBF changes the funding **txid → outpoint**, so the v1 refund (signed at
  funding time against the old outpoint, spec §6.3) must be **rebuilt and
  re-signed** against the new outpoint.
- **This re-sign is purely local — no counterparty.** The v1 refund is a
  **single-key** spend: the funder's own key on the CLTV timeout branch
  (`build_refund_tx` witness `[sig, pubkey, <empty>, witness_script]`, one
  `SecretKey`). After the bump: re-locate the HTLC output (same address and
  amount, new `txid:vout`), rebuild + re-sign + re-persist the refund.
- RBF is preferred for v1 because it is the **basic, universally-relayed**
  mechanism — no package-relay dependency, no change-output requirement.

### v2 funding → CPFP-via-change

- A child transaction spends the funding tx's **change output** (wallet-owned)
  at a high fee, dragging the funding in as a package. Mirrors the redeem-side
  `adaptor_cpfp_bump` (a child on the redeem's own output) and reuses
  `backend.wallet_sign_send`.
- The funding **txid/outpoint stays unchanged**, which is mandatory: the v2
  funding outpoint feeds the **2-of-2 MuSig2 cooperative-redeem adaptor
  signatures** that have already been exchanged with the counterparty. RBF would
  change the outpoint and invalidate those, and re-doing a MuSig2 round
  **requires the counterparty** — so RBF is impossible for v2.
- **Edge case:** a funding that consumed an exact UTXO (no change output) has
  nothing to CPFP from → it falls back to stall → refund. Acceptable (liveness).

### The asymmetry in one line

> v1's only outpoint-dependent downstream tx is the **single-key refund**
> (re-sign locally) → RBF is fine. v2's is the **2-of-2 cooperative redeem**
> (re-doing needs the counterparty) → CPFP is forced.

Both protocols cover **both** the maker and taker funding legs.

## Bump policy (already agreed)

- **Ceiling:** chase the current market estimate, clamped to
  `min(3 × initial funding feerate, 500 sat/vB)`. The initial feerate is
  recomputable at bump time from `gettransaction` (`fee / vsize`), so **no new
  persisted record field is needed**.
- **Cap constants already exist** in `engine.rs`: `FUNDING_BUMP_FEERATE_MULT = 3`,
  `FUNDING_VSIZE_EST = 250`, `MAX_REDEEM_FEERATE = 500` (reused as the absolute
  backstop).
- **On deadline-miss:** keep trying at the cap and let it stall → refund. No
  proactive early-abort (it would only improve speed; the timelock already
  protects the funds).

## Implementation sketch

1. **Backend:** a method to fetch a wallet tx's fee + vsize + its wallet-owned
   (change) output — e.g. `gettransaction` + decode, identifying the non-HTLC
   output by script.
2. **v1:** in `tick_one`, for a swap whose own funding is unconfirmed, below
   target, and before the fund-margin deadline → `bumpfee`, re-locate the HTLC
   output, rebuild/re-sign/persist the refund, update the stored funding
   `txid:vout`.
3. **v2:** in `adaptor_tick_one`, the symmetric case → build + `wallet_sign_send`
   a CPFP child on the funding's change output.
4. Reuse the existing cap constants and the `cpfp_child_fee` helper.

## Verification

Cannot be meaningfully unit-tested — it needs a **regtest fee-spike
simulation**: broadcast a funding at a low rate, raise the mempool floor, and
watch the bump land before the deadline. Build alongside the local
release/regtest iteration loop.
