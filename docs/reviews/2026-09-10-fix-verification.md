# Verification of the September 9 review fixes

## Final residual recheck — September 11, `415f493`

**The last reported residual is fixed. Findings 3 and 7 can now be marked closed for the specific gaps identified in this verification, within the configured chain-view trust model.** Previous assessments below are historical.

`tx_confirmations_min` now retains each responder's trust classification. A trusted Core node's abstention no longer lets one public secondary provide the entire finality verdict. Two public responders are required for that mainnet configuration; otherwise the method errors and history reconstruction keeps the spend shallow. A responding trusted node still permits the intended trusted-node policy, and qualifying multiple public responses use their minimum depth.

Verification: 210 pact-workspace tests passed, including the new responder-trust matrix. Four independent scratch probes passed: actual-wire coin guard, wrong-branch rejection, inflated history capped by zero finality, and Core abstention plus one public responder returning an error. The last probe explicitly supplies public-server health/trust metadata, which the earlier probe did not need because the old implementation only examined the primary's type. Probe source: [September 11 finality probes](2026-09-11-finality-probes.py).

Scope: this closes the reported failure paths, not every claim in the separate September 10 protocol report, deferred performance items, or all possible chain trust risks. The intentional single-server nodeless concession remains; this change is a quorum policy, not independent SPV inclusion validation. No end-to-end or desktop suite was repeated for this chain-only change. No production source was edited. Logs: [September 11 evidence](2026-09-11-finality-evidence/).

## Closure recheck at `23f1082`

Rechecked commits `be42701` and `23f1082` after the initial verification below. **Finding 3 is closed for the reported gaps; finding 7 remains partial.**

- Coin removal now reads the actual `asset` wire field and propagates listing errors. An independent probe using a serialized `ChainRef` now detects the live swap.
- A failed keep-running handoff now retains the window, displays the error and permits retry. Verified by source trace and UI build/lint, not an injected desktop filesystem failure.
- The v1 witness checker now binds the signature to the selected branch's key and checks refund CLTV. The prior wrong-branch probe is rejected, and the new repository regressions pass.
- History-derived depth is now capped by a separate finality read. With a finality reading of zero, the same signed transaction stays at zero even if its history height claims six confirmations. This probe now passes the intended behavior.

**Residual finding 7 — trusted-primary abstention allows a single secondary view to decide finality (Medium, conditional).** `pact/libswap/src/chain.rs:1985` selects quorum 1 whenever the configured primary is Core. The finality read at `:2013` does not require that trusted primary to be among the responders. Commit `23f1082` makes a Core node that cannot locate a transaction abstain, including an unknown transaction on a node without txindex (`:1116`). The remaining secondary view can then supply the entire finality verdict. In a Core-plus-one-Electrum configuration, that restores the single-untrusted-view decision the reconstruction change was intended to prevent. The scenario requires an incorrect/malicious secondary view and an unavailable or transaction-blind trusted primary; it is not an ordinary-user payment failure.

Independent scratch reproduction using the actual `MultiBackend`, mainnet parameters, a trusted-primary-shaped backend returning an error and one secondary returning 99: `tx_confirmations_final` returned **99**. This demonstrates quorum behavior; no real node deception or fund loss was performed. The secondary's trust metadata does not affect this decision because the quorum function examines the primary slot only.

Fix: base finality acceptance on the **responders' trust**, not merely the configured primary's type. A responding trusted Core can justify the single-source policy; if it abstains, require the intended independent untrusted-view quorum or keep the record unsettled. For a legitimate Core block-scan result on a txindex-less node, preserve explicit validated provenance rather than generally treating a missing finality answer as confirmation. Do not loosen this by simply treating an unknown transaction as confirmed or silently reducing the quorum.

Validation for this recheck: **230 existing Rust tests passed** (209 pact workspace, 21 Satchel), UI production build/lint passed, and four scratch probes executed (three fixed paths verified, one residual demonstrated). No end-to-end suite was rerun in this narrowly focused recheck. No production code changed. Probe: [closure probes](2026-09-10-closure-probes.py); logs in [closure evidence](2026-09-10-closure-evidence/).

## Initial verification at `b5a7e30` (historical)

Reviewed HEAD: `b5a7e302c055a289b81e897bfb789f5601fde09f`, including remediation commit `58dea6c` and the Core-fork compatibility follow-up. This checks the original September 9 findings and their fixes. It does not certify or re-review every claim in the separate September 10 protocol report. Dependency internals remain excluded except the small ownership API introduced specifically to fix original finding 8.

**Conclusion: substantial fixes landed, but the original list cannot yet be marked fully resolved.** Findings 3 and 7 remain partially fixed. Ten other numbered findings have appropriate fixes for their originally identified paths, with evidence and limitations below. Performance items were partly mitigated or explicitly deferred, not all fixed.

No production source was changed during this verification. Test probes use isolated scratch files, mock chain data or private regtest nodes. No real funds were used.

## Remaining gaps, in repair order

### A. Reconstruction still confuses signature validity with finality — original finding 7

