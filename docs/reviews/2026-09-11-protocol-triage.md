# Protocol report: severity reassessment and triage

Assessment date: 2026-09-11. Current HEAD: `f670cd9cd3548294c744abd0a3211572fc392784`.

Source report: [September 10 protocol-to-implementation review](2026-09-10-protocol-to-implementation.md), reviewed at `8ba68db`. This note preserves that historical report and distinguishes severity, current status, evidence confidence, and repair priority. It proposes work; it does not implement fixes.

**Recommendation: block the next release on the remaining swap-loss paths. Do not use the original repair order unchanged.** P1 is a fund-safety issue, several findings are already addressed, and some suggested repairs need stronger invariants. The report's evidence supports serious defects, but not its unqualified claim that two complete thefts were reproduced or require no additional preconditions.

## Rating policy

- Critical: a counterparty can plausibly cause loss of the victim's swap principal using ordinary protocol access, without a separate infrastructure compromise. A Critical rating here can be provisional pending an end-to-end loss reproduction.
- High: potential principal loss or broad interruption of fund monitoring, with additional timing, failure, or environmental conditions.
- Medium: recoverable lockup, bounded economic harm, service disruption, or a materially constrained security exposure.
- Low/Info: limited operational impact, documentation, or defense in depth without an established exploit.

Priority is separate: **P0** means a release gate; **P1** means the next repair batch; **P2** means scheduled maintenance. P0 includes validation that could lower or close a finding. Platform-specific release gates apply to that platform. These are engineering ratings, not calculated CVSS scores.

## What changes at current HEAD

1. **N3's malformed-nonce entry point is fixed.** `pact-proto/src/seal.rs:78` rejects non-12-byte nonces; `engine.rs:10631` contains a parser-opening panic boundary. The existing malformed-nonce regression was rerun for this assessment and passed. Keep the adopted-transaction panic claim separate: unchecked `output[0]` consumers remain, but the new witness authentication changes whether a lying view can reach them. The old library reproduction does not establish that residual path.
2. **N7's confirmed-input gaps are addressed.** V2 leg A calls `wallet_send_confirmed` (`engine.rs:2781`); Core funding enforces `minconf` or explicit confirmed input selection for unsupported forks (`chain.rs:1274`, `1369`). This does not close N10: the leg-A send still requests replaceability.
3. **N19 is partially fixed.** Author-scoped removal exists (`store.rs:831`), and the earlier verification records scoped tombstones. However, insertion still compares timestamps and deletes by `d_tag` alone (`store.rs:797–807`). Cross-author cache replacement remains; the original permanent-tombstone attack should not be described as unchanged. Old poisoned tombstones are a separate migration concern.
4. **The prior-review reconstruction escalation is no longer an unchanged open finding.** Current code authenticates witnesses and caps history depth using a finality read. `chain.rs:2014` bases finality acceptance on responding views' trust. The [prior fix verification](2026-09-10-fix-verification.md) records closure within the configured trust model. That is not independent chain inclusion validation or protection against an attacker controlling every view.

The source report's statement that all September 9 findings remain present is historical, not a description of this HEAD.

## Severity and disposition

“Retain” below means retain for triage, not that this assessment independently reproduced the finding. Explicit source checks are described above and below; other rows assess the report's stated evidence and preconditions.

