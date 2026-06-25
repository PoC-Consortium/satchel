# Post-mortem — live mainnet swaps `8cb7…` & `0beb…` (2026-06-25)

**Author:** analysis by Claude (Opus 4.8), driven by Johnny
**Scope:** two live `pact-htlc-v1` cross-chain atomic swaps (BTCX ⇄ BTC), traded against
the same counterparty (`b8b23cd8…2647`).
**Severity:** No loss of funds, no atomicity break. **Fee overpayment** (liveness/UX bug)
on the BTC-redeem leg. One confirmed root-cause bug + several latent issues found in audit.

---

## 1. Executive summary

Both swaps settled correctly and atomically — both counterparty `htlc_a` outputs are spent,
both of our `htlc_b` redeems are mined. The `redeemed_b` ("active") swap is **functionally
complete on-chain**; pactd simply hasn't relabelled it `completed` because our BTC redeem is
only 2/`n_b=6` confirmations deep in our own view (the counterparty already claimed the BTCX).

The "weird Bitcoin-QT activity" Johnny noticed — a new incoming transaction every ~30–60s —
was **pactd RBF-replacing its own BTC redeem on a fixed 30-second timer**, escalating the fee
+50% every tick with **no reference to the actual fee market and no rate limit**. Each
replacement pays our own wallet, so Core shows each as a fresh incoming tx.

The fee escalation is driven by *inter-block timing ÷ tick interval*, not by fee pressure.
On a quiet mainnet mempool (current `estimatesmartfee 6` ≈ **1 sat/vB**), our redeems were
ratcheted to **159 and 358 sat/vB** before a block happened to include one — a **22×–50×
overpay**. The same buggy code path runs on the counterparty's BTCX-redeem leg, but it
escaped lightly because BTCX blocks confirmed its redeem within a few ticks.

---

## 2. The two swaps

| | DONE swap | "Active" swap |
|---|---|---|
| swap_id | `8cb703b54ddc760d` (index 0) | `0beb7760664e0842` (index 1) |
| pactd state | `completed` | `redeemed_b` (on-chain: settled) |
| our role | initiator (Alice) | initiator (Alice) |
| we give → get | 550 BTCX → 0.00206250 BTC | 725 BTCX → 0.00304500 BTC |
| `n_a` / `n_b` | 10 / 6 | 10 / 6 |
| T2 / T1 | 1782387916 / 1782431116 | 1782388492 / 1782431692 |

Both are the classic 2-chain hash-timelock swap: we lock BTCX in HTLC-A (refund after T1),
the counterparty locks BTC in HTLC-B (refund after T2 < T1); we reveal the preimage to take
the BTC (RedeemedB), which lets the counterparty take the BTCX. `T2 < T1` guarantees we
always hold the last move.

---

## 3. Full fee accounting (both legs, both swaps)

> "Healthy" = priced near market and confirmed with ≤1 bump. "BUG" = the §4 escalation storm.

### DONE swap `8cb7…` (550 BTCX ⇄ 0.00206250 BTC)

| Leg | Owner | tx | feerate | abs fee | RBF txs | verdict |
|---|---|---|---|---|---|---|
| HTLC-A fund (lock BTCX) | us | `770f94…` | 2.0 sat/vB | 27,158 sat BTCX | 1 nurse bump | healthy |
| HTLC-B fund (lock BTC) | cpty | *(unobservable — no txindex)* | — | — | — | n/a |
| **HTLC-B redeem (claim BTC)** | **us** | `3acc65…` | **358 sat/vB** | **51,255 sat BTC** | **10 txs** | **BUG** |
| HTLC-A redeem (claim BTCX) | cpty | `0dc06b…` | 10.8 sat/vB | 1,550 sat BTCX | 1 tx | healthy |

Our BTC redeem fee = **24.8%** of the 206,250-sat trade value.

### "Active" swap `0beb…` (725 BTCX ⇄ 0.00304500 BTC)

| Leg | Owner | tx | feerate | abs fee | RBF txs | verdict |
|---|---|---|---|---|---|---|
| HTLC-A fund (lock BTCX) | us | `5448d1…` | 2.0 sat/vB | 36,648 sat BTCX | 1 nurse bump | healthy |
| HTLC-B fund (lock BTC) | cpty | *(unobservable — no txindex)* | — | — | — | n/a |
| **HTLC-B redeem (claim BTC)** | **us** | `8dde4e…` | **159 sat/vB** | **22,780 sat BTC** | **8 txs** | **BUG** |
| HTLC-A redeem (claim BTCX) | cpty | `7608126…` | 31.5 sat/vB | 4,500 sat BTCX | ~4 txs | mild |

