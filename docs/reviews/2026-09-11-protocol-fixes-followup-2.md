# Response to the second remediation review

2026-09-11; uncommitted working tree on `f670cd9`. Responds to [review 2](2026-09-11-protocol-fixes-review-2.md), without modifying that review. This supersedes the earlier follow-up's claim that B3 was fully closed.

## Assessment and corrections

All three required findings are legitimate.

| Finding | Assessment | Change |
| --- | --- | --- |
| R1: mixed settlement | High-priority accounting/state defect. The earlier claim guard did not fix reconciliation, and the test asserted before reconciliation. It did not prevent the economic loss inherent in a late claim. | Both protocols use the same role-aware settlement matrix. `Completed` requires our claim leg to be redeemed. A losing mixed pair becomes `Refunded` with persistent `settlement_loss=true`, no own settlement transaction, and an explicit loss detail. The UI explains that neither payment nor refund was received. Both legs must meet the conservative mixed-pair depth gate before a loss is latched. |
| R2: RPC deadlines | Medium-priority liveness regression: a wallet send could time out earlier and leave a deliberately fail-closed funding marker. | `send`, `bumpfee`, `submitpackage`, and `getblock` now receive the 120-second response deadline. A regression protects these method classifications. Ordinary reads remain 30 seconds; UTXO scans remain 300 seconds. |
| R3: default views | Medium-priority availability/compatibility defect, independently reproduced with production TLS and genesis verification. | Keep strict certificate validation and prune incompatible/unreachable defaults. The resulting fleets pass 6/6 BTC and 3/3 LTC probes. Update fleet comments, handbook, and the stale Cargo verifier comment. |

The loss flag defaults to false when reading older JSON records. It distinguishes terminal loss from successful refund without adding an incompatible state enum. The new UI loss message falls back to English until the next locale sync. This is an accounting correction, not a guarantee that a participant can recover funds after the counterparty's refund wins.

Direct and retry claim failures now share tested rejection guards. A typed Core `-25` error clears participant claim bytes and pending metadata, restores `FundedB`/`Signed`, and re-arms reconciliation. Ordinary policy errors, initiator reveals, and refunds do not use that rollback. The two live late-claim scenarios now assert the persisted state, loss flag, settled latch, absent settlement txids/bytes, and no rebroadcast **after** scheduler ticks.

The related dead-marker note is also addressed for settled records: both schedulers retire `claim_pending` before considering any pending broadcast, and depth-confirmed reconciliation clears it. A unit regression uses malformed pending bytes and no configured backend to verify no transaction parsing or RPC is attempted once settled. The stale T1 claim-cutoff comment and CA-to-self-signed reset documentation are corrected.

## Default view decision

Fresh probe pin directories were used; existing user pins were not reset. Original fleets reproduced 6/13 BTC and 3/7 LTC success. Removed BTC entries: emzy, bitaroo, qtornado, bitcoin.lu.ke, digitaleveryware, aranguren, and bluewallet. Removed LTC entries: backup.electrum-ltc.org, xurious, rentonisk (connection refused), and bysh. The remaining LTC entries include two cipig.net servers and one petrkr.net server: three endpoints are only two apparent operator groups. The handbook explicitly documents this availability/diversity limit.

Existing user overrides are not silently rewritten. Strict self-signed validation still requires a compliant leaf certificate and hostname SAN; TOFU does not bypass these checks. Deliberate CA-to-self-signed migration requires out-of-band verification, `tlspin forget`, and reconnecting. Normal CA renewal does not require a reset.

## Verification

- Strengthened `LateClaimAfterRefundV1` and `LateClaimAfterRefundV2`: passed, 14 s and 13 s. Logs now show `Refunded` with an explicit loss rather than `Completed`.
- Final workspace tests: 241 passed (187 libswap, 19 vendored TLS, 3 Core discovery, 2 vectors, 3 CLI, 27 pactd); exit 0. Workspace/all-target clippy with `-D warnings`: passed, exit 0.
- Desktop: 21 tests passed.
- UI lint and production build: passed (existing bundle-size warning remains).
- Production `electrum_probe -- btc --all` and `-- ltc --all` against the pruned lists: 6/6 and 3/3 passed, both exit 0.
- The first sweep hit a setup timing failure in `ConcurrentDrainNoDoubleSend`: the relay database contained the published offer but the taker cache was empty. `tick` starts its Nostr pass asynchronously, and this scenario injects an 800 ms delay; 20 rapid RPC polls could finish before the pass completed. Its setup/completion loops now use bounded elapsed-time waits (30/60 s, 250 ms polling), retaining the same zero-duplicate, zero-rejected-take and completed-swap assertions. The failed sweep coordinator was stopped by its verified PID; its current rescue suite was allowed to finish teardown. This failed run is not counted as a successful gate.
- Repaired concurrency scenario: passed separately in 21 s, exit 0, with zero duplicate takes under the injected 800 ms delay.
- **Final full merge gate: 69/69 scenarios, all nine suites passed, exit 0.** Run from `pact/harness` with `PYTHONUTF8=1`, `PYTHONUNBUFFERED=1`, `python -X utf8 test_runner.py --rebuild-cache`, no filter. No production or test changes were made during this final run.
- Final `git diff --check`: passed. Modified Python tests and coin TOML parse successfully.

| Suite | Passed | Duration |
| --- | ---: | ---: |
| framework_selftest | 3 | 6 s |
| multimachine | 1 | 1 s |
| swap_v1 | 25 | 380 s |
| swap_v1_rescue | 8 | 200 s |
| swap_v2_adaptor | 12 | 153 s |
| nodeless | 4 | 132 s |
| follow | 3 | 66 s |
| takeover | 10 | 246 s |
| upgrade | 3 | 59 s |

Evidence logs are `2026-09-11-review2-*.log` in this directory (ignored by Git); the complete successful sweep is `2026-09-11-review2-merge-gate-final.log`. The earlier 69-scenario run is historical evidence, not verification of this revision. No commit has been made.

## Remaining review notes

The process-global scan lock, bounded block-scan error cadence/deep-reorg cursor limitation, unresolved ambiguous funding markers, funding admission-floor changes, and recipient-quota exhaustion remain limitations. They are not made safe by this test run. The earlier follow-up's platform-validation and dependency-audit caveats still apply; dependencies did not change in this revision.
