# Review of the protocol remediation batch (2026-09-11)

Reviewed: the uncommitted working tree on `f670cd9` described by [the remediation handoff](2026-09-11-protocol-fixes.md), against [the severity reassessment](2026-09-11-protocol-triage.md) and [the original report](2026-09-10-protocol-to-implementation.md). No production source was changed by this review.

Method: every claimed change was traced in the current code by five parallel reviewers, each with the original finding, the triage row, and the handoff row in hand, and every review ran the batch's own tests plus the full regtest sweep independently. Evidence is cited as `file:line` in the working tree.

## Verdict

**Not ready to commit as is.** The handshake, replay, atomic-creation, claim write-ahead, revocation scoping, desktop hardening and clock changes are real and correct. But the batch was verified against a handful of scenarios, not the sweep, and the sweep fails: the new UTXO-scan throttle turns routine funding discovery into tick errors, and 20 of 65 scenarios across six suites fail on a clean run, including the hard-P2 queue, rescue, takeover and mixed-version cells. Four further defects introduced by the batch need fixing before it ships: a losing late claim leaves a permanent ghost `Completed`, the legacy Core spend lookup errors before its block-scan fallback, full-certificate TLS pinning strands views on routine certificate rotation, and the new funding fee reserve refuses legitimate small legs after the counterparty has already funded.

Everything below is ordered by what blocks a commit.

## Blocking

### B1. The UTXO-scan throttle breaks funding discovery (F2 change)

`pact/libswap/src/chain.rs:1018-1042`. After any `scantxoutset` for a script, the next call within 120 s returns `Err("UTXO scan throttled; retry later")`, and a concurrent caller gets `Err("UTXO scan busy")`. Every engine caller of `find_funding` (v1 `locate_funding`, v2 locate-first guards in `adaptor_fund`, the retry arms, chain-watched leg discovery) propagates that error with `?`, so the record's whole tick becomes an `error` event. On the 30 s scheduler a swap that depends on the scan loses up to three of every four ticks.

Reproduced: on a clean environment the full sweep fails 20 of 65 scenarios across six suites, every one with `find funding: 0 of 1 chain view(s) answered ... UTXO scan throttled; retry later` as the only tick event (see the tally under Independent verification). The handoff's verification ran only `BroadcastRecoveryV1`, `AdaptorSwap`, `AdaptorCorkboardSwap` and the two CPFP cells, none of which hits the throttle.

Fix: cache the result, not just the time. A throttled negative scan should return the cached `Ok(None)`; a positive result should be returned immediately and never throttled; a concurrent caller should wait for the slot, not fail. Also reconsider the 30 s `scantxoutset` read deadline (`rpc.rs:257`), which is below typical BTC-mainnet scan times; a timed-out scan keeps running in Core, the throttle re-arms, and funding detection may never complete.

### B2. Legacy Core spend lookup errors before the block-scan fallback (F1 change)

`chain.rs:1161-1186`. On nodes without `gettxspendingprevout` (Litecoin Core 0.21, any Core before 24) the bounded mempool walk does `ensure!(ids.len() <= 128)` and `ensure!` on a 2 s budget, and both return `Err` before the mined-spend block scan below is reached. On a Core-only LTC view with an ordinary mempool, `find_spend_witness` errors every tick, so a participant never learns the revealed secret and the initiator refunds after T1. The previous code was slow but complete; regtest cannot catch this (small mempool).

Fix: on a large or slow legacy mempool, skip to the (now bounded) block scan instead of erroring.

### B3. A losing late claim persists a permanent ghost `Completed` (N4 change)

`engine.rs:5076-5093` (v1 participant) and `3268-3279` (v2): the claim now writes `state = Completed` before broadcast, and the tick arms that call it (`8909-8918`, v2 `3952-3955`) no longer check that leg A is unspent. In the exact case the N4 fix targets, a claim after T1, the initiator may already have refunded leg A. The broadcast then fails with -25, but the record is `Completed`: reconcile treats it as terminal, `adopt_settlement_winner` emits `settlement-conflict` every tick, and the retained `claim_pending` marker rebroadcasts the dead claim every tick. Related: the v2 arms now also trigger `adaptor_redeem` when the committed outpoint is gone but a same-value replacement exists (`3671-3674`, `3972-3975`), which can only produce the same ghost because the MuSig2 signature is bound to the old outpoint.

