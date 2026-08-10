# Post-mortem: v1 funding orphaned by a sibling swap's RBF bump (2026-08-09)

**Status:** FIXED — by PREVENTION, deliberately without the cure arms (design
decision 2026-08-10): swap fundings now spend CONFIRMED coins only (wallet-btcx
confirmed-only selection + Core `send` minconf=1; v2's `wallet_build_funding`
too), so the orphanable parent/child shape is unrepresentable; a funding short
on confirmed coins while own pending change covers it is QUEUED
(`funding-queued`, typed `FundingQueued`) and survives the C8 pre-funding
timeout while the §7.4 fund window is open; the funding nurse refuses to
RBF-replace a tx with own-wallet descendants
(`funding-bump-skipped-descendants`, both backends) — protecting ordinary
sends a user chains on funding change, the one shape hard-P2 permits. Fix-plan
items 3/4 (re-fund heal + abort unblock) were dropped on purpose: with
prevention, the wedge state cannot form, and the manual rewind recipe below
remains the documented fallback. The chain-aware/CPFP escalation (open
question 1) was rejected — it would need cross-swap wallet-level machinery in
a strictly per-swap engine; queueing bounds the cost at ~one block of latency
against a §7.4 window measured in hours. Open question 2 (v2 exposure) was
audited REAL and is closed by the same confirmed-only rule in
`wallet_build_funding`.
**Severity:** no funds at risk, but a live trade is silently lost and the swap record
wedges permanently (unabortable, unrefundable, unfixable through the RPC surface).

## Summary

When two v1 swaps fund from the same wallet in close succession, the second
funding can spend the **unconfirmed change** of the first swap's funding
(bdk coin selection treats own unconfirmed change as spendable). The first
funding is **bump-eligible**: the funding fee nurse RBF-replaces it while it is
unconfirmed. The replacement invalidates the first funding's change output —
and with it the second swap's funding, which chains on that output. The second
funding vanishes from every mempool and from the wallet's canonical view. The
engine never notices: the record stays `funded_a` pointing at a txid that no
longer exists anywhere, the progress dock freezes at "your lock confirming
0/n", and the counterparty gives up at their 25%-of-window deadline.

This is a *sibling* of the #229 stale-pointer bug, but strictly worse: #229's
funding still existed on chain under a new txid (heal = re-adopt by spk); here
the funding **never confirms under any txid** — there is nothing to re-adopt.

## Timeline (2026-08-09, UTC; mainnet, machine M-0fa9)