| ID | Reassessed severity / status | Priority | Reason or qualification |
| --- | --- | --- | --- |
| N1 | **Critical, provisional; open** | P0 | Current `recv_adaptor` still overwrites either funding pointer without role/state/immutability checks (`engine.rs:2510`). Record corruption was reproduced in the supplied evidence; full theft was not. Loss is the victim's funded leg; the attacker recovers its own leg. |
| N2 | **High; open** | P0 | Accept paths still create records through upserts. The stated attack requires a new accepted take/init with a reused anchor; it is not established that arbitrary unsolicited init replay always reaches acceptance. Upgrade to Critical if ordinary relay delivery alone reproduces a complete theft. |
| N3 | **Main remote vector fixed; residual unverified** | P0 validation | Close the malformed-nonce path. Test malformed adopted transactions against current signature/classification gates before assigning the old High rating to the residual. |
| N4 | **High; open** | P0 | Both protocols still suppress participant claims after the secret is visible (`engine.rs:3756`, `8554`). Honest lateness suffices; a malicious clock is an additional trigger, not a prerequisite. |
| N5 | **High; open** | P0 | Source confirms send precedes vout lookup, height reads, and persistence (`engine.rs:4586–4629`), and `Funded` classification falls through. A failure plus retry can duplicate principal exposure. |
| N6 | **High; retain pending fault injection** | P0 | Broadcast-before-write can leave a public secret without active settlement fee management. Loss additionally requires a competing settlement winning. Validate recovery at current HEAD. |
| N7 | **Addressed for reported confirmed-input gaps** | Closed scope | Retain regression coverage and fork compatibility; do not count it as an open High. |
| N8 | **Medium, down from High** | P1 | Deterministically derived funds are not cryptographically burned. Current payout ownership gates also affect whether the fallback is used. Missing recovery support can block a claim or strand proceeds; reproduce that current path before claiming principal loss. Failing address acquisition early is still worthwhile. |
| N9 | **High, up from Medium, conditional** | P1; gate nodeless release if confirmed | Automatic excessive funding fees can consume substantial wallet value. Requires a bad fee source and sufficient balance; quantify the actual cap and loss in a wallet test. |
| N10 | **Medium; retain** | P1 | External replacement is an extra prerequisite. Recovering a refund is required even when the signed redeem cannot be repaired. |
| N11 | **High, up from Medium, conditional** | P1; gate affected network configuration | If the reported pinned dependency accepts arbitrary certificates, an on-path attacker defeats assumed view independence. This is a trust-boundary failure, not just a privacy issue. Dependency implementation was not re-inspected here. |
| N12 | **Medium; retain** | P1 | A view outage suppresses a consensus-enforced refund. Loss needs an additional adverse settlement; ordinary effect is delayed recovery. |
| N13 | **Medium; retain** | P1 | Public service disk exhaustion. A signature alone does not prevent identities being created cheaply; budgets and retention are the actual availability controls. |
| N14 | **Low, down from Medium** | P2 | Stated path kills the shared daemon when nothing is active; user-visible disruption is recoverable. Raise if it bypasses the active-swap lifecycle guard. |
| N15 | **High, up from Medium, conditional on Unix exposure** | P0 for affected Unix distributions | Another local user obtaining the cookie or seed can cause wallet-wide loss. Requires permissive file modes and traversable parent directories; a 0755 home alone does not establish exposure. Test the actual install path and umask. |
| N16 | **Low currently; retain unsafe sink** | P1 small fix | The demonstrated shell sink is real, but the current caller requires a compromised trusted response. Do not rate a hypothetical future offer-link caller as current remote code execution. |
| N17 | **Medium; open** | P1 | Current `ui/src/dialogs/WalletActions.tsx:245` still strips commas. The fee confirmation reduces exploitability but does not make silent numeric reinterpretation acceptable. |
| N18 | **Low, down from Medium** | P2 | Report establishes failed courtesy withdrawal, not principal loss. |
| N19 | **Medium; partially fixed** | P1 | Scope insert/update/cache identity as well as deletion. The remaining replacement path is visible in current source. |
| N20 | **Split; no useful single severity** | P1/P2 | Reachable dependency DoS, exposed RPC credentials, release integrity, and optional hardening have different impacts and prerequisites. See below. |
| P1 | **High, up from Medium** | P0 | A participant unable to relay its fixed-fee claim after revelation is a potential principal-loss path. This is implementation policy as well as specification. A low rate alone is not a theft reproduction: sustain rejection through the relevant refund race. |
| P2 | **Medium; retain** | P1 | Small-leg fee economics can prevent economical recovery. A temporary high fee does not prove permanent loss; competing-spend risk is conditional. |
| P3 | **Low for spec mismatch; Medium for timing policy** | P1 with N4 | Separate documentation from actual funding/reveal windows. The claimed one-hour confirmation budget is an estimate, not a deterministic deadline guarantee. |
| P4 | **Medium; retain** | P1 before recovery assurances ship | An incorrect seed-only recovery promise can leave users without required scope metadata. Treat backup documentation as user-facing safety work. |
| P5 | **Low/Info; split** | P2 | Move live-datadir cloning and normative replay/nonce requirements into P0 invariant documentation. Other inconsistencies remain maintenance work. |
| F1 | **High availability; retain** | P0 bound the work; P1 optimize | Global-lock network work can delay every swap's safety actions. Source still enumerates `getrawmempool` (`chain.rs:1055`). The reported minutes are modeled, not measured here. |
| F2 | **Medium; retain** | P1 with F1 | Repeated global scans compound F1; coalesce, serialize and bound them. |
| F3 | **Low, down from Medium absent load evidence** | P2 | Repeated key access is wasteful; establish contribution to missed deadlines before elevating. Caching decrypted seeds also changes memory lifetime. |
| F4, F5 | **Low** | P2 | Cache unnecessary probes and avoid historical full-record work after fixing unbounded chain work. |
| F6 | **Medium availability, up from Low** | P1 | Requests accumulating behind the global lock amplify F1. Add in-flight suppression and bounded server work; memoization is a separate Low item. |
| F7 | **Info** | No defect ticket | Baselines need environment/workload metadata; bundle size is not a security severity. |

For N20, create separate tickets: reachable dependency advisories (provisional Medium, verify against current lockfiles), explicit RPC credentials in process arguments (High if exposed to other local users), audit enforcement and release provenance (Medium), and CSP/zeroization/toolchain/unsafe-code policy (Low/Info unless tied to a concrete defect). No current advisory lookup or fresh audit was performed here; do not promise that a particular `cargo update` clears every issue.