Fix: check the leg is unspent before the write-ahead, and on a definitive -25 roll the record back or mark it for reconcile instead of leaving `Completed`.

### B4. Full-certificate TOFU pins strand views on routine rotation (N11 change)

`vendor/electrum-btcx/src/backend.rs:249-299`. The custom verifier now verifies the handshake signature with real crypto and pins the SHA-256 of the full end-entity certificate on first contact, failing closed on any change. That is sound against an on-path attacker after first contact. It is also a liveness hazard: the default mainnet view list is 13 public `ssl://` servers, mostly on Let's Encrypt with 60-to-90-day renewals, and there is no RPC or UI to inspect or clear a pin. Every renewal makes that view fail closed with an unactionable error; if enough views rotate mid-swap the engine loses chain access before a deadline. Certificates with X.509 v1 also now fail inside the signature helper.

Fix: validate CA-issued certificates against a root store first (webpki-roots or platform), pin only self-signed ones or pin the SPKI, and add a "forget pin" affordance. The handbook still promises protection against "a single lying server" without the first-contact caveat.

### B5. The funding fee reserve refuses legitimate legs, after the counterparty funded (N9/P2 change)

`engine.rs:1276-1301`. The non-regtest reserve minimum is `max(3 × rate, 20) sat/vB × 155 vB + 546`, which at the 20 sat/vB floor is 3,646 sat, above `MIN_LEG_VALUE_SAT` (3,430), so legs the offer and take gates accept are refused at funding. The economic cap `rate_kvb <= amount / 2` requires a leg of at least 2,000 sat per sat/vB of market rate (200,000 sat at 100 sat/vB) and refuses ordinary small mainnet swaps. Neither check runs in the offer or take pre-flight, so the participant discovers the refusal in `adaptor_build_leg_b` after leg A is confirmed; the initiator's funds are then locked until T1 and the error repeats every tick with no abort or notice to the counterparty. No unit test covers `funding_fee`, the reserve, or the cap.

Fix: align the reserve floor with the leg minimum, move both checks into the offer and take pre-flight, and add tests.

### B6. The verification claim in the handoff is not sufficient

The handoff reports unit tests, clippy, audits, and five regtest scenarios. The full nine-suite sweep is the project's merge gate and was not run; it fails on this tree. The sweep must be green before this batch is committed.

## Should fix before release

