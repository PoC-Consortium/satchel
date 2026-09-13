# Review of the S1–S3 fixes (2026-09-13)

Reviewed: the uncommitted working tree on `master` at `3adf847` (merged PR #237), per [the fix note](2026-09-13-security-fixes.md), against [the 2026-09-12 review](2026-09-12-security-review.md) and [its triage](2026-09-12-security-triage.md). No production source was changed by this review.

## Verdict

**Ready to commit.** All three findings are fixed at the right layer, each has direct regression coverage that asserts the recovered outcome rather than the absence of the old symptom, and every gate passes independently here: 241 pact-workspace tests, corkboard 6, clippy clean, and the full nine-suite sweep at 83 of 83 scenarios on a freshly built daemon. The reviewer's own probe, re-run against this tree, now fails its "vulnerable behavior" assertion in the expected direction.

## S1 — initiator refund after a losing first reveal: fixed

`engine.rs` wraps both schedulers. `tick_one` (`8978-8996`) and `adaptor_tick_one` (`3653-3673`) run the existing arm, then, for an initiator still in `RedeemedB` and not settled, reload the record and try the own-leg refund (`try_refund_due(.., "a")` / `adaptor_refund_if_due`) regardless of whether the arm succeeded or failed. The reload matters: a reveal that reached `n_b` during the same tick moves the record to `Completed`, and the refund check is skipped. An inputs-spent error from the arm is routed through the claim-rejection helper first, which only acts on participant claims (`reject_v1_claim`, `928-937`), so an initiator's failed reveal falls through to the refund check rather than being rolled back.

The refund itself keeps the evidence that the secret escaped: v1 stores the reveal bytes under `reveal_tx:<id>:b` before overwriting the single settlement slot (`5494-5501`), v2 keeps its separate chain-B slot. The record ends in `Refunded`, never a pre-reveal state, so no path can fund it again. `refund()` still requires MTP at or past T1 and the output live, and v2's `adaptor_refund_if_due` now defers to reconcile when the own leg is already spent and no exact-value replacement exists (`3605-3629`), rather than erroring every tick.

Coverage: `SettlementRecoveryEvictedReveal`, `InitiatorConflict`, `RevealOutage` (with a retained write-ahead marker and chain B stopped), plus the negative cases `FinalReveal` (no refund once the reveal is final) and `ShallowReveal` (no refund while mined but shallow), for both protocols. `framework/settlement.py` asserts the refund is broadcast by a scheduler tick with no manual RPC, the output is spent, the reveal bytes are retained, the marker is cleared, and the record latches settled.

Semantics note: the refund fires while the reveal is unconfirmed past T1, which is exactly what `spec/protocol.md` §9.5 requires. It does not remove the post-reveal race; a counterparty holding the secret can still compete for the output, and the fix note says so.

## S2 — accepted claim that later loses: fixed

`reconcile_driven_v1` and `reconcile_driven_v2` now early-exit only on `settled || Aborted` (`8318`, `8442`) instead of on any terminal state, so an unsettled `Completed` or `Refunded` record is classified through the role-aware matrix. `v2_adopt_final` and the v1 writer replace a losing local candidate with the actual winner instead of never clobbering (`8280-8302`, `8417-8420`), and the loss case clears the settlement fields and sets `settlement_loss`. The ordinary v2 nurse's inputs-spent error reaches `reject_v2_claim` via the wrapper. The reconcile unit test now asserts that unsettled terminals still owe chain truth and only the settled latch skips classification.

Coverage: `SettlementRecoveryParticipantConflict` (accepted claim, marker already cleared, evicted, counterparty refund confirmed; asserts `refunded / settled / settlement_loss`, all settlement fields cleared, marker gone, and a silent next tick) and `LostRefund` (the initiator's refund loses to the participant's claim; asserts `completed`, the original reveal adopted as the winner, no loss flag), both protocols. Re-running the reviewer's probe here produced `state=refunded, settled=true, settlement_loss=true, retains_dead_claim=false` and an assertion failure on the old expectation.

## S3 — revoked-offer replay: fixed

`corkboard/src/main.rs`: `offer_window` (`45-63`) derives `created`/`expires` from the signed body, rejects a missing or future creation time, a zero TTL, and an already-expired publication, and caps TTL at seven days. `prune` no longer deletes revoked rows (`170`); they live as tombstones until their signed expiry. `post_offer` checks the existing row before quotas (`199-215`): a revoked id is refused, a different envelope or identity under the same id is refused, and a byte-identical live retry succeeds without renewing expiry. `open_db` migrates receipt-time rows to signed lifetimes and keeps their revocation flags.

Coverage: three unit tests (validity bounds and non-renewal; live retry at quota without renewal; revocation surviving prune, restart and migration, and rejection of an expired publication). Acknowledged limits are accurate: rows an older board already deleted cannot be recovered, and tombstones count toward the per-identity quota until expiry.

## Documentation

`spec/protocol.md:598` no longer states the T1−1h cutoff.

## Independent verification

| Check | Result |
|---|---|
| pact workspace tests | 241 passed (one fixture flake, see below) |
| corkboard tests / clippy | 6 passed / clean |
| pact clippy `-D warnings` | clean |
| Reviewer's probe against this tree | first probe fails its vulnerable-behavior assertion (S2 v1 now reconciles to a loss); script stops there by design |
| Full regtest sweep, all nine suites | 83 scenarios, exit 0 |
| Author's `2026-09-13-security-merge-gate-final.log` | single invocation, 83 passed, 14 settlement-recovery cells |

Flake note: `legacy_slow_mempool_still_reaches_block_scan` in `pact/libswap/tests/core_discovery.rs` failed once in my run while the harness was building alongside, then passed three times in isolation. Its fixture server uses a 3-second read timeout with `unwrap` on the accept side, so heavy concurrent load can kill the server thread before the request arrives. None of the files involved changed in this batch; it is a test-robustness item, not a code defect.

## Minor notes, not blocking

- The refund-after-tick wrapper runs a `store.get` on every tick for an initiator in `RedeemedB`; negligible, but it could be skipped when the inner result already transitioned the record.
- The reviewer's probe script asserts the vulnerable behavior and therefore cannot serve as a regression suite as-is; the harness scenarios cover that role.