The unnumbered Low list also needs separation. Keep scoped-wallet discovery gaps, Core settlement liveness and unchecked raw-transaction identity as Medium **investigation** items where they affect recovery. Consumed nonces are sensitive but deleting one SQL column does not establish erasure from WALs or backups. Remaining privacy, installer, parser-width and maintenance items can stay Low/Info pending stronger reachability evidence.

## Proposed repair batches and exit criteria

### 1. Preserve handshake and recovery invariants — P0

Address N1 and N2 together. Require correct sender role and legal phase; accept exact duplicate messages as idempotent no-ops where delivery/recovery needs it, and reject conflicting repeats. Create swaps atomically with uniqueness protection instead of using the update operation for creation. Check both protocol namespaces and applicable tombstones. Do not reset an existing live record on an init collision.

Separate immutable signing-session outpoints from discovered recovery outpoints. Repointing an outpoint after signing must never make the engine use old signatures against a new transaction or reopen a consumed nonce session. Recovery may need refund-only behavior.

Exit: tests cover both roles, both protocols where applicable, wrong-leg messages, late/conflicting duplicates, restart/replay, and reused anchors. A private regtest scenario demonstrates that an attacker cannot divert monitoring and then retain the victim's principal. Existing evidence only reproduces corruption, so this loss-level test matters.

### 2. Claim and broadcast recovery — P0

Address N4, N5, N6 and P1. Continue eligible claims after revelation while the output remains spendable; preserve validation and payout checks. Distinguish the first secret reveal from a claim using an already-public secret. Clock uncertainty must not turn a rescue claim into permanent inactivity.

Persist transaction intent/material before broadcast where possible; reconcile ambiguous sends through wallet history and exact transaction identity before allowing a fresh payment. Moving the txid write earlier only narrows the crash window. Adopt our unconfirmed settlement after restart so the fee nurse resumes. Funding lookup must not treat an inconclusive view as evidence that no prior send occurred.

Reject unacceptable pre-signed fees before commitments, but do not present a current estimate as protection against future congestion. Define and exercise a viable claim-fee escalation/recovery mechanism for each supported backend; negotiate or constrain exposure where that mechanism is unavailable.

Exit: inject failure after broadcast, during follow-up RPCs, and at database writes; restart and retry without duplicate funding. Verify fee nursing resumes and that late/public-secret claims are attempted. Exercise a parent rejected by relay fee policy and a competing refund in private regtest.

### 3. Contain daemon-wide stalls and settle stale findings — P0/P1

Keep the passing nonce regression. Test current adoption gates with zero-output, invalid-witness and wrong-destination transactions; validate before storage/use and remove panic-prone assumptions. Recovering a poisoned mutex or catching an arbitrary engine panic does not prove that partially completed mutations are coherent.

Bound total chain work per operation and per scheduler pass; inactivity timeouts alone are insufficient. Optimize Core spend lookup, use scan watermarks, coalesce expensive scans, and prevent UI request accumulation. If moving work outside the lock, preserve serialization of wallet mutations and validate record versions before applying results.

Exit: malformed input cannot stop unrelated swap monitoring; a slow/unavailable backend and a large mempool cannot indefinitely block another swap's due refund. Measure scheduler and RPC latency under the tested load. Set the acceptance budget before implementation.

### 4. Wallet exposure and network trust — P1, conditional release gates

Handle N9/N11/N15 and exposed Core credentials as their own security changes. Verify actual Unix directory traversal/file modes and repair existing sensitive files, including relevant database sidecars; creation modes alone leave existing installs exposed. Inspect the pinned TLS dependency and define authenticated server trust without silently accepting changed identities. Apply funding/send fee limits at the wallet boundary and test them through automatic funding, not only the bump policy.

### 5. Recovery usability, market integrity and maintenance — P1/P2

Batch N8/N10/N12, fee parsing/economics, cache author scoping, storage budgets, and recovery documentation next. Remove the Windows shell sink as a small isolated repair. Then finish shutdown/single-instance behavior, remaining performance tuning, dependency/release controls and spec cleanup.

Non-replaceable signaling is a mitigation for N10, not the recovery invariant. Likewise, raising a claim fee cannot make an output larger: P2 needs adequate funding or an explicitly supported additional-input/package mechanism, not an instruction to ignore value limits.

## Evidence and limits of this reassessment

Read the original report, supplied reproduction source, prior fix-verification note, recent commit history, and current source around handshake writes, claim gates, funding persistence, nonce parsing, confirmed-input selection, finality, cache identity, fee parsing and Core mempool lookup. No production source changed. The existing malformed-nonce regression passed with `cargo test --locked --manifest-path pact-proto/Cargo.toml malformed_nonce_length_is_an_error_not_a_panic` (1 passed).

No full harness, original handshake probe, on-chain theft sequence, adversarial Electrum test, current dependency audit or OS permission experiment was rerun. Historical test results are attributed to the supplied verification note, not counted as new validation. The highest-impact claims remain release-gating validation targets even where complete loss has not yet been demonstrated.