- **N5 v1 timeout bypasses the send marker.** The C8 arm at `engine.rs:9022-9108` retries `fund()`, swallows the error, and calls `abort()`, which has no `funding_may_exist` check (`12087-12097`). A swap whose send may have gone out is aborted and tombstoned. Any send error other than insufficient funds also leaves the marker `"pending"` forever.
- **N5 v2 leg A wedge.** The built leg-A transaction is persisted under `funding_tx:{id}:a` (`2937`) and never cleared or unlocked; if its inputs are spent elsewhere (Core `lockunspent` is memory-only) every retry gets -25, `adaptor_abort` refuses, the C8 arm skips, and only a manual meta deletion recovers. `wallet_cancel_funding` is only wired for leg B (`3094-3102`).
- **N6 refunds are not write-ahead.** v1 `refund()` broadcasts at `5189` then writes; v2 `adaptor_refund` broadcasts at `3349` then writes. The handoff's "both protocols persist claim bytes and state before broadcast" holds for redeems only. `claim_pending` is also never cleared by a refund, so an initiator whose reveal failed and who later refunds rebroadcasts the chain-A refund to chain B every tick (`8641-8646`).
- **N10 reachability.** The exact-value replacement heal exists only in `adaptor_refund` (`3313-3322`); the initiator's Signed-state refund arm (`3800-3811`) still requires the original outpoint to be live, so after an external replacement the heal is reached only pre-Signed or by manual RPC. The handoff's open item 4 is accurate.
- **N13 quota DoS.** Per-recipient quotas key on `to`, not `from`; one key posting to 32 arbitrary recipients fills the 8,192-row global relay quota and closes the board to honest writers for seven days (`corkboard/src/main.rs:272-280`). Offers likewise via fresh keys. Add a per-sender quota or rate limit.
- **N1 hardening.** Deterministic conflicts (`late …`, `conflicting …`, wrong leg) are plain `ensure!`, so a pinned counterparty can cost the receiver ten head-of-line-blocked relay passes per bad message; tag them permanent like the v1 accept duplicate. The old handler recorded `funding_*_height` for the counterparty's leg; the new one does not, which silently disables the reconcile trigger at `3787` for driven initiators.
- **P1 fee acceptance.** `ensure_permanent!(body.redeem_feerate_a >= local estimate)` at `2071-2074` aborts a swap between honest parties on a 1 sat/vB estimator skew. Package rescue is skipped entirely when the estimate says the parent already meets target (`4172-4179`), and while unrelayable two `submitpackage` calls plus one error event fire per tick.
- **Transport deadlines.** The new 10 s RPC read deadline (`rpc.rs:257-278`) is tight for `sendtoaddress`, `fundrawtransaction`, `signrawtransactionwithwallet` and `listtransactions 1000`; a client-side timeout on a server-side success is exactly the N5 ambiguous-send case and this raises its frequency.
- **N20 gaps.** The audit workflow is still schedule-only, not a PR gate. The Core credentials file `coin-arguments.json` is written 0600 but never deleted. Release signatures and attestation remain absent.
- **N3 residual.** `old_tx.input[0]` at `4452` and `9371` cannot panic (every adopted spend has a located input) but can pick the wrong input of a multi-input adopted transaction; select by outpoint.
- **N15 coverage.** Wallet databases live in a subdirectory that is not repaired; only the 0700 parent protects them. The seed file is created 0644 until the next open. `write_private` has a symlink TOCTOU.
- **P3 not fixed.** `spec/protocol.md` §7.4 still lets Bob fund until T2−3h while Alice aborts at T2−3h, and `spec/protocol-v2.md:190` still forbids funding before a verified adaptor signature, which the code contradicts. The handoff's "adaptor exchange ordering" claim is not in the diff.

## Verified

| Finding | Status | Notes |
|---|---|---|
| N1 | Verified | Sender pinned, wrong leg rejected, phase gated, pointers immutable once set or once a nonce session exists, exact duplicates idempotent, nonce and partial ordering enforced, no consumed session reopened. Tests cover the post-Signed branches only. |
| N2 | Verified | One transaction: `INSERT … WHERE NOT EXISTS` across both tables and the `used_swap:` / `purged_foreign:` markers, then a plain `INSERT`. All four creation sites use it. A replayed init leaves the live record byte-identical (tested at engine level for v2, store level for both). |
| N3 | Verified as claimed | Zero `output[0]` sites remain; each is `.first().context(...)?`. |
| N7 | Verified | Leg A via `wallet_build_funding`, persisted before broadcast, resent as the same bytes; confirmed-only and non-RBF on both wallets; `FundingQueued` classification intact. Fee units consistent (sat/kvB inside, converted at each wallet boundary). |
| N8 | Verified | Commitment refuses without both sweeps and a locally owned payout; `None` from a locked wallet refuses. Placeholders at handshake are deliberate. |
| N12 | Verified | Refund clock needs one responder and takes the minimum; reveal clocks keep the quorum and the maximum. Safe direction. |
| N14 | Verified | `tauri-plugin-single-instance`; second instance exits before touching the daemon; no stale-lock case found. One instance across networks (documented). |
| N16 | Verified | `Url::parse`, http(s) with host only, `rundll32 url.dll,FileProtocolHandler`; no `cmd.exe`. |
| N17 | Verified | Locale-aware parse in the UI; server rejects non-finite, zero, or >500 sat/vB; bdk clamps every send path. |
| N18 | Verified | `stop` RPC, 100 ms polling up to 12 s, then kill; daemon de-lists and flushes within the window in the normal case. |
| N19 | Verified | Author read from the envelope JSON, so no migration; upsert and active selection scoped by `(d_tag, from)`; cross-author test present. Legacy id-only tombstones still honored (acknowledged). |
| N6 (redeems) | Verified | Marker set, record written, then broadcast; retry rebroadcasts the current `final_tx_hex` (never a pre-bump version); marker cleared on success; lost-write recovery adopts an authenticated prior claim. |
| F3 | Verified | `OnceCell<Arc<PactSeed>>`, cleared on create/import; no stale-key path. |
| F6 | Verified | `useRef` in-flight flags reset in `finally`. |
| P4 | Verified | Scoped derivation formula matches `keys.rs`; no vectors added. |

