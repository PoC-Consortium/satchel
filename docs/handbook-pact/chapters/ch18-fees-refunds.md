# Fees, Fee-Bumping & Auto-Refund

Atomic swaps are fee-sensitive in a way ordinary payments are not: a redeem
that fails to confirm before a timelock matures can cost a party its funds (see
the chapter "Timelocks & Action Deadlines"). Pact therefore treats fee-bumping
and auto-refund as first-class, scheduler-driven mechanisms. This chapter
covers how v1 and v2 bump stuck spends — including the important v2 asymmetry
where one path can be bumped and one cannot — and how the engine fires the
timelock refund automatically and safely.

## v1 fee-bumping (RBF)

In v1 **both** the redeem and the refund are RBF-bumpable. Both spends signal
RBF (`nSequence = 0xFFFFFFFD`), and because the v1 keys sign deterministically
(ECDSA), the engine can re-sign a higher-fee replacement unilaterally
(`maybe_bump`, `engine.rs:2915-2984`).

The fee escalation on each bump is roughly +50% (`engine.rs:2915-2984`):

```text
new_fee = old_fee + max(old_fee / 2, MIN_SPEND_FEE_SAT)
```

with `MIN_SPEND_FEE_SAT = 1000` (`swap.rs:38`). If raising the fee further
would push the swept output below `DUST_LIMIT_SAT = 546` (`swap.rs:20`), the
engine stops escalating and **rebroadcasts the existing transaction** instead —
a higher fee that dusts the output would be worse than simply retrying.

## v2 fee-bumping: a split design

v2 is asymmetric, and the asymmetry is load-bearing (spec v2 §8):

| v2 spend | Bumpable? | Why |
|---|---|---|
| Single-key CLTV refund | **Yes**, RBF | single-key, deterministic re-sign |
| Cooperative MuSig2 key-path redeem | **No** | fee sealed into the pre-signed adaptor sighash |

### The refund is RBF-bumpable

The v2 single-key refund (`adaptor_bump_refund`) bumps exactly like v1: same
~+50% escalation, deterministic single-key Schnorr re-sign, RBF sequence. No
interactive ceremony is needed because the refund tapleaf is single-signature
(see the chapter "v2 Taproot/MuSig2 Adaptor Swaps").

### The cooperative redeem is NOT bumpable

The cooperative key-path redeem's fee is fixed at signing time: the fee is part
of the sighash the MuSig2 adaptor session signed, and re-signing would require
re-running the interactive ceremony. The engine cannot raise it after the fact.
Two mitigations make this safe in practice.

**(a) Over-provision the fee at init.** The adaptor redeem feerate is set high
*before* signing so the sealed fee is generous (`engine.rs`,
`adaptor_redeem_feerate`):

```text
adaptor_redeem_feerate = live_6block_estimate × ADAPTOR_REDEEM_FEERATE_MULT(3)
                         clamped to MAX_REDEEM_FEERATE = 500 sat/vB
                         fallback 20 sat/vB if no estimate
                         fixed 2 sat/vB on regtest
```

Tripling the 6-block estimate (capped at 500 sat/vB) buys headroom against fee
spikes between signing and broadcast.

**(b) The CPFP redeem-bump child (v2+).** If the over-provisioned redeem is
*still* too slow, the claimer accelerates it with a child-pays-for-parent
transaction (`adaptor_cpfp_bump`, `engine.rs:1942-1985`):

- The child spends the redeem's **own vout 0** — the claimer's wallet-owned
  sweep output — so it is self-funded and needs no extra inputs.
- The child signals RBF, so the *child* itself can be bumped further.
- Child vsize is `CPFP_CHILD_VSIZE = 150` (`engine.rs`).
- It emits `adaptor-cpfp` / `adaptor-rebroadcast` events.

This is a **plain CPFP** (no `submitpackage` / package relay): the parent
redeem stays relayable on its own, so a normal CPFP child suffices to drag it
through. Proven by `test_adaptor_redeem_cpfp` (and `..._ltc`, the first v2 swap
on litecoind).

> **Note** — The cooperative redeem is not RBF-bumpable, so it is handled by fee
> over-provisioning plus a CPFP child: enough fee is committed up front, and a
> CPFP child can drag the parent through if conditions tighten before the
> deadline. The single-key refund path is always bumpable, so the *funder* is
> never stuck. See the chapter "Network Support, Reorgs & Safety".

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

## Fee constants reference

| Constant | Value | Meaning |
|---|---|---|
| `MIN_SPEND_FEE_SAT` | 1000 sat | floor for any HTLC/leg spend fee |
| `DUST_LIMIT_SAT` | 546 sat | swept output must stay above this |
| `ADAPTOR_REDEEM_FEERATE_MULT` | 3× | over-provision multiplier (v2 redeem) |
| `MAX_REDEEM_FEERATE` | 500 sat/vB | clamp on the over-provisioned feerate |
| `CPFP_CHILD_VSIZE` | 150 vB | the CPFP redeem-bump child |

The v1 escalation step (`+max(old/2, 1000)`) and the v2 over-provision-plus-CPFP
strategy are two answers to the same question — *how do I make sure a
time-critical spend confirms?* — chosen because v1 can re-sign freely and v2's
cooperative redeem cannot.
