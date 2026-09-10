# Application security and performance review — 2026-09-09

Reviewed revision: `8ba68dbb60d09272796bc4883a3d4bd32e196f7a`.

This review pass is complete. It combines source review, targeted reproductions and automated end-to-end validation. It is not a guarantee that all bugs have been found. No production source was changed. Dependency internals and dependency vulnerability auditing are excluded from this pass. Earlier application findings are consolidated here so the review can be read without the chat history. Reproduction scripts and test logs are saved in [the evidence directory](2026-09-09-evidence/README.md).

## Priority findings

### 1. High — the desktop RPC bridge can repeat a payment after an ambiguous failure

Location: `satchel/src/main.rs:1068`, `pactd_rpc`; transport at `:995`.

After **any** first-call error, the bridge probes the daemon and retries the same method and parameters. This includes a response lost after a successful operation, not just rejected authentication. The cookie need not have changed. A non-idempotent operation such as a wallet send can therefore execute twice. This is a code-path finding; a duplicate on-chain payment was not executed during this review.

Remediation: distinguish authentication rejection and pre-dispatch connection failures from uncertain outcomes. Never automatically replay money-moving calls after an uncertain outcome. Assign a persistent operation ID before submission and have the daemon transactionally deduplicate it; provide an operation-status query for recovery. Test a server that commits an operation and closes before returning its response.

### 2. High — merchant changes can discard the engine monitoring unfinished swaps

Locations: `pact/pactd/src/merchants.rs:310` (`create`), `:404` (`ensure_safe_to_switch_away`), `:529` (`is_terminal`).

Creating a merchant replaces the active engine without invoking the safety guard. Loading/unloading invokes a guard that checks only v1 records, ignores listing errors, and treats Completed/Refunded as terminal without checking settlement. The old engine no longer watches those timelocks.

Reproduced against the actual daemon, in both debug and freshly built release configurations, using valid synthetic persisted records and private temporary data directories:

- Control: `loadmerchant` rejected an active v1 record.
- `createmerchant` switched away from that same active v1 record.
- `loadmerchant` switched away from a signed v2 record.
- `loadmerchant` switched away from a completed v1 record with `settled=false`.

These were daemon integration probes, not funded on-chain swaps. Remediation: one daemon-owned safety predicate covering both protocols, driven ownership, unsettled transactions and unreadable state; apply it to every engine replacement. Alternatively keep all exposed merchants' engines running.

### 3. High — shutdown safety relies on optional UI progress rather than settlement state

Locations: `satchel/ui/src/format.ts:428`, `satchel/ui/src/AppContext.tsx:238`, `satchel/ui/src/components/ExitGate.tsx:108`, `satchel/src/main.rs:1128`.

`isFinalizing` recognizes only `completed` plus a transient settlement progress entry. A completed record with missing progress is considered finished; a refunded record is considered finished even with settlement progress. Progress retrieval can fail independently, and progress is rebuilt after startup. The exit gate uses this predicate to select the clean-exit path, which can stop the managed daemon.

Actual TypeScript predicate reproduction: completed/no progress → inactive; completed/settlement → active; refunded/settlement → inactive. No full desktop click-through was performed.

The error fallback at `ExitGate.tsx:152` also destroys the window when the quit command fails. If a requested keep-running handoff fails before the detach flag is set, the ordinary exit handler can stop the daemon instead.

Remediation: enforce stop/detach safety in the daemon using durable state, return that verdict to the UI, and preserve the running engine when detach fails. Apply the same protection to coin removal (`satchel/src/main.rs:848`), which currently removes configuration and relaunches without a live-swap guard. Its generic warning does not enforce timelock safety.

### 4. High — direct v2 redeem bypasses the scheduler's confirmation gate

Locations: `pact/libswap/src/engine.rs:2849` (`adaptor_redeem`), scheduler checks around `:3380`; RPC dispatch `pact/pactd/src/main.rs:1622`.

The direct redeem method checks protocol state and deadline but does not enforce the funding-output/depth checks used by the scheduler before revealing the adaptor secret. The authenticated RPC surface reaches this weaker method directly. This is an API safety failure; it does not require an unauthenticated remote caller to invoke RPC.

Reproduced on local regtest: leg B had zero confirmations, the record required one, and direct `adaptorredeem` accepted the request and transitioned to `redeemed_b`.