## Independent verification

| Check | Result |
|---|---|
| pact workspace tests (`cargo test --workspace`) | 212 passed (180 libswap, 2 vectors, 3 pact-cli, 27 pactd) |
| corkboard / satchel / crier / pact-proto / pact-nostr | 2 / 21 / 17 / 11 / 8 passed |
| clippy `-D warnings`, every crate | clean |
| UI `tsc` build and eslint | clean |
| Full regtest sweep (clean environment, all nine suites) | **20 of 65 scenarios fail, every one on the scan throttle (B1)**; the other 45 pass |

Per suite on the clean run: self-test 3/3, multi-machine 1/1, v1 11/24, rescue 6/8, v2 9/10, nodeless 2/4, follow 3/3, takeover 9/10, upgrade 2/3. Every failing scenario shows the same tick error, `find funding: 0 of 1 chain view(s) answered ... UTXO scan throttled; retry later`, between 26 and 55 times per scenario; the failures are: v1 `DaemonAutopilotSwap`, `ChainWatchedFunding`, `FundingFeeBumpV1`, `FundingRbfPointerResync`, `TakerDisplayFollowsRbf`, `SettlementRbfRaceRedeem`, `SiblingFundingQueueV1`, `FundingBumpDescendantBelt`, `CorkboardSwap`, `BoardResetRecovery`, `NostrRelaySwap`, `ConcurrentDrainNoDoubleSend`, `PrivateOfferSwap`; rescue `RescueMakerFundedAV1`, `RescueTakerPostRevealV1`; v2 `SiblingFundingQueueV2`; nodeless `V1NodelessMaker`, `V1NodelessBothSides`; takeover `PrefundTakeoverAbortsBlind` (the pre-funding abort arm rediscovers the leg first and never reaches the abort); upgrade `MixedVersionSwapV1`. The handoff's own five scenarios (`BroadcastRecoveryV1`, `AdaptorSwap`, `AdaptorCorkboardSwap`, both CPFP cells) all pass here too; they are simply the ones that never call `find_funding` twice within two minutes.

Environment note: three earlier sweep attempts were contaminated (a review probe rebuilt the shared node cache concurrently, and stale regtest daemons from the interrupted runs then poisoned the next cache build with a future-dated tip). Those runs are not counted; every scenario listed above was re-run on a rebuilt cache with no stale daemons, and the environment-only failures (`CompleteSwap`, `BroadcastRecoveryV1`, four rescue cells, and the `time-too-old` setup failures) all pass on re-run.

## Process notes

- Corkboard and clients must be upgraded in lockstep (`/v1/relay` now requires a signed `relay_post`); the Nostr transport is unaffected.
- `vendor/electrum-btcx` is a patched copy of the upstream crate wired through `[patch]`. It has no lockfile of its own and is outside the audit set; the patch must be upstreamed or maintained.
- `Store` now holds `std::cell::OnceCell`, which makes it `!Sync`; it compiles only because the engine sits behind the registry mutex. Consider `std::sync::OnceLock`.
- `overflow-checks = true` in release means an arithmetic overflow now panics inside a tick under the registry lock instead of wrapping; acceptable given the poison recovery, but worth knowing.
