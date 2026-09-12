# Third review of the protocol remediation batch (2026-09-11)

Reviewed: the amended uncommitted working tree on `f670cd9`, per [the second follow-up](2026-09-11-protocol-fixes-followup-2.md), against the three items [the second review](2026-09-11-protocol-fixes-review-2.md) required before commit. No production source was changed by this review.

## Verdict

**Ready to commit.** All three required items are fixed in the code, each carries a regression test, and every gate passes independently here: 241 pact-workspace tests, corkboard 3, satchel 21, clippy clean in every crate, UI build and lint clean, and the full nine-suite regtest sweep at 69 of 69 on a rebuilt cache.

## The three required items

### R1. Mixed settlement pair — fixed

`engine.rs:8165-8181`: `v1_settled_terminal` is role-aware. `Completed` requires OUR claim leg to be redeemed; otherwise `Refunded` with our refund adopted if our leg was refunded; otherwise `Refunded` with no settlement of our own. `v2_settled_terminal` (`8205-8224`) delegates to it. Both reconcile writers (`7938-7958`, `8066-8079`) treat the no-settlement case as a loss: `settlement_loss = true`, no `final_txid`/hex adopted, `settled = true`, and the loss is latched only after the conservative mixed-pair depth gate (`follow_purge_ok`). The flag is `#[serde(default)]` on both records (`store.rs:141`, `275`), so older records read `false`; `listswaps`/`listadaptorswaps` emit it, the UI type carries it, and `narrate.ts:69` shows an explicit loss message (English only until the next locale sync, as stated).

Tests: the matrix unit test covers both roles and all four pairs; `missing_inputs_abandons_only_participant_claims_and_rearms_reconcile` (`15541`) exercises the -25 abandon path directly (policy error leaves the claim, inputs-spent restores `FundedB`/`Signed`, clears bytes and marker, re-arms reconcile); a settled-record test verifies a dead `claim_pending` is retired before any parse or RPC. Both `LateClaimAfterRefund` scenarios now assert after the ticks: state `refunded`, `settled`, `settlement_loss`, and no settlement txids or bytes. Both pass here.

Note on semantics: the losing participant's terminal state is `Refunded` plus the loss flag rather than a new enum variant. That is an accounting choice that avoids a wire-incompatible state; the flag and narration make the outcome explicit, which is what the finding required.

### R2. RPC deadlines — fixed

`rpc.rs:459-474`: `send`, `bumpfee`, `submitpackage` and `getblock` join the 120-second class with the other wallet operations; `scantxoutset` stays at 300 s and ordinary reads at 30 s. A unit test pins the classification of the funding and rescue RPCs.

### R3. Default views — fixed by decision, documented

The strict verifier is kept unchanged. `satchel/coins.toml` is pruned to the six BTC and three LTC endpoints that pass the production TLS, protocol and genesis probe; the file comments name the probe and date. Handbook ch06 documents the policy (X.509 v1, CA-only leaf and CN-only certificates are not supported), that existing user overrides are not rewritten, and the reduced operator diversity of the LTC list (two operator groups). The stale verifier comment in `pact/Cargo.toml` is corrected, and ch06 now says a CA-to-self-signed migration needs `tlspin forget`.

This is an acceptable decision. It should be revisited if the LTC list's two-operator diversity turns out to matter for the mainnet two-view quorum in practice.

## Independent verification

| Check | Result |
|---|---|
| pact workspace tests | 241 passed (19 vendored TLS, 187 libswap, 3 core_discovery, 2 vectors, 3 pact-cli, 27 pactd) |
| corkboard / satchel | 3 / 21 passed |
| clippy `-D warnings`, pact workspace and satchel | clean |
| UI `tsc` build and eslint | clean |
| Full regtest sweep, rebuilt cache, no stale processes | 9 suites, 69 scenarios, exit 0 |
| Author's `2026-09-11-review2-merge-gate-final.log` | single invocation, 69 passed, 0 failed |

## Carried-over notes, not blocking

Unchanged from the second review: the process-global scan lock, the transient block-scan `pending` errors on Core-only views and the deeper-than-six reorg cursor hole, the absence of an operator path for a stuck ambiguous v1 funding marker, `mempoolminfee` not consulted at funding time, and the per-recipient corkboard quota being fillable by one hostile key. The follow-up lists them as known limits; they are suitable follow-up tickets rather than commit blockers.