Our BTC redeem fee = **7.5%** of the 304,500-sat trade value.

**Reading:** Only the **BTC-redeem (claim) leg on our side** was badly overpriced. Our
funding legs were healthy (the funding nurse works correctly). The counterparty's BTCX-redeem
legs ran the *same buggy code* but escaped with 10.8 and 31.5 sat/vB because BTCX blocks
confirmed them within a few ticks.

> **Did the counterparty overpay?** **No, not materially.** Their worst leg was 4,500 sat
> BTCX (31.5 sat/vB) on the active swap; the done swap's BTCX redeem was a clean single tx at
> 10.8 sat/vB. They were never hit by the storm. But they are exposed to the identical bug —
> on any swap where their claim chain has a slow next block, they would overpay exactly as we
> did. We could not observe their BTC-*funding* fee (BTC node has no txindex and it isn't a
> wallet tx of ours); their funding is presumed fine since our redeem of it succeeded.

---

## 4. Root cause #1 (confirmed) — market-blind, rate-unlimited redeem fee escalation

### Evidence — the RBF chain of the active swap's BTC redeem (`walletconflicts`)

8 redeem transactions, one every ~31 seconds (= `--tick-secs 30`), each +50%:

| Δt (s) | abs fee (sat) | feerate |
|---|---|---|
| +0 | 1,000 | 7 sat/vB ← initial redeem (floored) |
| +32 | 2,000 | 14 |
| +31 | 3,000 | 21 |
| +30 | 4,500 | 31 |
| +31 | 6,750 | 47 |
| +31 | 10,125 | 71 |
| +31 | 15,187 | 106 |
| +31 | 22,780 | 159 ← mined |

8 txs in **217 s**, **7 → 159 sat/vB (~22×)**. The done swap did the same to **10 txs /
358 sat/vB**. The escalation level is set by how long the next block took, *not* by fee
demand: a block at tick 2 would have settled it at 14 sat/vB.

### Mechanism

In state `RedeemedB`, every scheduler tick calls `maybe_bump()`:

- `pact/libswap/src/engine.rs:3092` — the `RedeemedB` arm calls `maybe_bump` whenever the
  redeem has `< n_b` confirmations.
- `pact/libswap/src/engine.rs:3279` `maybe_bump()` — computes
  `new_fee = escalate(old_fee, step_pct=50, vsize)` and rebroadcasts. **It never reads the
  mempool / fee market.** Only stop conditions: the tx confirms, or it hits the 500 sat/vB
  ceiling.
- `pact/libswap/src/fee_policy.rs:147` `escalate()` — blindly multiplies the previous fee by
  1.5 (with a 1,000-sat floor for the first steps).

### Contrast — the funding nurse does it right

`maybe_bump_funding_v1()` (`engine.rs:3371`) reads the live market and bails when already
priced enough:

```rust
let market = backend.fee_rate_sat_per_vb()?;
let target = market.min(ceiling).min(reservation_mult * old_feerate);
if target <= old_feerate { return Ok(None); }   // already paying enough → no-op
```

The redeem/refund bumper is missing **both** halves: it never fetches `market`, and it has no
"already paying enough" early-return. That asymmetry is the whole bug.

### Why it only bit our BTC leg

- Funding legs → handled by the correct nurse → 2 sat/vB, fine.
- Counterparty's BTCX-redeem leg → same buggy escalator, but BTCX produced a block within a
  few ticks → mild.
- Our BTC-redeem legs → caught in longer BTC inter-block gaps (~10 min) → 8–10 ticks of +50%
  → 159 / 358 sat/vB.

Note: once the redeem is mined (1 conf), the spent input makes any replacement a rejected
double-spend, so the *new-tx* storm stops at first confirmation — but pactd keeps calling
`maybe_bump` every tick from 1→`n_b` confs, emitting rejected broadcasts (harmless noise).

**The same market-blind `escalate()` drives two more sites** that this incident didn't
exercise but which share the identical bug: the **v1 refund** (`maybe_bump`, same function)
and the **v2 adaptor refund** (`adaptor_bump_refund`, `engine.rs:2317`). A v2 redeem is
safe (it CPFP-bumps toward market, `adaptor_cpfp_bump`), but it rebroadcasts its parent
every tick. The consolidated fix across all sites is in [`fee-bump-design.md`](fee-bump-design.md).

