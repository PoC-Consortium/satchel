# Security review of the current protocol implementation

Date: 2026-09-12. Reviewed commit: `9348bae` on `fix/protocol-remediation-reviewed` (PR #237). `origin/master` was fetched and still pointed to the parent, `f670cd9`; the PR branch was the latest implementation. No production source was modified during this review.

## Verdict and triage

**Three confirmed findings: one high-priority recovery failure and two medium-priority issues.** The previous third review correctly verified its requested fixes, but its scenarios did not cover a claim that was initially accepted and subsequently lost a conflict. That distinction exposes both an unattended-refund gap and another route to incorrect completion accounting.

| ID | Severity / priority | Recommendation |
| --- | --- | --- |
| S1 | High / P1 | Fix before relying on unattended swaps: keep the initiator's own refund armed while the first reveal remains unsettled. |
| S2 | Medium / P2 | Fix alongside S1 before merge: reconcile unsettled terminal records when their accepted claim loses a conflict. |
| S3 | Medium / P2 | Fix Corkboard revocation replay before the next public board rollout; maker-side rejection limits impact. |

No signing-key extraction, arbitrary code execution, or new unconditional theft primitive was demonstrated. S1 leaves recoverable funds exposed after the normal refund deadline; S2 conceals an already-realized loss. These are separate effects and should not be described as a proof that the implementation caused every underlying claim/refund race.

## S1 — A losing first reveal permanently suppresses the initiator's automatic refund

**Evidence:** [`engine.rs:9128`](../../pact/libswap/src/engine.rs#L9128), [`engine.rs:3854`](../../pact/libswap/src/engine.rs#L3854), [`engine.rs:4332`](../../pact/libswap/src/engine.rs#L4332). The independent-refund helper exists at [`engine.rs:9460`](../../pact/libswap/src/engine.rs#L9460), but the `RedeemedB` arms do not use it.

After Alice broadcasts her first reveal on B, v1's `RedeemedB` branch only checks/nurses that transaction. V2 immediately returns from `adaptor_keep_moving`. Neither branch reaches Alice's due refund on A when her reveal is evicted or loses to Bob's confirmed refund. V1 keeps reporting `settlement-conflict`; v2 propagates `-25` from the normal rebroadcast path. Reconciliation cannot terminate the pair while A remains live, and the next tick repeats the same branch.

**Reproduced in both protocols:** fund both legs, let Alice's redeem enter B's real mempool without mining it, restart the isolated B node without loading its mempool, let Bob refund B, mine that refund, and advance both clocks beyond T1. After three Alice ticks, her state remains `redeemed_b` and A remains unspent. A manual Alice refund immediately succeeds and confirms. Thus the automatic refund was available and omitted, rather than refused by CLTV or insufficient funds.

The restart models accepted-but-evicted transactions; it is not an assertion that a counterparty can issue RPCs to the victim's node. A stalled/evicted reveal and a competing refund are the preconditions. Once the secret was published, the counterparty may still claim A while Alice's scheduler fails to exercise its now-valid refund. Operator intervention remains possible, but unattended protection has failed.

**Fix:** while the reveal is not final at the required depth, independently check the still-unspent own leg for a due refund. A B lookup error, missing-input rejection, or confirmed competing B spend must not suppress A's refund. Preserve evidence that the secret escaped; never reset this to a pre-reveal state or authorize fresh funding. Define the winner/reconciliation transition when the own refund succeeds.

**Regression requirements:** accepted-but-evicted and accepted-but-conflicted first reveals, both protocols; A refundable/B failed or unreachable; assert automatic refund broadcast and confirmation without manual RPC. Retain the rule against refunding after an irrevocably successful settlement. This implements the spec's existing requirement that the refund remain scheduled until the corresponding redeem confirms (`spec/protocol.md:590–606`).

## S2 — An initially accepted participant claim can still remain falsely Completed after losing

**Evidence:** terminal-state early exits at [`engine.rs:8265`](../../pact/libswap/src/engine.rs#L8265) and [`engine.rs:8393`](../../pact/libswap/src/engine.rs#L8393); v1 conflict detection at [`engine.rs:9798`](../../pact/libswap/src/engine.rs#L9798); v2 normal rebroadcast at [`engine.rs:4332`](../../pact/libswap/src/engine.rs#L4332).

The previous fix covers a missing output before claim creation, immediate claim rejection, and a retained `claim_pending` retry. A successful initial broadcast clears that marker. If the transaction later disappears and the counterparty refunds the claim leg, the ordinary nurse takes a different path:

- V1 detects the foreign spend and requests reconciliation, but `reconcile_driven_v1` immediately marks every `Completed` record reconciled, even when `settled=false`.
- V2's ordinary `adaptor_keep_moving` rebroadcast propagates the inputs-spent error without invoking the new participant rollback guard. Its reconciliation routine also excludes `Completed` unconditionally.

**Reproduced in both protocols:** Alice redeems B; Bob successfully submits his A claim; evict the unconfirmed A claim with a controlled node restart; advance past T1; Alice refunds A and confirms. After three Bob ticks, both implementations retain `state=completed`, `settled=false`, `settlement_loss=false`, and the dead claim txid. V1 emits repeated conflict events; v2 emits repeated `-25` errors. Both legs are conclusively settled, but the corrected role-aware loss matrix is never reached.

**Impact:** incorrect terminal/accounting data, absent loss narration, and permanent retry/watch activity. This is not a finding that Bob can still recover A after Alice's refund confirms; the loss already happened. It is a missing conflict-recovery path that the earlier late-call tests did not exercise.

**Fix:** allow requested reconciliation of unsettled `Completed`/`Refunded` records. Route ordinary nurse conflict failures through the same role-aware handling as immediate/pending failures, preserving the initiator's secret-release evidence. Apply the confirmed winner matrix and retire dead bytes/markers only after the appropriate depth gate. Avoid reopening truly settled records on an unverified single-view hint.

**Regression requirements:** initial broadcast must actually succeed and its pending marker be absent before injecting the conflict. Tick after the refund confirms; assert loss flag, correct terminal state, no own settlement pointer, and no repeated broadcasts. Cover both roles' lost settlements as well as the participant case reproduced here.

## S3 — Replaying a public signed offer restores it after revocation

**Evidence:** [`corkboard/src/main.rs:124`](../../corkboard/src/main.rs#L124), [`main.rs:148`](../../corkboard/src/main.rs#L148), and [`main.rs:228`](../../corkboard/src/main.rs#L228).

`revoke_offer` marks a row revoked, but `post_offer` calls `prune`, which deletes revoked rows before inserting. Replaying the original public, correctly signed offer therefore removes its revocation record and inserts it again. The server assigns a fresh receipt time and TTL; it does not retain a replay-prevention tombstone. Signature verification authenticates the old content but does not establish renewed authorization to publish it.

**Reproduced against the real Corkboard HTTP API:** publish an offer, fetch its signed envelope from the public list, revoke it through the maker, confirm the list is empty, then POST the unchanged captured envelope. The offer appears in the public list again. The replay requires no maker private key.

**Impact and limits:** stale/revoked listings can be republished, impairing listing integrity and causing futile takes. Replays can attribute retained offers to a victim's identity and occupy its offer quota. The maker's separate local offer ledger still rejects revoked offers (`engine.rs:11852` onward), so this does not establish unauthorized funding or acceptance of a revoked trade. Endless extension of client-accepted economic validity was not demonstrated; clients also inspect the signed offer's own validity.

**Fix:** persist revocation identity/tombstones independently of live offer pruning for at least the full accepted replay window; validate signed creation/expiry and cap retention to that window. Reject an old publication whose author/offer identity has been revoked. Preserve legitimate idempotent retries and require a fresh offer identity for intentional republication.

**Regression requirements:** publish → revoke → replay, including intervening prune and restart; an expired publication must not gain a fresh lifetime from server receipt time.

## Other observations

- `spec/protocol.md:598` still says Bob must broadcast before T1 minus one hour, despite the corrected public-secret claim policy elsewhere. This is a documentation inconsistency, not a newly demonstrated code cutoff.
- Previously documented global scan-lock/network stalls, the deep-reorg scan cursor limitation, unresolved ambiguous funding markers, recipient quota abuse, and limited LTC operator diversity remain relevant. This review does not claim to close them.
- Static checks of RPC authentication, envelope identity checks, nonce persistence, CA/TOFU signature verification, and chain-view finality quorum did not establish another bypass in the paths inspected. This is not a formal cryptographic or whole-repository audit.

## Verification and artifacts

- Fresh workspace test run: **241 passed**, exit 0. Existing tests passing does not cover the counterexamples above.
- Five isolated regtest/HTTP probes reproduced the findings: losing accepted participant claims (v1/v2), omitted automatic initiator refunds (v1/v2), and revoked-offer replay. Probe exit 0 means the assertions confirming the vulnerable behavior succeeded, not that the code is secure.
- Reproducer: [`2026-09-12-security-probes.py`](2026-09-12-security-probes.py). It reuses the existing handshake setup but changes the adversarial sequence in memory; no production files are patched. All nodes use private harness ports/datadirs and are torn down by the harness.
- Portable observations: [`2026-09-12-security-evidence.json`](2026-09-12-security-evidence.json). Full local logs: `2026-09-12-security-probes.log` and `2026-09-12-workspace-tests.log` (ignored by Git).
- The first probe attempt needed the cached chain's mock time on restart; another attempt corrected a harness helper's non-returning `start()` call. The final complete five-probe run returned 0. The one relay left by the setup error was stopped using its verified PID and exact temporary database path.
- The full 69-scenario gate, desktop/UI checks, dependency audits, and native-platform checks were **not rerun for this review**. Their prior results are in the third review; no production changes were made here.

These artifacts are local and uncommitted. No PR comment, source fix, or remote change was made as part of this review.
