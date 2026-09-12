# Second review of the protocol remediation batch (2026-09-11)

Reviewed: the amended uncommitted working tree on `f670cd9`, per [the corrective follow-up](2026-09-11-protocol-fixes-followup.md), against [the first review](2026-09-11-protocol-fixes-review.md). No production source was changed by this review. Method as before: every correction traced in the current code by parallel reviewers with exact `file:line` evidence, plus an independent run of every test gate.

## Verdict

**Close to ready. Four of the six blockers are closed and the merge gate is green; three items still have to be fixed before commit, all small.** The scan-throttle regression is gone (cached results, `Ok(None)` on throttle, invalidation on a new tip, waiting instead of failing), the legacy Core spend lookup falls through to the block scan, the fee economics moved into pre-flight with a post-commit cap instead of a refusal, and the TLS verifier is now the right design. What remains: the losing late claim still ends up `Completed` one tick later via reconcile (the batch's own logs show it), the new RPC deadline table puts the actual v1 funding RPC in the 30-second bucket, and the stricter certificate validation rejects more than half of the shipped default view lists without any documentation or decision.

## Required before commit

### R1. The mixed settlement pair still books the losing participant as `Completed` (B3, partial)

The claim path is fixed: the exact outpoint is checked unspent before any write (`engine.rs:5272-5278`, v2 `3417-3427`), an inputs-spent rejection restores `FundedB` / `Signed`, clears the markers and requests reconcile (`928-952`), and the pending retry applies the same (`8901-8910`, `3641-3649`). But the reconcile that follows classifies the pair "A refunded by Alice, B redeemed by Alice" through `v1_settled_terminal` (`8114-8125`) and `v2_settled_terminal` (`8150-8153`), whose rule is `completed = a.kind == Redeem || b.kind == Redeem`. For the participant that yields `Completed` with the counterparty's refund adopted as his own `final_txid` (`8309-8316`), then `settled = true`. The batch's own evidence shows it: `2026-09-11-late-claim-v1.log` and `-v2.log` both print `reconciled chain truth → Completed: both legs settled on-chain (A Refund, B Redeem)` right before the scenario's "stayed out of Completed" line, because `swap_v1.py:61` asserts before the ticks at `:68-69`.

That mapping predates this batch, but the batch's N4 change makes the case reachable and claims it closed. Fix: make the mixed pair role-aware (the participant's leg A was refunded, so his terminal is `Refunded`-with-loss or a distinct state, never `Completed`, and never the counterparty's refund as his settlement), and move the scenario's state assertion after the ticks. Also add a direct test of the -25 abandon path; today it has only call sites.

### R2. Core `send` is in the 30-second RPC bucket (regression)

`rpc.rs:257-266` classifies `sendtoaddress`, `sendmany`, `fundrawtransaction`, `walletcreatefundedpsbt`, `signrawtransactionwithwallet` and `listtransactions` at 120 s and everything else at 30 s. The v1 confirmed-only funding uses Core `send` (`chain.rs:1459`), which therefore dropped from the previous uniform 120 s to 30 s. A slow wallet send now times out client-side, `is_pre_send_rejection` is false, the `funding_attempt` marker stays `"pending"`, and the swap is blocked with "funding outcome unresolved" — the exact ambiguous-send case the deadline change was meant to reduce. `bumpfee` (`1913`) and `submitpackage` (`1811`) are in the same situation; `getblock` at verbosity 2 (`1243`, the block scan) is marginal. Fix: add `send`, `bumpfee`, `submitpackage` (and `getblock`) to the 120 s list.

### R3. The stricter verifier rejects more than half of the shipped default views (B4, partial)

The verifier is now correct: `WebPkiServerVerifier` over `webpki-roots` first (`vendor/electrum-btcx/src/backend.rs:325-329`, `520-525`), TOFU only for a certificate whose self-signature is verified (`331-340`), CA trust remembered so a later self-signed certificate fails closed (`251-267`, `277-283`), pins persisted only after the handshake signature verifies (`371-401`), and an authenticated `tlspin inspect|forget` RPC (`pactd/src/main.rs:1215-1223`). The 19 loopback tests pass here.

But webpki rejects X.509 v1 certificates, self-signed certificates carrying `basicConstraints CA:TRUE` (what a stock `openssl req -x509` produces), and CN-only names without a SAN. Probing the shipped lists (`satchel/coins.toml:82`, `:121`) through the same logic: BTC 6 of 13 usable, LTC 3 of 7 usable; rejected are emzy, qtornado, xurious, bysh (v1), bitaroo, bitcoin.lu.ke, backup.electrum-ltc.org (CA:TRUE), digitaleveryware, aranguren (no SAN), bluewallet (certificate for the wrong host, correctly rejected). The two-view mainnet quorum still passes, but half the list is dead on a fresh install and neither the handbook nor `coins.toml` says so. This needs an explicit decision: either let the self-signed branch tolerate v1 / CA:TRUE / CN-only certificates (the pin, not the hostname, is what binds a self-signed endpoint, so this does not weaken the CA path), or prune and re-document the default lists. `pact/Cargo.toml:31` still describes the old accept-any-cert verifier.

## Closed since the first review

| Item | Status | Evidence |
|---|---|---|
| B1 scan throttle | Closed | `chain.rs:1033-1103`: result cached per `(endpoint, script)`, cached negative returns `Ok(None)` within 120 s and only while the best-block hash is unchanged, cached positive re-validated with `gettxout`, one global mutex that waits, failures not cached. No `throttled`/`busy` error path remains. Loopback fixture `tests/core_discovery.rs` (3 tests) passes. All 20 previously failing scenarios pass. |
| B2 legacy spend lookup | Closed | `chain.rs:1198-1210`: mempool over 128 skips the walk, a budget-exhausted walk breaks instead of erroring, an evicted transaction in the walk is skipped; control reaches the block scan. Fixture covers 200-entry and slow mempools. Residual: on modern Core a transaction evicted between `gettxspendingprevout` and `getrawtransaction` still aborts that pass (recovers next tick). |
| B5 fee economics | Closed | Pre-flight in `ensure_can_fund_new_offer` / `ensure_can_fund` (`engine.rs:1626`, `1523`) at post and take; `funding_fee` (`1305-1320`) caps the rate to `max(amount/2, coin floor)` with no refusal; unit tests at the exact floor, below it, the budget boundary and extreme input pass. Regtest skips pre-flight, so this is unit-tested only. |
| B6 merge gate | Closed | Author's `2026-09-11-merge-gate-final.log` is a single invocation, 69 passed. Independently reproduced here on a rebuilt cache with no stale processes: 9 suites, 69 scenarios, exit 0. |
| N5 v1 aborts | Closed with gaps | Manual abort, peer abort and the pre-funding timeout consult `funding_may_exist`; definite pre-send RPC codes clear the marker. Gaps: no operator path to resolve a stuck `"pending"` marker; nodeless errors are never `RpcError` so never cleared; the C8 arm emits a tick error every pass while the marker exists. |
| N5 v2 leg-A intent | Closed | Cleared after successful funding; a confirmed competing spend (found spend, different txid, `tx_confirmations_final > 0`) cancels and unlocks; an inconclusive one does not. `ConflictedLegAIntent` covers no-conflict and confirmed-conflict, not mempool-only. |
| N6 refunds | Closed | Both protocols write ahead; the retry chain is chosen from role and settlement type, so a chain-A refund cannot go to chain B. |
| N10 | Closed | Initiator Signed-state refund arm reaches the exact-value replacement; claims stay bound to their signing outpoint. |
| N1 hardening | Closed | Deterministic conflicts are `ensure_permanent!`; `funding_*_height` recorded again. |
| N3 residual | Closed | Bump and winner adoption select the input by witness script / outpoint. |
| P1 | Closed | Tolerance `max(estimate−1, 3/4·estimate)`; rejected-parent CPFP priced above the parent and the primary's `mempoolminfee`; inputs-spent skips the package; one rescue attempt per tick. |
| N13 | Closed | Sender column with `ALTER TABLE` migration, per-sender cap 256, test for 257 recipients passes. Note the 7-day retention means one key is limited to 256 relay posts per board per week. |
| N20 | Closed | Audits on PRs and inside the engine/UI jobs; launch-arguments file deleted by pactd via `--delete-coin-config-file` on the success path only. |
| N15 | Closed (Unix untested here) | `O_NOFOLLOW`, modes on the descriptor, recursion into wallet directories, seed repair after install. |
| P3 | Closed | Spec text now matches `action_margins` and the arms; one stale doc comment at `engine.rs:485` still mentions a T1−1h claim cutoff. |

## Independent verification

| Check | Result |
|---|---|
| pact workspace tests | 238 passed (19 vendored TLS, 184 libswap, 3 core_discovery, 2 vectors, 3 pact-cli, 27 pactd) |
| corkboard / satchel / crier / pact-proto / pact-nostr | 3 / 21 / 17 / 11 / 8 passed |
| clippy `-D warnings`, every crate | clean |
| UI `tsc` build, eslint, `npm audit --audit-level=high` | clean, 0 vulnerabilities |
| Full regtest sweep, rebuilt cache, no stale processes | 9 suites, 69 scenarios, exit 0 |

## Minor notes, not blocking

- The scan mutex is process-global, so a 300 s mainnet `scantxoutset` on one coin blocks another coin's `find_funding`.
- The block-scan `pending` result still surfaces as a tick error on Core-only views until the cursor catches up (bounded, transient), and a reorg deeper than six blocks can hide an already-scanned spend until restart.
- `claim_pending` is never cleared for a lost initiator reveal or a lost refund; the marker block runs before the `settled` early return, so a settled record with a dead marker rebroadcasts every tick.
- `mempoolminfee` is not consulted at funding time; a funding capped below a risen admission floor waits for the funding nurse rather than erroring.
- Corkboard: a hostile key can still fill one victim's 256-row recipient quota for seven days.
- Handbook ch06 should say that CA-to-self-signed migration also needs `tlspin forget`.
