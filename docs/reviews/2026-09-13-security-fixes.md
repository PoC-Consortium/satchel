# Settlement recovery and offer replay fixes

Base: `master` / `origin/master` at `3adf847` (merged PR #237). Working branch: `fix/settlement-recovery-and-offer-replay`.

The [security review](2026-09-12-security-review.md) and [independent triage](2026-09-12-security-triage.md) agree on three valid findings. This batch addresses all three, plus the stale participant-claim deadline in the protocol text.

## Changes

- **S1, High:** An initiator in `RedeemedB` checks its own due refund after nursing the reveal, including when nursing fails or a pending send marker encounters a chain-B outage. The record is reloaded first: a reveal finalized during this tick prevents the refund. The refund checks for live funding, uses the existing write-ahead send path, and leaves the record in `Refunded`. v1 retains the published reveal in `reveal_tx:<swap_id>:b` metadata; v2 retains its separate chain-B transaction slot. Neither returns to a state eligible for fresh funding.
- **S2, Medium:** Requested reconciliation now processes unsettled `Completed` and `Refunded` records. The existing role-aware, confirmation-depth-gated matrix adopts the actual settlement winner, clears a losing claim when there is no own settlement, and records settlement loss. A latched `settled` record stays retired. The ordinary v2 nurse now routes inputs-spent errors through the claim-rejection recovery path. Winning v2 transaction data replaces a stale candidate in the relevant slot.
- **S3, Medium:** Corkboard uses signed `created` and `ttl_secs` for expiry, rejects expired publications, and retains revoked rows through their signed validity window. Identical live retries succeed even at quota without renewing expiry. Startup migrates surviving receipt-time rows to signed lifetimes while retaining their revocation flags.
- **Documentation:** Removed the obsolete `T1 − 1 h` deadline from Bob's claim step; the surrounding text already permits claiming a revealed secret while the output remains unspent.

## Regression coverage

Fourteen new scenarios are part of the normal harness sweep: seven each for v1 and v2. They exercise an accepted participant claim losing later, a first reveal losing to a refund, an evicted reveal, chain-B outage with a retained send marker, final and mined-but-shallow reveals, and an accepted initiator refund losing later. Assertions cover automatic broadcast, confirmed outcome, loss flag, winning transaction adoption, retained reveal evidence, cleared send markers, and retirement of settled records.

Three Corkboard tests cover signed validity bounds, live retries at quota without renewal, and revocation surviving pruning, restart, and migration. The reconciliation unit test now distinguishes unsettled terminal records from the settled latch.

Existing settlement-latch scenarios accept either a `settled` nurse event or a `reconciled` event, while retaining their settled-record and watch-retirement assertions. The first sweep caught this obsolete event-only expectation in `DaemonAutopilotSwap`; its log is retained as `2026-09-13-security-merge-gate.log`. The initial shallow-v1 fixture also needed funding confirmations mined before delivery of the funding message; the corrected targeted run passed.

Validation: targeted settlement scenarios passed (14/14), Pact workspace tests passed (241), Corkboard tests passed (6), protocol/Nostr/crier tests passed (11/8/17), affected crates and supporting crates passed clippy with warnings denied, and the UI production build passed. Formatting and `git diff --check` passed.

**Full clean merge gate: PASS, 83/83 scenarios across all nine suites, exit 0.** Command: `python -X utf8 test_runner.py --rebuild-cache` from `pact/harness`, with UTF-8/unbuffered Python output. No filters or skipped suites. Log: [2026-09-13-security-merge-gate-final.log](2026-09-13-security-merge-gate-final.log). Source and test files were unchanged during this final run.

| Suite | Scenarios | Result |
|---|---:|---|
| Framework | 3 | PASS |
| Multi-machine | 1 | PASS |
| v1 | 32 | PASS |
| Rescue | 8 | PASS |
| v2 adaptor | 19 | PASS |
| Nodeless | 4 | PASS |
| Follow | 3 | PASS |
| Takeover | 10 | PASS |
| Upgrade / mixed version | 3 | PASS |

Independent review approved this batch for commit: [review and reproduced verification](2026-09-13-security-fixes-review.md). The pre-existing review, triage, probes, and evidence artifacts are included for traceability. The review's parenthetical description of `ShallowReveal` is a wording error: this scenario asserts that a due refund DOES fire while the reveal is mined but below its confirmation target.

## Limits

- An upgrade cannot recover revocation rows already deleted by an older Corkboard. Expired signed offers are nevertheless rejected. Surviving tombstones count toward the existing storage quotas until expiry, keeping retention bounded; makers who publish and revoke frequently can reach their per-identity quota during that window.
- Independent refund scheduling still follows bounded RPC attempts in the tick; it does not add parallel per-chain scheduling or remove the previously documented chain-backend limits.
- These fixes do not eliminate the underlying post-reveal race: a counterparty knowing the secret can compete for the output. They restore automatic refund attempts and accurate accounting of the confirmed outcome.
