# Corrective follow-up to the remediation review

**Historical first follow-up:** [review 2](2026-09-11-protocol-fixes-review-2.md) found that the B3 reconciliation path and its assertion were still wrong, the deadline table omitted Core `send`, and default view compatibility needed correction. See [the second follow-up](2026-09-11-protocol-fixes-followup-2.md) for the current changes and verification.

2026-09-11; uncommitted changes on `f670cd9`. Addresses [the independent review](2026-09-11-protocol-fixes-review.md). The initial handoff ran five scenarios and did not meet the merge gate. It should not have been presented as ready for review without explicitly completing that gate.

## Blocking regressions

| Review item | Correction | Regression evidence |
| --- | --- | --- |
| B1 / F2 | Cache successful scan results; return cached negative answers, invalidate negatives on a changed best-block hash, refresh positive outputs with `gettxout`, and serialize concurrent scans by waiting. Failed/incomplete scans are not cached. | Real JSON-RPC fixture tests concurrent reads, cache hits, new-tip invalidation and spent-output invalidation. Final full sweep: 69/69 passed. |
| B2 / F1 | Large legacy mempools skip to block discovery; exhausted small-mempool budgets fall through too. Evicted transaction reads do not abort the block scan. | RPC fixtures emulate absent `gettxspendingprevout`, 200-entry mempool and a slow mempool read; both still recover the mined witness. |
| B3 / N4 | Check the exact claim outpoint before persisting a participant claim. An inputs-spent broadcast rejection restores a nonterminal state and clears its failed claim and retry marker, then requests reconciliation. Apply this to pending retries as well. V2 claims never substitute a replacement outpoint into an old signature. | New `LateClaimAfterRefundV1/V2` scenarios also check that a leftover refund retry marker is processed on the refund chain. |
| B4 / N11 | CA roots, hostname and validity validation precede self-signed TOFU. Remember CA trust to prevent later downgrade. Only a cryptographically self-signed certificate can use TOFU; pins are saved after handshake proof. Authenticated `tlspin inspect/forget` supplies a reset path. | Actual loopback TLS tests: CA certificate/key rotation, CA-to-self-signed downgrade rejection, hostname rejection without pinning, self-signed pin change/reset. |
| B5 / N9/P2 | Use the existing 3430-sat planning floor. Check current reserve and amount-relative economics in offer/take preflight, before commitment. After commitment, cap the selected funding rate to the value budget rather than introduce a new market-dependent amount refusal. | Unit coverage at the exact floor, below it, the high-rate economic boundary, and extreme input. Existing offline envelope construction remains supported. |
| B6 | Run the nine-suite entry point on a rebuilt cache with no stale harness processes, then rebuild and rerun after final code changes. | Final rebuilt-cache run: 69/69 scenarios, all nine suites passed, exit 0. Changes remain uncommitted for independent review. |

## Other release findings addressed

- V1 manual and timeout aborts respect ambiguous funding markers. Regression covers both RPC refusal and a timeout tick. Definite pre-send RPC failures clear the marker; unknown outcomes remain fail-closed.
- V2 clears built leg-A metadata after recording successful funding. A confirmed competing spend can cancel an impossible unbroadcast intent and unlock its inputs; an inconclusive/unconfirmed conflict cannot authorize cancellation. `ConflictedLegAIntent` tests this boundary.
- Refunds in both protocols now persist bytes/state before broadcast. The pending-transaction path chooses the chain from role **and settlement type**, so it cannot send a chain-A refund to chain B. Participant claim conflicts do not erase an initiator's first-reveal tracking.
- Signed-state v2 initiator refund reaches exact-value replacement discovery. Cooperative claims remain bound to their original signing outpoints.
- Deterministic v2 peer-message conflicts are permanent relay failures; valid funding messages again record the observed funding height.
- A rejected cooperative parent forces CPFP pricing above its own rate and the primary node's current admission floor even if an estimator is stale. Inputs-spent failures skip pointless package submission. Failed pending v2 rescue is attempted once per tick. Fee acceptance tolerates small estimator skew.
- Corkboard stores the authenticated sender, migrates old databases, and caps retained messages per sender as well as recipient/global limits. One key using 257 recipients is rejected at message 257.
- Audits run on PRs and are also steps in the existing CI engine/UI jobs, so a vulnerability fails those jobs. The private desktop launch-arguments file is deleted by pactd after successful consumption via an explicit launch flag.
- RPC deadlines distinguish ordinary reads (30 s), potentially slow wallet operations (120 s), and UTXO scans (300 s), reducing avoidable ambiguous wallet sends.
- Unix private-directory repair descends into wallet directories; private writes use `O_NOFOLLOW` and set modes on the opened descriptor. Seed installation repairs file modes before returning. These Unix paths still require Unix runtime validation.
- Settlement bump/winner tracking selects the HTLC/refund witness input instead of assuming input zero.
- P3 spec text now actually matches the engine: funding and first-reveal cutoffs differ, public-secret claims have no clock cutoff, Alice may fund before adaptor signatures, and Bob must have them before broadcasting leg B.