| time | event |
|---|---|
| 18:47 | swap `0e8b0900` taken (50k sat BTC leg) |
| 18:56 | `0e8b0900` funded via `dd55c9f5…` @ 1.122 sat/vB; change 1,673,040 sat unconfirmed |
| 19:13 | swap `60e58838` taken (50k sat BTC leg, same counterparty) |
| 19:18 | `60e58838` auto-funded via `f0f3282b…` — sole input **`dd55c9f5:0`, the sibling's unconfirmed change** |
| 19:22 | funding nurse RBF-bumps `dd55c9f5` → `223ecf92…` (1.122 → 2.122 sat/vB). `dd55c9f5` replaced ⇒ **`f0f3282b` orphaned, permanently invalid** |
| 19:29 | `223ecf92` confirms (block 961765). `0e8b0900` healthy; `60e58838` zombie at "0/6" |
| 19:21+ | only trace in logs: `electrum_client … "missing transaction"`; **zero** log lines for `60e58838` after its auto-fund |
| 20:07 | manual remediation (below); fresh funding `e8739834…` broadcast, same HTLC spk |
| 20:03–20:08 | **the same shape recurred immediately**: third swap `4e499b8b` funded via `4a10674a` @1.12, nurse-bumped 27 s later → `85e0599b` @2.14 (bookkeeping repointed correctly, #229 fix works); the remediation funding `e8739834` then chained on `85e0599b`'s unconfirmed change — same hazard, resolved only because both confirmed together |

Evidence: raw txs recovered from the bdk store (`bdk_txs` keeps non-canonical
txs). `f0f3282b` spends `dd55c9f5:0`; `223ecf92` spends exactly `dd55c9f5`'s
inputs (`a580af47:0` + `f99460f4:0`) — a straight BIP125 replacement.

## Why the engine can't see or fix it (all paths audited)

- **Nurse** (`maybe_bump_funding_v1`): dead outpoint reads as "nothing to
  bump" → silent.
- **Heal** (`maybe_resync_funding_v1`, engine.rs ~8853): requires a live HTLC
  found by spk; finds nothing → `None` ("spent or invisible — not ours to
  judge here").
- **Reconcile** (`reconcile_driven_v1`, ~7443): `(Unfunded, Unfunded)` is
  "still in flight — the drive arms take it from here" → marks reconciled, no
  state change. Post-T1 this loops forever via `try_refund_due` →
  `locate_funding` = `None` → `request_reconcile` → no-op. **The zombie never
  terminates.**
- **`fund` RPC** (~4270): initiator gate `state == Accepted` → refuses in
  `funded_a`. (Everything *after* the gate would do the right thing:
  `locate_funding` → miss → leg classifies `Unfunded` → fresh `wallet_send`.)
- **`abort` RPC** (~11157): guard is pointer-based (`htlc_a_txid.is_some()`)
  → "our HTLC is funded — use refund instead". But refund has nothing to
  spend. Every exit is closed.
- **Rewind-to-`accepted` alone doesn't work**: the C8 pre-funding stale-abort
  (~8311, `PRE_FUNDING_TIMEOUT_SECS` = 15 min) matches **before** the
  `(Initiator, Accepted)` auto-fund retry arm (~8386) and aborts the record.

## Live remediation used (the "unwedge" recipe)

Safe because the wallet never lost anything (the orphan's inputs revert to the
sibling's change) and `fund()` is idempotent (it adopts an on-chain funding
if one exists rather than double-funding).

1. Backup `merchants/m1/pact.sqlite` (sqlite backup API — safe against the
   live writer; `swaps` rows are single JSON blobs).
2. Rewrite the record in place: `state` → `"accepted"`, `htlc_a_txid` /
   `htlc_a_vout` / `htlc_a_height` / `refund_tx_hex` → `null`, and
   **`created_at` → now** (defuses C8).
3. `pact-cli tick` → the retry arm fires `fund()`: fresh funding to the
   **same HTLC script** (counterparty's spk-watch needs no message), pointer
   persisted, refund re-signed. Verified in mempool within seconds.

Participant twin (untested, for completeness): state `funded_b` with a dead
leg-b pointer → rewind to `funded_a` + clear leg-b pointer/refund; the
`(Participant, FundedA)` retry arm re-funds each tick and its §7.4 gate
auto-aborts cleanly if past the fund deadline (no `created_at` trick needed).

If found too late to re-fund (initiator near/past T1): same rewind but leave
`created_at` stale — C8 then cleanly aborts + tombstones on the next tick.

Ops detection recipe: `swapprogress` frozen at `confs: 0` **and** the swap's
funding txid **absent from `listtransactions <coin>`** (a healthy unconfirmed
funding is always present; bdk drops orphans from the canonical view; on a
Core-wallet coin the conflicted tx shows negative confirmations instead).
Log signature: an `auto-fund` line with no nurse/heal lines after it, plus
electrum "missing transaction" warnings.

## Fix plan (tomorrow)

Belt and suspenders, smallest-risk first:

1. **Nurse: descendant-aware bump.** Before RBF-replacing funding X, check
   the wallet for own txs spending X's outputs. If any exist, skip the bump
   and emit a `funding-bump-skipped-descendants` event (simplest safe
   behavior; re-chaining children onto the replacement's change is possible
   but much more machinery for a rare case).
2. **Coin selection: don't chain swap fundings on bump-eligible parents.**
   When building a v1 funding, mark unconfirmed change of *live-swap funding
   txs* unspendable (bdk `unspendable` set). Ordinary sends can keep the old
   behavior.
3. **Heal the wedge shape.** In `(Initiator, FundedA)` (and participant
   `FundedB` for leg b), after `maybe_resync_funding_v1` returns `None`:
   if the stored pointer is dead **and** `locate_funding` is `None` **and**
   the leg classifies conclusively `Unfunded` **and** the wallet's canonical
   view no longer contains the funding txid → the funding was invalidated
   pre-confirmation. Re-fund in place (rebuild via the `fund()` internals,
   repoint, re-sign refund, relay a fresh `funded` envelope) while safely
   inside the deadline; otherwise clear the pointer and abort cleanly.
4. **Unblock manual exits.** `abort`'s guard should treat a provably-dead,
   conclusively-unfunded pointer as "nothing committed" (same evidence bar as
   3) instead of refusing on `htlc_a_txid.is_some()`.
5. **e2e**: two swaps on one wallet; force the second funding to chain on the
   first's unconfirmed change; nurse-bump the first; assert the second swap
   heals (re-funds under the new arm) and completes. Plus a wedge-shape cell
   asserting the abort path when past deadline.

## Open questions

- Should the nurse's skip (fix 1) escalate — e.g. CPFP instead of RBF when
  descendants exist — so an underpriced parent still converges? (CPFP on the
  change output doesn't invalidate children.)
- v2 exposure: adaptor funding uses reserved inputs (`adaptor_cancel_built_leg_b`
  machinery) — audit whether the same chain-on-unconfirmed-change shape is
  possible there.