Locations: `pact/libswap/src/reconstruct.rs:323`, `:408`; consumers in `pact/libswap/src/engine.rs:7700–7930`.

The added signature verification rejects random fabricated signatures, which is useful. However, the number of confirmations still comes from `tip - provider_reported_height + 1`. No inclusion proof or conservative confirmation quorum is applied at this decision boundary. A correctly signed transaction seen in a mempool, or a replaced/conflicting signed transaction, is not necessarily mined. Its signature cannot authenticate a history height.

**Reproduced with the actual updated classifier in a scratch module:** the same signed v1 redeem and funding transaction returned zero spend confirmations for provider height 0, and six confirmations for provider height 95 with tip 100. No transaction bytes or signature changed. This is a mocked-history reproduction, not an on-chain loss demonstration. Reconciliation still uses this depth to write `settled=true` and retire monitoring.

**Criticality: Medium under the stated untrusted-backend threat model, with potentially high fund-loss impact.** Requires an incorrect or malicious history view during a relevant unsettled/recovery state; it is not an ordinary UI action or an unauthenticated daemon RPC.

Fix: separate spend discovery/authenticity from inclusion/finality. Before latching terminal state, establish the selected spend's depth using independently validated chain evidence or the same conservative responder-quorum policy used by normal settlement. Add a negative regression test using a valid signature with invented positive history heights. Do not mark this finding closed solely because signature verification passes.

### B. The new v1 witness checker accepts the wrong branch's key — original finding 7 remediation

Location: `pact/libswap/src/reconstruct.rs:189`.

The checker accepts a key if its raw bytes or hash appear anywhere in the witness script. A v1 HTLC contains both the redeem and refund keys, but each branch authorizes only one. Finding any script key and verifying its signature is not equivalent to validating the selected branch. The checker also does not execute CLTV or the full witness script.

**Reproduced:** using the actual updated function and existing fixture helpers, a refund-shaped witness signed with the **redeem** key returned `witness_authentic=true` and classified as `Refund`. The real refund branch requires the refund key. This probe establishes an invalid-spend acceptance at the classifier boundary; it does not demonstrate a consensus-valid spend or completed theft.

**Criticality: Medium, conditional.** A random third-party history server cannot forge either key, but a key-holding counterparty working with an untrusted view can supply this class of evidence. This is an incomplete defense in the original reconstruction trust boundary.

Fix: validate against the locally reconstructed HTLC template, selected branch, required key, preimage and transaction locktime/sequence constraints, or use a suitable complete script-validation mechanism. Keep finality validation from A separate: even a fully valid spend need not be mined.

### C. The new coin-removal guard does not match actual RPC records — original finding 3

Locations: `satchel/src/main.rs:934–940`; serialization at `pact/libswap/src/messages.rs:17–23`; RPC wrapping at `pact/pactd/src/main.rs:547`.

`swap_needs_coin` reads `rec[leg]["coin_id"]`. `ChainRef` serializes that field as **`asset`**, and the direct Rust bridge receives the raw RPC payload. The JavaScript-side normalization does not run in this Rust guard. The new unit test uses handcrafted `coin_id` JSON, so it misses the mismatch.

**Reproduced:** passed a record containing a real serialized `ChainRef`, local ownership, state `signed`, and `settled=false` into an extracted copy of the actual guard. It returned false for the required BTC backend. Thus the new protection does not block ordinary coin removal for real swap records.

Additionally, `live_swaps_on_coin` at `:916` treats RPC/listing errors as an empty list. A busy, unreachable or failing daemon is not evidence that no funds require its backend.

**Criticality: Medium.** Reachable through normal Satchel coin removal; no manual RPC or malicious counterparty is required to bypass the intended guard. Financial loss depends on the resulting monitoring interruption and swap state.

Fix: use the actual wire key or deserialize a typed record; fail closed on an unknown listing. Add a test that serializes a real record instead of inventing its JSON shape. A daemon-owned atomic guard is preferable to a UI-side check followed later by configuration mutation/relaunch.

### D. Failed keep-running handoff still closes the window — original finding 3

Locations: `satchel/ui/src/components/ExitGate.tsx:152–160`; `satchel/src/main.rs:1367–1370`, exit handling at `:1859` onward.

The durable `settled` UI predicate is fixed. The separate handoff error path is unchanged: if writing `running-pactd.json` fails, `quit_app` returns before setting the detach flag. The UI catches that error and destroys the window. The exit handler can then stop the managed daemon despite the user selecting keep-running.

**Evidence: source trace, not a desktop filesystem-failure injection. Criticality: Medium, conditional on a handoff write failure while monitoring is required.** Disk-full or permission failures are examples of the trigger, not events observed in this review.

Fix: retain the window and running daemon on a failed keep-running request, surface the error and allow retry. Do not turn failure to detach into stop-and-exit. Add a handoff-write failure test through the quit flow.

## Original-finding disposition