---

## 5. Additional findings (code audit of the full swap flow)

Ranked. (#1 below is the most valuable structural fix — it removes the *reason* the buggy
escalator exists.)

| # | Sev | Where | Issue |
|---|---|---|---|
| A1 | **High** | `engine.rs:2666,2736` (redeem), `2598,2832,3465` (refund); `swap.rs:42` | **Initial v1 spend is priced off `estimatesmartfee 6`.** On a quiet mempool that's ~1 sat/vB → floored to the 1,000-sat min → an under-priced first broadcast that *forces* the §4 escalator to take over. The v2 path already prices its first broadcast at `live × committed_mult` clamped; the v1 redeem/refund should do the same, with a tighter 1–2 block conf target. |
| A2 | Med | `chain.rs:1013` | `MultiBackend::fee_rate_sat_per_vb` returns the **MAX across backends**, with **no value cap underneath it**. *Revised after research:* eigenwallet deliberately takes the MAX of Electrum + mempool.space "to ensure lock txs can always be published" — but only because it caps the result at 20% / 100k sat. Eclair caps at the value-at-risk. **So MAX is fine; MAX *without a cap* is the hazard** — and we have no cap. Fix is the value-at-risk cap (see design doc), after which MAX is defensible; optionally switch to median. |
| A3 | Med | `engine.rs:3099` (participant redeem-A), `3112` (refund) vs `3085` (redeem-B `>= n_b`) | **Inconsistent reorg policy.** Bob's chain-A redeem and refunds stop nursing at `tx_confirmations >= 1`, while our redeem-B correctly waits for full `n_b` ("a shallow redeem can still reorg away"). Bob's redeem is equally reorg-exposed; a refund reorged out after being marked terminal is never re-driven. Use `confirmations_for` uniformly. |
| A4 | Low | `fee_policy.rs:153` | BIP125 Rule-4 floor hardcodes `incremental_relay = vsize` (= 1 sat/vB). If a node sets `incrementalrelayfee` above 1 sat/vB, every replacement is rejected as "insufficient fee" and the escalator loops emitting rejects each tick. Source it from `getmempoolinfo.incrementalrelayfee`. |
| A5 | Low | `engine.rs:3314-3347` | The bump path rebuilds + rebroadcasts the redeem/refund **without re-checking the HTLC is still unspent** (unlike `refund()` M7 guard at 2810 and `fund()` reorg recheck). Not dangerous (witness can't be forged) but wastes an RPC and emits a misleading `fee-bump` event every tick. Gate on a quick `get_txout`. |
| A6 | n/a | `engine.rs:2133` | *Non-issue by design.* CPFP fee accounting `amount − output[0]` relies on the redeem being single-output — which is guaranteed by design, so the assumption is correct. Listed only to keep the invariant explicit. No action. |
| A7 | Note | `engine.rs:715-760` + `chain.rs:374` | Fund-time reservation basis (`reservation_mult × live`, floored at 20) differs from what `sendtoaddress` actually pays (the node's own wallet fee policy). Reserve and broadcast should share one feerate (e.g. `send`/`fundrawtransaction` with explicit feerate). *Subsumed by A8 for v2.* |
| A8 | **Med** | `engine.rs:1653` (`adaptor_fund`) + `2232` (`maybe_bump_funding_v2`) | **v2 funding bumpability is not guaranteed.** v2 funds via the node wallet's `sendtoaddress` and **cannot RBF** (the txid is fixed by the exchanged MuSig2 adaptor sigs), so its *only* bump path is CPFP on the funding's change output. But the node's coin selection may produce **no change** (exact-UTXO) or sub-dust change → the funding is **unbumpable**. If it went out underpriced and fees then rise, it can't be accelerated → stalls → forced refund (swap lost; funds safe). We can't even *predict* bumpability, since the node owns coin selection. **Fix:** build the v2 funding ourselves (`fundrawtransaction`, explicit feerate) with a guaranteed wallet-owned anchor/change output ≥ a CPFP-viable minimum — making bumpability deterministic. Subsumes A7 for v2. |

**Checked and OK:** §7.4 margin arithmetic (`action_safe` strict `<`, `deadline_clock` max,
`tip_median_time_min` for refund readiness); `-27`/`is_already_broadcast` idempotency (no
double-broadcast strand); funding nurse market gating; `wallet_send` sets BIP125-replaceable;
offer/structure validation enforces `T2 < T1` and margin room.

---

## 6. Impact assessment

- **Funds safety:** intact. Both swaps atomic, both legs settled, no double-spend or stranded
  HTLC. The `redeemed_b` swap is fully settled on-chain (counterparty already took the BTCX in
  block 38280); it will auto-flip to `completed` at 6 confs.
- **Financial impact (us):** ~74k sat BTC overpaid across the two redeems vs. a competent
  market-priced redeem (~1–2k sat each) — i.e. our BTC-claim legs cost 7.5% and 24.8% of trade
  value instead of <1%.
- **Counterparty:** unharmed; modest BTCX-redeem fees (1,550 / 4,500 sat). Exposed to the same
  bug on future swaps.
- **UX:** the per-tick replacement storm is alarming to anyone watching their node wallet, and
  looks like a malfunction (it is).

---

## 7. Recommendations (prioritized)

> The full remediation — one bump strategy unifying all six sites (v1+v2), informed by
> LND's `LinearFeeFunction`, Eclair's `ReplaceableTx*`, and eigenwallet's `estimate_fee` — is
> specified in **[`fee-bump-design.md`](fee-bump-design.md)**. Summary below.

1. **Add a value-at-risk cap (the single most important guardrail; was missing entirely).**
   Every reference caps the absolute fee at the value being claimed (Eclair: the full amount
   at risk; eigenwallet: 20% + 100k sat; LND: a value-proportional budget). We had only an
   absolute 500 sat/vB ceiling. A `fee_abs ≤ value_being_claimed` cap *alone* bounds both
   incidents here. (Decision: cap at 100% of the claim — Eclair's model.)
2. **Fix the escalator to mirror the funding nurse (root cause).** Replace the market-blind
   geometric `escalate()` with a market-tracking target, **urgency scaled by §7.4 deadline
   proximity** (LND/Eclair both scale feerate by blocks-to-deadline), and an "already paying
   enough → no-op" gate. Bump on **new blocks**, not the 30s wall-clock tick. (Addresses §4,
   v1 redeem/refund + v2 refund.)
3. **Price the first broadcast correctly (A1).** Build the initial v1 redeem/refund at a
   competitive feerate (like v2's `committed_mult`), with a *dynamic* min-relay floor, not the
   flat 1,000-sat constant — so escalation is a rare safety net, not the norm.
4. **Value cap makes MAX-across-estimators safe (A2).** With the cap in place, MAX is
   defensible (eigenwallet does exactly this); median is an optional further refinement.
5. **Unify confirmation policy (A3)** — `confirmations_for`, not bare `1`, on every
   stop-nursing / completion gate (also stops the post-`§4` "bump a 1-confirmed tx" rejects).
6. **Immediate mitigation (no code change):** lower `max_feerate_sat_vb` via `setfeepolicy`
   on the live node to cap the worst case, and/or raise `--tick-secs` so escalation is slower.
   (Neither current swap needs intervention.)
7. **Guarantee v2 funding bumpability (A8).** Build the v2 funding ourselves with an explicit
   feerate and a guaranteed CPFP anchor, so an underpriced v2 funding can always be bumped
   (it can't RBF). Subsumes A7 for v2.
8. Follow-ups: A4/A5 are folded into the design doc's new RBF path; A6 is a non-issue
   (single-output redeem by design); A7 (v1 funding fee chosen by the node) is a minor
   pricing track. See design doc §5 coverage.

---

## Appendix — data sources & limitations

- Live `pactd` (mainnet) at `127.0.0.1:9737`, data-dir
  `…\AppData\Local\org.pocx.satchel\pactd`. `--tick-secs 30`, `--auto-fund`.
- BTCX node `127.0.0.1:8332` (`/wallet/Trading`), height 38293, mempool empty,
  `estimatesmartfee 6` ≈ 1.0 sat/vB. **No txindex** → counterparty BTCX redeems found by
  block scan (blocks 38255 `0dc06b…`, 38280 `7608126…`).
- BTC node `127.0.0.1:9332` (`/wallet/Johnny`), height 955282, `estimatesmartfee 6` ≈
  1.0 sat/vB, mempool ~83k txs at 0.1 sat/vB minfee. **No txindex.**
- RBF chains reconstructed from wallet `walletconflicts`.
- **Limitations:** the counterparty's BTC *funding* fee is unobservable (no txindex, not our
  wallet tx). All counterparty figures are the on-chain claim legs only. Fee figures use each
  tx's actual `vsize`.
