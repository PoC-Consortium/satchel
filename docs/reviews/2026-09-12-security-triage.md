# Triage of the 2026-09-12 security review

Assessed 2026-09-13 against `9348bae` on `fix/protocol-remediation-reviewed` (PR #237, open, base `master` at `f670cd9`). Source: [the review](2026-09-12-security-review.md) and its [probes](2026-09-12-security-probes.py). This is an assessment only; no code was changed. Each claim was traced in the source, and the reviewer's five probes were re-run here (result recorded at the end).

## Verdict

**All three findings are valid. None is a false positive.** S1 is the one that matters for funds and should block merging PR #237; S2 and S3 are small and belong in the same fix batch because S3 is a regression the batch itself introduced.

| ID | Reviewer | Ours | Fix before merging #237? |
|---|---|---|---|
| S1 | High / P1 | **High** | Yes |
| S2 | Medium / P2 | **Medium** | Yes, same batch (shares the reconcile change with S1) |
| S3 | Medium / P2 | **Medium** (regression from the N13 change) | Yes, small |

## S1 — a losing first reveal suppresses the initiator's automatic refund. Valid, High.

Confirmed in the source. The v1 `(Initiator, RedeemedB)` arm (`engine.rs:9128-9153`) does three things only: book `Completed` at `n_b` depth, wait while mined-but-shallow, or nurse the unconfirmed reveal. Nursing finds the foreign spend (Bob's refund of B), emits `settlement-conflict`, and requests reconcile. Nothing in that arm ever calls `try_refund_due(rec, "a")`; the independent-refund helper `refund_after` (`9460`) is wired only into the `FundedA` and `FundedB` arms (`9046`, `9123`). Reconcile cannot resolve the pair either: with A still live and B spent, the v1 matrix hits `(Spent(_), _) | (_, Spent(_)) => return Ok(None)`, so the record sits in `RedeemedB` forever. v2 is the same shape: `(Initiator, RedeemedB)` (`3854`) goes to `adaptor_keep_moving`, which re-broadcasts the dead claim and propagates the -25 (`4332`) before any refund consideration; `adaptor_refund_if_due` is reached only from the pre-Signed branch (`3755`).

Why High rather than Medium: this is not only an accounting gap. Alice's reveal published `s` the moment it entered a mempool. The N4 change in this very batch makes Bob's engine claim A with a public secret at any time. In the race where Alice's reveal is evicted or loses to Bob's refund of B, Bob ends up holding B and able to take A, and the only thing standing between Alice and losing A is her own refund of A after T1, which the scheduler never attempts. The reviewer's probe proves the refund is valid and broadcastable at that moment (manual refund succeeds immediately). The spec already requires this: "a refund MUST stay scheduled until the corresponding redeem is confirmed" (`spec/protocol.md:604-606`).

Fix shape (the reviewer's is right): while the reveal is not final at `n_b`, the `RedeemedB` arms must also check the own leg for a due refund, independently of chain-B errors, missing-input rejections, or a confirmed competing B spend; use `refund_after` there the way the funded arms do. A refund that succeeds must leave the record in a state that records the secret escaped (never back to a pre-reveal state, never eligible for fresh funding), and the reconcile matrix needs the `(A Refund by us, B Refund by them)` and `(A Refund, B Redeem-lost)` outcomes defined. Regressions: accepted-but-evicted and accepted-but-conflicted first reveals, both protocols, asserting the automatic refund broadcast and confirmation with no manual RPC, plus the existing rule that no refund fires after an irrevocably successful settlement.

## S2 — an accepted participant claim that later loses stays `Completed`. Valid, Medium.

Confirmed. `reconcile_driven_v1` (`8265-8271`) and `reconcile_driven_v2` (`8393-8399`) return early for any `Completed`/`Refunded`/`Aborted` record, regardless of `settled`. So the `(Participant, Completed)` nurse arm sees the foreign spend, emits `settlement-conflict`, requests reconcile, and reconcile immediately marks the record reconciled without touching it. v2 additionally propagates the -25 from `adaptor_keep_moving` without going through the `reject_v2_claim` rollback added for the immediate and pending-retry paths. The third review's late-claim scenarios covered a claim refused up front and a retained pending marker; they did not cover a claim that was accepted, cleared its marker, and lost later. The reviewer's probe fills exactly that gap.

Impact is accounting and noise: the loss already happened, the record shows `completed / settled=false / settlement_loss=false` with a dead claim txid, and the nurse retries every tick. Fix: let a requested reconcile process unsettled `Completed`/`Refunded` records through the role-aware matrix (gated on the confirmed-winner depth so a single-view hint cannot reopen a truly settled record), and route the ordinary nurse's inputs-spent failure through the same rollback as the immediate and pending paths, preserving the initiator's first-reveal evidence. Cover both roles' lost settlements.

## S3 — replaying a public signed offer restores it after revocation. Valid, Medium, and a regression.

Confirmed. `prune` (`corkboard/src/main.rs:122-131`) deletes rows with `revoked != 0`, and `post_offer` (`149-160`) calls `prune` before its `INSERT … ON CONFLICT(offer_id) DO NOTHING`. Before the N13 change the revoked row stayed and the conflict clause acted as a tombstone; the first review of this batch noted the weakening ("anyone can re-post that id afterwards"), and this review demonstrates it with a replay of the captured signed envelope, no maker key needed. The server also assigns a fresh receipt time and TTL, so a replayed old offer gets a new lifetime, and each replay counts against the victim's per-identity offer quota (128), which turns it into a way to exhaust a maker's quota with the maker's own stale envelopes.

Bounded impact: the maker's local ledger still refuses a revoked offer, so no trade proceeds; the harm is listing integrity, futile takes, and quota occupancy. Fix: keep revoked rows (or a separate tombstone of `(offer_id, identity)`) for at least the offer's signed validity window and reject a re-post that matches a tombstone; derive expiry from the signed `created`/`ttl` rather than server receipt time so an expired publication cannot regain a lifetime; keep idempotent retries of a live offer working. Regression: publish, revoke, replay, including an intervening prune and a restart.

## Other observations

- `spec/protocol.md:598` still says Bob broadcasts "before `T1 − 1 h`". The batch's P3 pass fixed the surrounding text but missed this line. Documentation only.
- The remaining known limits (global scan lock, deep-reorg cursor, ambiguous funding markers, recipient quota, LTC operator diversity) are unchanged and already tracked.

## Reproduction

The reviewer's probe script was re-run here unchanged, on an idle harness, from `9348bae`. All five probes reproduced with exit 0: `accepted-then-lost` v1 and v2 (record `completed / settled=false / settlement_loss=false`, dead claim retained, `settlement-conflict` and `-25` every tick), `initiator-refund-starved` v1 and v2 (`redeemed_b` after three ticks past T1 with leg A unspent, then a manual refund succeeds at once), and `revoked-offer-replay` (`revived: true`). The findings are established on the PR branch, not only on the reviewer's machine.