| Original item | Assessment at reviewed HEAD | Evidence / qualification |
| --- | --- | --- |
| 1 — ambiguous RPC replay | Fixed for automatic replay | Failure stages are distinguished; uncertain outcomes replay only an explicit read allowlist. Sent payment requests do not replay. Policy tests pass; no full lost-response desktop injection run. Durable idempotency would still improve manual recovery but is not necessary to close the original automatic-retry defect. |
| 2 — merchant switching | Fixed for reviewed replacement paths | Create, load and unload share a guard; both protocols, unsettled terminal records and listing errors covered; foreign records excluded by engine ownership. Merchant regression test passes. |
| 3 — exit/coin lifecycle | **Partial** | Current local records retain durable `settled` through both UI conversions; completed/refunded with false latch remain active even without progress. Coin-removal wire mismatch and failed-detach fallback remain (C/D). Older-daemon fallback retains old progress semantics. |
| 4 — direct v2 redeem gate | Fixed for original first-reveal bypass | Central gate checks leg B's visibility, script, value, depth and payout ownership. Direct-RPC regtest rejects zero confirmations and succeeds at required depth. Participant custody gate also added. |
| 5 — malformed nonce panic | Fixed for identified input | Exact 12-byte validation; parser-opening catch boundary skips bad mail and advances cursor. Malformed-nonce tests pass. General poison recovery is not itself proof that arbitrary partially completed engine operations are coherent; retain specific invariant checks rather than relying on that claim. |
| 6 — normal v1 settlement uses max | Fixed in normal settlement arms | Normal v1 retirement now calls `tx_confirmations_min`, including the added initiator Completed arm. Alternate reconstruction finality remains finding 7, not closure of that separate path. |
| 7 — reconstruction evidence | **Partial** | Random signature forgery rejected; inclusion/finality still unverified and wrong-branch signature accepted (A/B). |
| 8 — nodeless ownership | Appropriate fix | Adapter delegates to the newly pinned wallet's actual `is_mine`, which returns unknown while locked. Inspected that narrow dependency API; no Core-to-nodeless desktop migration executed. |
| 9 — opposite-chain refund outage | Fixed for original initiator paths | Chain-B work is isolated from due chain-A refund. Regtest passes with B unreachable. |
| 10 — revocation author boundary | Fixed for newly processed deletions | Author travels through conversion, tombstone and conditional removal/own-ledger updates. Cross-author regression passes. Legacy ID-only tombstones remain honored: upgrades do not automatically repair already-poisoned cache history. |
| 11 — foreign amount separator | Fixed for original decimal-separator case | Actual TypeScript probe retains `0.001` under German locale and rejects parsing; English `0,001` likewise rejected. Completed/refunded latch predicates also checked. This is not a general certification of every possible pasted numeric notation. |
| 12 — Core reservation on signing error | Fixed for ordinary returned signing/decoding errors | Error path cancels selected inputs using funded hex. A failed cleanup RPC or process crash cannot be guaranteed to release reservations; those are separate recovery limitations. Source trace; no new signing-failure node probe run. |
| V2 confirmed-input concern | Addressed, including Core-fork fallback | Leg A calls confirmed-only send; leg B enforces minconf or explicitly selects confirmed inputs with add_inputs=false. Queued-leg-A, Litecoin v1 and Litecoin v2 tests pass. This checks confirmed ancestry, not the separate external-RBF concern in the September 10 report. |

## Performance disposition

- Desktop and Corkboard transport timeouts were added, and reprobe now runs off the async executor. These are useful mitigations. Socket read deadlines are inactivity limits, not total operation deadlines.
- The shared registry still covers chain/network work. The original cross-operation blocking finding is **mitigated/deferred**, not fixed by adding timeouts.
- Historical full-record loading/polling, relay/inbox retention and bundle size were **explicitly deferred** in triage. Treat them as accepted scope/risk decisions rather than implemented fixes. Own-addressed mail and a consumption cursor do not bound the total number of retained rows.

## Validation

- Pact workspace: 207 tests passed (175 libswap, two individual vector tests, 3 CLI, 27 pactd).
- Satchel: 21 tests passed; pact-proto: 11; pact-nostr: 8; crier: 17. **264 existing Rust tests passed in total.**
- UI production build and lint passed; existing large-bundle warning remains.
- Five targeted end-to-end scenarios passed: `AdaptorDirectRedeemGate`, `SiblingFundingQueueV2`, `RefundSurvivesOtherChainOutage`, `V1SwapLtcLegB`, `AdaptorRedeemCpfpLtc`. The two CLI-driven v1 scenarios were repeated after explicitly rebuilding the full workspace to ensure current CLI code.
- Three independent scratch probes reproduced remaining defects A/B/C. Their passing assertions mean the **undesired behavior was demonstrated**, not that the fixes are correct.
- TypeScript probe confirmed durable unsettled-state handling and rejection of foreign decimal separators.

The full nine-suite harness was not repeated in this focused verification. No claim is made that every crash, malicious backend, desktop interaction or unrelated finding in the new report was exercised.

Probe source: [2026-09-10-fix-probes.py](2026-09-10-fix-probes.py). Logs: [verification evidence](2026-09-10-fix-verification-evidence/).