Remediation: enforce the common funding value/script, confirmation and payout-ownership gates inside the money-moving method itself. Scheduler checks should be supplemental. Add direct-RPC negative tests for zero depth, mismatched output and foreign payout.

### 5. High — malformed encrypted envelope nonce can panic the daemon processing path

Locations: `pact-proto/src/seal.rs:73`, relay opening at `pact/libswap/src/engine.rs:10373`; daemon registry locking in `pact/pactd/src/main.rs`.

The decoded nonce slice is converted to a fixed-size nonce without checking its length. Valid hexadecimal of an invalid length reaches a panicking conversion instead of a recoverable parse error. Relay processing happens while the registry mutex is held; unwinding poisons that mutex and subsequent paths using `expect` can fail too, preventing swap monitoring.

Isolated compiled reproduction: decoded lengths 0, 1, 11 and 13 panic; length 12 follows the ordinary error path. This demonstrates the parser defect; an external relay attack was not executed.

Remediation: validate exact fixed-field sizes before conversion, return structured errors, and test malformed envelopes through the daemon processing boundary. Avoid making malformed remote input capable of poisoning shared engine state.

### 6. High — v1 settlement uses the most optimistic backend confirmation count

Locations: `pact/libswap/src/engine.rs:8237`, `:8267`, `:8282`; aggregation `pact/libswap/src/chain.rs:2095`.

The v1 settlement branches consume the maximum confirmation count across backends. A single incorrect backend can satisfy the settlement threshold while other views disagree, letting the engine retire monitoring prematurely. The v2 settlement path around `engine.rs:3664` uses the conservative minimum mechanism instead.

Evidence: source trace; no dishonest-server end-to-end reproduction. Remediation: use the intended conservative settlement policy consistently across protocols and recovery paths. Test backend disagreement explicitly.

### 7. High — reconstructed spend/finality evidence is weaker than the decision it authorizes

Locations: `pact/libswap/src/reconstruct.rs:160` and `:218`, `classify_v1_spend`, `classify_v2_spend`; retirement consumers such as `pact/libswap/src/engine.rs:7513`.

Reconstruction combines a selected backend's script history heights, fetched transactions and witness shape to classify a leg as spent and sufficiently confirmed. V1 branch shape and v2 signature-length checks are not full spend validation. Verifying the fetched transaction's own hash does not establish its inclusion in the claimed block. An untrusted history provider can therefore supply evidence insufficient for the finality decision that consumes it.

Evidence: source trace, not a completed fabricated-history reproduction. This is an application trust-boundary finding, independent of dependency implementation defects. Remediation: tie finality to verified chain evidence or a clearly enforced trusted-node model; check confirmations conservatively and avoid retiring a swap on uncorroborated historical assertions.

### 8. High, conditional — nodeless takeover assumes ownership of every payout address

Locations: application adapter `pact/libswap/src/wallet_bdk.rs:139`; `pact/libswap/src/engine.rs:9414`, `:9483`.

`wallet_owns_address` returns `Some(true)` for every address. The takeover gate relies on this answer to decide whether cooperative redemption pays a wallet controlled on the recovering machine. A transcript originally made with a Core wallet can contain a sweep address unrelated to the merchant seed. Restoring that merchant with a nodeless wallet does not recover the Core wallet's private keys, yet this adapter reports ownership and permits cooperative completion instead of refund-only recovery.

Evidence: source trace. The Core-to-nodeless migration scenario was not executed end-to-end. Remediation: check actual script ownership/derivability; until a precise check is available, return unknown and preserve refund-only behavior. Same merchant identity alone is not proof of sweep-address ownership.

### 9. Medium — failure of the other chain prevents an otherwise available v1 refund

Location: `pact/libswap/src/engine.rs:8190` and the subsequent refund dispatch.

The automatic v1 path obtains information from the opposite backend before attempting the due refund on the funded chain. An unavailable opposite backend returns early. This couples recovery to a service that is unnecessary for broadcasting the refund.

Reproduced on isolated regtest: the funded chain's median time exceeded the refund deadline; stopping the opposite backend made `tick` error, while direct refund succeeded and entered `refunded`.

Remediation: attempt each locally actionable refund independently and report other-chain failures without suppressing it. Cover both roles and both backend-outage directions.