## Verification record

The first follow-up sweep began only after identifying and stopping the stale Corkboard process for `pact-V1NodelessBothSides-t_ss5zov`; it had prevented the build from replacing corkboard.exe. No user daemon was stopped. Funded cache deletion was confined to `C:/code/pocx/satchel/pact/harness/cache`.

The preliminary sweep passed all nine suites. The final run rebuilds the final tree, uses `PYTHONUTF8=1`, `PYTHONUNBUFFERED=1`, and executes `python -X utf8 test_runner.py --rebuild-cache` from `pact/harness` without a scenario filter. Its complete log is `2026-09-11-merge-gate-final.log` (ignored by Git).

Completed checks on Windows:

- `cargo test --workspace` in `pact`: 238 tests passed, including the vendored Electrum TLS tests and Core discovery fixtures. `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Desktop: 21 tests passed; Corkboard: 3 tests passed. Clippy passed for both.
- UI lint and production build passed. Targeted transitive dependency updates removed the audit findings; npm audit reports zero vulnerabilities.
- Cargo audits passed for all seven lockfiles with zero vulnerability findings. Informational unsoundness, unmaintained, and yanked-package warnings remain in the saved JSON reports; this is not a warning-free dependency tree.
- Targeted `ConflictedLegAIntent`, `LateClaimAfterRefundV1`, and `LateClaimAfterRefundV2` scenarios passed; all three are included again in the final full sweep.

Final full sweep: **69 passed, zero failed; all nine suites passed; exit 0**. No production code or test changes were made during this final run.

| Suite | Scenarios passed | Duration |
| --- | ---: | ---: |
| framework_selftest | 3 | 6 s |
| multimachine | 1 | 1 s |
| swap_v1 | 25 | 389 s |
| swap_v1_rescue | 8 | 221 s |
| swap_v2_adaptor | 12 | 166 s |
| nodeless | 4 | 136 s |
| follow | 3 | 68 s |
| takeover | 10 | 255 s |
| upgrade | 3 | 59 s |

The full sweep includes the previously failing hard-P2 queue, nodeless, pre-funding takeover abort, mixed-version swap, and both new late-claim-after-refund cases. Legacy Core compatibility and certificate rotation are separately covered by loopback fixtures rather than inferred from regtest success.

## Remaining limits

This does not establish a scheduler-wide deadline under arbitrary DNS/network stalls: the registry lock still encloses some network work. Larger method-specific deadlines are an explicit safety/liveness tradeoff. Claim rescue still depends on backend package support and available output value; accepted-parent CPFP tests do not prove survival of every sustained fee spike/refund race. Unresolvable ambiguous v1 sends remain blocked rather than being sent twice. Self-signed TLS first contact still requires trust; legacy certificates must satisfy current certificate validation requirements. Platform signing/notarization/attestation, broad key erasure/privacy work and the other explicitly deferred items in the original handoff are not claimed complete.