### 10. Medium — Nostr revocation cache discards the author's identity

Locations: `pact-nostr/src/lib.rs` (`revoked_offer_from_event`, around `:199`), `pact/pactd/src/nostr_service.rs:126`.

The event conversion checks an author's own coordinate but passes only the swap ID onward. Daemon revocation storage and cache removal then key on swap ID alone. A valid deletion for another author's same-ID coordinate can suppress a victim offer. This affects availability/discovery; the reviewed path does not itself move funds. The crier implementation retains maker identity, showing the intended distinction.

Evidence: source trace. Remediation: carry and check `(author, identifier)` throughout conversion, cache lookup and tombstone storage, including locally published offers.

### 11. Medium — amount sanitization can multiply pasted values

Location: `satchel/ui/src/format.ts:66`; consumed by wallet-send and offer amount inputs.

The input sanitizer removes foreign decimal separators before strict parsing sees them. With German locale, pasted `0.001` becomes `0001` and the wire amount becomes 1 coin; with English locale, `0,001` has the same result. The send confirmation displays the transformed amount, so this is not an invisible post-confirmation change, but it creates a substantial avoidable user-error risk.

Reproduced by executing the actual TypeScript formatting functions under mocked locales. Remediation: reject ambiguous/foreign separators or explicitly normalize a single unambiguous pasted decimal; never silently delete a character that changes numerical magnitude.

### 12. Medium — Core funding construction leaves input reservations on error

Location: `pact/libswap/src/chain.rs:1232–1295`.

`fundrawtransaction` locks selected inputs, then signing and local decoding can return errors. Those error paths do not unlock the selected inputs, and no successful funding record is returned for the normal cancellation path to use. A signing failure can strand spendable wallet inputs until manual unlock or node restart.

Evidence: source trace. Remediation: retain the funded transaction immediately, release its reservations on every subsequent error, and transfer reservation ownership only after durable storage. Test an incomplete signing response and a signing RPC failure.

## Performance and availability

### Shared registry lock holds unrelated requests behind network work

Locations: `pact/pactd/src/main.rs:207`, scheduler around `:2438`.

Network-dependent engine work and board synchronization execute while holding the shared registry lock. A slow external call delays unrelated RPCs and timelock work. Reproduced using a private local board stub with a two-second poll delay: an otherwise empty `listswaps` request blocked for **2.001 seconds** behind `tick`.

Move network waits outside the global registry critical section, use bounded timeouts and cancellation, and preserve per-swap serialization where correctness requires it. Measure refund latency under a slow or failing board, not only overall request throughput.

### History costs scale with all past swaps

Locations: `pact/libswap/src/store.rs:575`, `:1030`; `engine.rs:4884`, `:5088`; UI polling in `satchel/ui/src/AppContext.tsx` and rendering in `screens/SwapsScreen.tsx`.

Tick and progress refresh load full historical records. The UI repeatedly fetches all swaps, including raw transaction data, and renders unpaginated history. A fresh optimized release build produced these local measurements:

| Settled records | Tick median | listswaps median | listswaps response bytes |
| ---: | ---: | ---: | ---: |
| 0 | 1.11 ms | 0.56 ms | 36 |
| 1,000 | 7.83 ms | 35.78 ms | 1,970,035 |
| 10,000 | 72.48 ms | 317.83 ms | 19,700,035 |

Method: actual daemon, isolated SQLite database, synthetic valid settled v1 records, five warm samples per operation, no coin backends. Each record had two 250-byte transaction blobs represented as hex. Timings include local HTTP and Python JSON parsing, and were collected while the end-to-end suite also ran. They are illustrative, not controlled production throughput or GUI frame-time measurements. At a four-second poll interval the 10,000-row response alone represents about 4.9 MB/s of repeated local JSON payload before other requests.

Use an indexed active/unsettled query for the scheduler, paginated compact history summaries for the UI, and an explicit detail endpoint for raw transactions. Fetch changes incrementally and virtualize long lists.

### Transport waits are unbounded in the desktop bridge

Location: `satchel/src/main.rs:995`, plus `health_ok`/`probe_adoptable`.

The manual TCP client sets no connection/read/write deadline and reads to EOF. A stalled peer can retain a blocking task indefinitely; health reprobes also perform synchronous work from an async command path. Add bounded transport deadlines and keep blocking probes off async executor threads. Combine this with the safe retry policy in finding 1.

### Relay storage and catch-up grow without a bounded lifecycle

Locations: `corkboard/src/main.rs:205`; daemon Nostr inbox persistence/consumption at `pact/libswap/src/store.rs:762`, `:773`.

The reviewed relay ingestion path lacks a complete expiry/deletion/quota lifecycle, while persisted inbox history accumulates. Corkboard already caps individual blob size and returns at most 100 messages per poll; those limits do not cap total retained storage. The daemon Nostr inbox query has no batch limit. These create disk-growth and catch-up costs, especially for publicly reachable services. Retain the existing request limits, add global/per-recipient storage budgets and a retention policy that preserves required rescue data, and batch the daemon inbox consumption. Validate behavior under a full disk and a large inbox. This was a source review, not an Internet load test.

The UI production build also emits a large-chunk warning: main bundle approximately 2,009 kB, 647 kB gzip. Split infrequently used screens/heavy modules and measure startup before selecting further changes.

## Additional issue requiring a focused regression test

The confirmed-input guarantee documented in `docs/postmortems/2026-08-09-orphaned-funding-sibling-rbf.md` is not explicit in Core v2 funding construction. `chain.rs:1264` passes `lockUnspents`, `fee_rate` and `replaceable=false` to `fundrawtransaction`, but no confirmed-input selection restriction; v1's separate `wallet_send_confirmed` explicitly supplies `minconf=1`. Also inspect v2 leg A's ordinary `wallet_send` path (`engine.rs:2649`). Verify with a wallet whose only available balance is unconfirmed change. This is recorded as an unresolved regression concern, not a reproduced orphaning incident. Non-replaceability of the child does not by itself establish confirmed ancestry.

## Validation and coverage

Source review covered protocol implementation, wallet/chain application adapters, scheduler/recovery/settlement, persistence, authenticated daemon RPC, merchant lifecycle, desktop bridge and UI transaction/shutdown paths, corkboard and Nostr application mapping, and crier behavior. Examination depth varied; this was not a formal cryptographic proof or an exhaustive test of every line or fault interleaving.

- Rust tests across the project: **255 passed** in the review session (202 pact workspace, 10 pact-proto, 7 pact-nostr, 17 crier, 19 Satchel). Corkboard compiled and its test target contained zero tests.
- UI production build and lint passed.
- Fresh `cargo build --release --locked -p pactd` passed.
- Targeted reproductions: nonce-length panic; direct v2 zero-depth redeem; opposite-backend refund outage; merchant switching; UI amount/terminal predicates; registry-lock blocking; history scaling.
- Full harness: **61 scenarios across all nine registered suites passed across the original run and focused UTF-8 reruns**. The original run had 57 passes and four failures, all `UnicodeEncodeError` while printing an arrow under Windows CP1252. The four affected scenarios passed with `PYTHONIOENCODING=utf-8`: `RescueTakerPostRevealV2`, `AdaptorSwap`, `OwnerReturnsAfterTakeoverV1`, and `OwnerReturnsAfterTakeoverV2`. The original runner correctly exited nonzero; this was not a single all-green invocation. Original and rerun logs are preserved. Set UTF-8 explicitly in Windows harness execution to avoid these spurious failures.

| End-to-end suite | Unique scenarios passed, including focused reruns |
| --- | ---: |
| Framework self-test | 3 |
| Multiple machines | 1 |
| V1 swaps | 21 |
| Rescue | 8 |
| V2 adaptor swaps | 8 |
| Nodeless | 4 |
| Follow | 3 |
| Takeover | 10 |
| Upgrade/mixed versions | 3 |

No production/mainnet funds were used. No dependency security conclusions are included. Manual GUI interaction, long-duration production load, fabricated-history attacks and Core-to-nodeless payout migration were not exercised. Findings marked source trace should receive targeted regression tests as part of remediation.

## Suggested repair order

First unify the daemon's safety checks for every stop/switch/redeem path, remove ambiguous payment replay, and make malformed envelopes recoverable errors. Then repair settlement/reconstruction trust decisions and payout ownership. Next isolate refund work from unrelated outages and network locks. Follow with amount parsing and reservation cleanup, then active-state queries, compact pagination and retention limits. Existing happy-path tests passing does not invalidate the failure-path findings above.
