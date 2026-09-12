# Protocol-to-implementation review — 2026-09-10

Reviewed revision: `8ba68dbb60d09272796bc4883a3d4bd32e196f7a` (branch `fix/settled-latch-scheduler-stall`).

Scope: the protocol as specified (`spec/`) and as implemented, end to end: cryptographic core, handshake, engine state machines (v1 and v2), scheduler, chain backends and wallets, key and secret storage, wire format and transports (Nostr, Corkboard), daemon RPC surface, desktop shell and UI, dependencies and release pipeline. Focus: security and performance.

This document reports **only findings that the 2026-09-09 review does not cover**. Every finding in that record was re-verified at this revision and is still present; none is repeated here. Where new evidence materially changes a prior finding, it is noted in one line under "Prior findings" at the end. No production source was changed. Reproduction sources and audit output are in [the evidence directory](2026-09-10-evidence/README.md).

Evidence strength is stated per finding: *reproduced* (a probe executed the defect), *source trace* (the path was read end to end but not executed), or *measured*.

## Verdict in brief

The cryptography is sound. The v1 script, the v2 MuSig2 aggregation, tweak, adaptor verification, nonce write-ahead discipline and the depth gates before any money moves are all correct and tested. The losses found are in three places the cryptography cannot protect: handshake messages that are applied without state or role gates, crash windows between a broadcast and its database write, and finality or clock decisions that trust a single chain view. One unauthenticated network input can also stop the daemon permanently.

Two findings let a **malicious counterparty take both legs of a v2 swap** with no precondition beyond being the counterparty. Those should be fixed before the next release.

## Protocol layer (specification)

### P1. Medium — the initiator unilaterally fixes the participant's non-bumpable redeem fee

`messages.rs:145-150`, `engine.rs:1997-2000`, `engine.rs:1265-1272`.

The v2 `init` carries `redeem_feerate_a`, the rate committed into the participant's key-path redeem of leg A. That signature cannot be RBF-bumped. The participant only range-checks the value to `[1, 500]`. The participant's rescue is a CPFP child, but the child is attempted only after the parent is in the mempool (`engine.rs:3781-3860`) and no package submission is used. A malicious initiator sends rate 1 and times the reveal for a moment when the mempool minimum exceeds it. The participant's parent is rejected, CPFP is impossible, and the initiator refunds leg A at T1 having already claimed leg B. Source trace.

Fix: the participant rejects an `init` whose `redeem_feerate_a` is below its own estimate, or the two sides negotiate the maximum of both estimates. Submit parent and child as a package where the node supports it.

### P2. Medium — fee economics: the minimum leg can become unrefundable

`swap.rs:44-60`, `fee_policy.rs:28,183-186`, `swap.rs:151-155`.

`MIN_LEG_VALUE_SAT` is 3,430 sat, sized as 330 sat plus 155 vB at 20 sat/vB. Refund and redeem fees both come out of the leg. At 50 sat/vB a 146 vB refund costs 7,300 sat, more than the leg, so `build_refund_tx` fails its dust guard and the leg cannot be refunded until fees fall. The claim rate is `min(market, value/vsize)`, so at the cap the refund goes out at about 21 sat/vB regardless of market. After T both branches stay valid, so a refund that never confirms is a race the secret holder can win later. Source trace.

Fix: size the gate from a live conservative rate (for example three times the current estimate, floor 20 sat/vB). After T, prioritise confirmation over the value cap. Document that a leg smaller than its own fee is a permanent-loss risk.

### P3. Medium — spec §7.4 is internally inconsistent and diverges from the engine

`spec/protocol.md` §7.4; `engine.rs:4319-4326`, `8132`, `2868`; `spec/protocol-v2.md` §7; `engine.rs:2649-2657`.

The spec lets Bob fund until `T2 − 3h` but tells Alice to abort if leg B is not `N_B` deep by `T2 − 3h`, so Bob's real deadline is earlier and unstated. The engine enforces a 3 h fund margin and a 2 h reveal margin with no give-up rule. With `N_B = 6` on BTC the engine leaves about one hour for six blocks, which will fail liveness often and cost Bob funding plus refund fees. The v2 spec says a party MUST NOT broadcast funding until it holds a verified adaptor signature, but the initiator broadcasts leg A immediately. Source trace.

Fix: define the fund deadline as `T2 − reveal_margin − N_B × expected_block_time`, or drop the abort clause. Rewrite v2 §7 to describe two-phase participant funding versus immediate initiator funding.

### P4. Medium — the spec omits the machine-scope derivation levels; "seed-only recovery" is no longer true

`keys.rs:104-160`; `spec/protocol.md` §4.1, §4.3, §11; `spec/protocol-v2.md` §3, §9.

Every counter-based path now carries two hardened 31-bit scope levels: `m/7228'/1'/coin'/scope_hi'/scope_lo'/i'`. The spec paths and the claim that the seed alone re-derives `s` and `t` are wrong unless `machine.json` or the relay snapshot carrying `derive_scope` survives. A third-party implementer following the spec cannot reproduce Pact's recovery. The vectors pin only the legacy scope. Source trace.

Fix: document the scope levels and the `0 = legacy` marker, state the snapshot dependency, add scoped and anchored-participant vectors.

### P5. Low — spec and code nits

- v2 spec §8 says the key-path redeem is not bumpable and signals RBF; the code CPFP-bumps it (`engine.rs:3781-3865`) and sets `ENABLE_LOCKTIME_NO_RBF` (`taproot.rs:158-175`). The handbook is right, the spec is stale.
- Spec §7.3 defaults are 6 h / 12 h; the shipped default preset is 12 h / 24 h (`OfferForm.tsx:57-60`).
- v2 §3.1 says `t = TaggedHash(...) mod n`; the code errors on an out-of-range hash (`keys.rs:295-300`).
- The nonce is built with `SecNonce::build_with_pubkey` without the secret key (`adaptor_engine.rs:45-49`); the spec says nonces are bound to the signer's key. Fine with a good RNG. A cloned live datadir on two machines would reuse a secnonce; the spec should forbid copying a live datadir.
- Vectors contain no sighash or signed-transaction vectors and no anchored-participant derivations; the v1 vector uses the mainnet coin type with regtest addresses.
- The envelope has no timestamp or nonce by design, so replay defence rests entirely on `swap_id` uniqueness plus monotonic handlers. That requirement should be normative, because findings N1 and N2 below are exactly where it is not met.
- No clock-divergence refusal exists although §10 recommends one (`engine.rs:510`, `9783`).

## Implementation security findings

### N1. Critical — a counterparty can overwrite our v2 funding pointers after `Signed`; the participant loses both legs

`engine.rs:2510-2535` (`recv_adaptor`, `"funding_ready"`); consumers `engine.rs:3573` (claim arm), `3611` (refund arm), `2934` (manual redeem), `7637-7745` (reconcile never re-points). **Reproduced** (two independent in-process reproductions: `handshake_probe.rs` and `funding_ready_overwrite_test.diff`).

The `funding_ready` handler has no state gate, no check that `chain` is the sender's own leg, and no "pointer already set" check. It runs before the drive-state gate. A pinned counterparty can therefore rewrite either leg's outpoint at any time.

Attack: after Bob's real `funding_ready(b)` and both sides reach `Signed`, Alice sends `funding_ready{chain:"b", txid: bogus}`. Bob's tick still broadcasts his real leg B from the stored hex. Alice waits `n_b`, redeems B and reveals `t`. Bob's claim arm scans for a spend of the bogus outpoint forever; his refund arm sees the bogus output missing, requests reconcile, which classifies the pair as pending and never re-points; the manual `adaptorredeem` RPC rebuilds the leg-A transaction from the bogus pointer and is rejected. At T1 Alice refunds A. Bob loses leg B. The symmetric spoof (`chain:"a"` sent to Alice) wedges Alice's T1 auto-refund. `nonces` and `partial_sigs` are likewise ungated (`engine.rs:2537-2551`); the nonce store makes that harmless today (a re-sign under a different message is refused, verified by test), but the same gate should cover them.

Fix: accept `funding_ready` only for the sender's own leg (initiator sends "a", participant sends "b"), only in `Accepted`, only while the pointer is unset. Make pointers immutable once a nonce session exists. In reconcile, re-point a leg whose recorded outpoint is gone by re-locating the derived script.

### N2. High — a replayed `init` with a reused anchor silently resets a live participant record

`engine.rs:1673-1800` (`accept`), `1946-2115` (`adaptor_accept`), `11071-11077` (relay init path), `store.rs:547`, `1002` (`ON CONFLICT DO UPDATE`). **Reproduced** (`handshake_probe.rs`): a `Signed` v2 record with leg-B pointer and assembled signatures became `Accepted` with all of them `None`; a v1 `FundedB` record with a signed refund became `Accepted` with the refund gone.

Neither accept path checks whether `swap_id` already exists. `init_matches_offer` does not check it either. The store upserts.

Attack: the maker relists an offer; Bob takes it; the maker's client answers with an `init` reusing the first swap's `T` (or `H`), so `swap_id` matches the live record. Bob's record is wiped while his real leg B stays locked. The C8 reaper aborts the now-empty record after 15 minutes and tombstones it (`engine.rs:3145-3160`). Alice holds σ_B and `t`, claims B whenever she likes, and refunds A after T1. Bob's engine no longer watches anything.

Fix: refuse an `init` whose `swap_id` exists in either table. State in both specs that anchors MUST be unique per swap and receivers MUST reject a reused one.

### N3. High — a malformed gift wrap stops the daemon permanently (remote, unauthenticated)

`pact-proto/src/seal.rs:295-298`; `pact-nostr/src/lib.rs:160`; `engine.rs:10373`; `pactd/src/main.rs:207, 226, 245, 2357, 2499`. **Reproduced** at the library layer (`wire_probe.rs`: decoded nonce lengths 0, 1, 11, 13 and 24 bytes all panic; only 12 takes the error path).

The prior review recorded the nonce-length panic as a parser defect. The new finding is its full reach. `unwrap_giftwrap` checks only the ephemeral nostr signature, which anyone can produce, and returns the content verbatim; the daemon stores it in `nostr_inbox` unvalidated. On the next tick `sync_board` opens it inside the closure that holds the registry mutex. A panic is not an `Err`, so it unwinds and poisons the `std::sync::Mutex`. Every later `registry.lock().expect("registry mutex poisoned")` panics: the scheduler task dies silently, every RPC returns "task panicked", timelocks are no longer watched. There is no `catch_unwind` anywhere in `pactd` or the engine. Any relay user who knows a victim's npub can do this.

The same poison path is reachable from a lying chain view: `v2_adopt_final` (`engine.rs:7485-7511`) adopts a spend transaction served by a view, and `adaptor_keep_moving`, `adaptor_bump_refund` and `adaptor_cpfp_bump` index `output[0]` on it (`engine.rs:3660`, `3789`, `4025-4026`, `4069`). A deserialisable zero-output transaction spending our outpoint panics the same way.

Fix: validate exact field sizes before the AEAD call and return a structured error. Replace every poison `expect` with `unwrap_or_else(PoisonError::into_inner)` or a `parking_lot` mutex. Wrap each `tick_one` and `adaptor_tick_one` in `catch_unwind` so one record cannot take the pass down. Validate adopted transactions (at least one output, output 0 pays a wallet address) before adoption.

### N4. High — the participant gives up its claim after the secret is already public, on a clock a single view controls

`chain.rs:2084-2093` (`tip_median_time` = maximum over all responders); `engine.rs:510-516` (`deadline_clock` = `max(local, chain_mtp)`); claim arms `engine.rs:8324-8331` (v1), `3311-3317` and `3573-3580` (v2). Source trace.

The "most advanced clock refuses earliest" rule is safe for reveal decisions, where refusing means both sides refund. It is unsafe for claim-after-reveal decisions, where refusing means the counterparty keeps both legs. Once `s` or `t` is on chain, attempting the claim costs nothing and the counterparty's refund is only valid after T1 by consensus. Two consequences:

- Honest case: past `T1 − 1h` the participant stops trying although it could still win the race until MTP reaches T1.
- Adversarial case: the initiator operates or intercepts one of the participant's Electrum servers and reports MTP ≥ `T1 − 1h` while revealing. The participant idles every tick; at real T1 the initiator refunds A.

The manual `redeem` RPC is not gated on this clock, but nothing tells the user to call it.

Fix: never refuse a claim-after-reveal on a clock; use the clock only to escalate fees. Aggregate deadline clocks by median or minimum over quorum. Reject any view whose MTP exceeds `local_now + 2h`, which consensus makes impossible.

### N5. High — v1 `fund()` can broadcast a second funding for the same leg

`engine.rs:4356-4454` (`fund`), `4789-4842` (`locate_funding`), `chain.rs:888-923` (Core `find_funding` via `scantxoutset`, confirmed outputs only). Source trace; the code comment at `engine.rs:4351-4354` acknowledges the window.

After `wallet_send_confirmed` (4410) the record is written at 4454. In between, `find_vout` (4434), two `tip_height` calls (4446, 4452) and the `put` itself can fail (RPC timeout, node restart, `SQLITE_BUSY`, process death). The record then stays `Accepted` or `FundedA` and the retry arms (`8408-8434`, `8536-8556`, `8509-8523`) call `fund()` again 30 seconds later, long before the transaction is mined. On a Core-only backend `locate_funding` finds nothing because `scantxoutset` sees confirmed outputs only. The history classification bails only on `Spent` or `Vanished` (4372-4384); `Funded` or `None` falls through to a second send. Two HTLC outputs pay the same script; the refund covers one; the counterparty claims both with `s`. Loss is one full leg. BTCX is the Core-backed coin, so the initiator on BTCX is the common case.

Fix: treat `LegClass::Funded` as "adopt the pointer, do not send". Before sending, ask the wallet for an own unconfirmed transaction paying `(script, amount)`; our funding is always a wallet transaction, so this works on every backend. Persist the txid before resolving the vout.

### N6. High — a lost `RedeemedB` write leaves the initiator's reveal un-nursed

`engine.rs:4546-4581` (`redeem`: broadcast at 4555, `put` at 4674), `8139-8156` (`FundedB` arm), `7574` (`reconcile_driven_v1`). Source trace.

If the `put` fails or the process dies after the redeem is broadcast, the record stays `FundedB`. On every later tick the `FundedB` arm sees the leg-B output gone and only emits a reorg alert and a reconcile request. Reconcile classifies `(Spent{Redeem, 0 confs}, Funded)` and returns pending without adopting the spend. Nothing fee-bumps the redeem. Fees rise, the transaction sits, Bob refunds B at T2, reads `s` from the mempool and redeems A before T1. This is the exact §7.4 hazard the `RedeemedB` nurse exists to prevent.

Fix: in the `FundedB` missing-output branch (and the participant twin at 8341), call `find_spend_tx`; if `settlement_spend_is_ours` with `is_redeem = true`, persist `final_*` and the settlement state so the nurse takes over.

### N7. High — v2 fundings are not confirmed-only on Core; leg A is not confirmed-only on any backend

`chain.rs:1264` (`fundrawtransaction` with `lockUnspents`, `fee_rate`, `replaceable:false` and no `minconf`); `engine.rs:2646` (v2 leg A via plain `wallet_send`; Core `sendtoaddress` with no `minconf` and `replaceable=true`, `chain.rs:1118-1160`; nodeless `confirmed_only=false`). **Reproduced** on a private regtest Core v31: with 48.99 BTC of unconfirmed change in the wallet, the exact `chain.rs:1264` call selected that unconfirmed output as an input; the same call with `"minconf":1` returned `-4 Insufficient funds`, so the existing queued-funding classifier would work unchanged.

The 2026-08-09 post-mortem and the PR #232 commit message state that v2's funding builder got the confirmed-only rule. Only the bdk path did. For v2 the consequence is worse than the post-mortem's wedge: the leg-B funding txid is committed into the MuSig2 pre-signatures, so an RBF of an unconfirmed parent orphans a funding that cannot be re-signed without a fresh session.

Fix: add `"minconf":1` at `chain.rs:1264` (Core ≥ 25). Route v2 leg A through `wallet_send_confirmed` with the same `FundingQueued` handling. Send leg A non-replaceable (see N10).

### N8. High — the v2 fallback sweep destination has no spend path

`engine.rs:1836-1841`, `2017-2022` (`unwrap_or_default` on `wallet_new_address`), `adaptor_redeem_dest` `137-141`, `2199-2203`. Source trace.

If `wallet_new_address()` fails at init or accept (Core briefly unreachable, `verify_chain` hiccup, locked nodeless seed), the sweep address is empty and the co-signed redeem pays P2TR of the claimer's swap key. The seed can derive that key, but no code path spends, sweeps or even displays such an output, and CPFP is impossible. Proceeds are stuck in the hot seed with no UI. The spec permits the fallback; the implementation lacks its spend side.

Fix: refuse to init or accept without a sweep address, or implement a sweeper for the deterministic destination.

### N9. Medium — the funding fee rate comes from a single view and is capped only at 10,000 sat/vB on nodeless coins

`chain.rs:2104-2127` (`fee_rate_for*` = maximum over any responder); electrum-btcx `backend.rs:29`, `:1008` (`SANITY_MAX_SAT_PER_VB = 10,000`); `engine.rs:2646-2650`, `4410-4414` (`SendFee::Target` passed straight through); wallet-btcx `lib.rs:431-443` (no absolute cap). Source trace.

`FeeBumpPolicy::max_feerate_sat_vb` (500) and the percentage cap govern bumps and claims only, not the initial funding rate. On a nodeless coin one buggy or malicious view makes the autopilot fund at up to 10,000 sat/vB, roughly 1.5 to 2.5 million sat per funding, with no user interaction. Core is bounded by its own `-maxtxfee`.

Fix: clamp funding and send rates to `max_feerate_sat_vb` at the call site; aggregate estimates by median over quorum; add an amount-relative cap like claims have.

### N10. Medium — an external RBF of a v2 leg-A funding is never healed

`engine.rs:2646` (leg A sent RBF-signalled), `3183-3200` (rediscovery only when the pointer is `None`), `7690-7693` (reconcile marks reconciled without adopting). Source trace.

The engine refuses to bump a swap funding, but the node does not: Core's `bumpfee`, the Core GUI or another wallet on the same node can replace leg A after nonces are exchanged. Then σ_A and σ_B commit to a dead outpoint (expected, the swap cannot complete), but Alice's record keeps the dead txid and her T1 refund wedges exactly as in N1.

Fix: send leg A non-replaceable (Core `replaceable=false`, bdk `ENABLE_LOCKTIME_NO_RBF`), and add the re-point arm from N1.

### N11. Medium — Electrum `ssl://` accepts any certificate; `tcp://` is plaintext

electrum-btcx `backend.rs:240-285`, `333-361` (`AcceptAnyServerCert`, SNI only, no pinning); `docs/handbook-pact/chapters/ch06-configuring-coins.md:68`, `118-124`. Source trace.

An on-path attacker controls every configured view at once, so `integrity_quorum = 2`, the byte-agreement check in `get_txout`, and the single-lying-view arguments in N4 and N9 all reduce to one attacker. The code comment says this is deliberate; the handbook promises protection against "a single lying server" without the network-attacker caveat.

Fix: trust-on-first-use fingerprint pinning per server, stored in the coin config, as Electrum desktop does. Warn on `tcp://` for mainnet.

### N12. Medium — the participant's refund needs two live views for a read a liar cannot exploit

`chain.rs:1832-1842` (`integrity_quorum`), `1881-1890` (`tip_median_time_min`), `engine.rs:8566`. Source trace.

With exactly two Electrum servers configured and one in failure backoff, `try_refund_due` errors every tick ("tip mtp: 1 of 2 answered, 2 needed") and the refund is never broadcast. A wrong MTP can only produce a harmless non-final rejection of our own refund. Requiring quorum here trades liveness for a safety property that does not exist.

Fix: for refund readiness accept one responder once the wall clock is past T by a margin, or attempt the broadcast and treat non-final as retry later.

### N13. Medium — Corkboard `relay_post` is unauthenticated and nothing is ever deleted

`corkboard/src/main.rs:205-229`. Source trace.

The relay write endpoint checks only a 64 KB size cap and a 32-byte recipient; there is no signature. There is no `DELETE FROM relay` or `DELETE FROM offers` anywhere in the server. Any client can fill a public board's disk with rows addressed to any pubkey. The prior review recorded the missing lifecycle; the missing authentication is new.

Fix: require a signed envelope on `relay_post`, add per-identity and global storage budgets, and prune expired offers and delivered mail.

### N14. Medium — no single-instance guard; a second Satchel adopts the first's daemon and the first's exit kills it

`satchel/src/main.rs:1602-1610`. Source trace.

On launch, if the port answers with our cookie and network, the instance adopts the daemon as external. Two Satchel processes then share one `pactd`. When the spawning instance closes with nothing active it runs `stop_managed` and terminates the daemon under the second instance, which shows "disconnected" and cannot relaunch. No `tauri-plugin-single-instance` is present.

Fix: add the single-instance plugin and focus the existing window.

### N15. Medium — cookie, seed, state and wallet databases are created with default permissions on Linux and macOS

`pactd/src/main.rs:2131` (`fs::write` for `.cookie`), `2228` (`create_dir_all`); seedstore `lib.rs:402-413` (`File::create`); `store.rs:310`; wallet-btcx `lib.rs:230`. No `set_permissions` or `mode(0o600)` exists anywhere in the tree. Source trace.

Under a 0755 home directory any local user can read the RPC cookie and call `sendtoaddress`, `importseed` or `stop`; on Linux the obfuscated seed is plaintext-equivalent. Windows is protected by the profile ACL.

Fix: `OpenOptions::mode(0o600)` for the four files and `DirBuilder::mode(0o700)` for the data directory on Unix, as Bitcoin Core does for its cookie.

### N16. Medium — `open_external` on Windows is a `cmd.exe` command-injection sink

`satchel/src/main.rs:1275-1289`. **Reproduced** in the scratchpad with `echo` standing in for `start`: a URL containing `"&echo INJECTED>marker&rem "` executed the injected command.

Rust quotes arguments for `CommandLineToArgvW`, but `cmd.exe` does not honour that escaping: an embedded quote closes the string and `&`, `|`, `>` become operators; `%VAR%` expands inside quotes. The scheme check does not prevent this. The sole caller today is the update dialog passing GitHub's `html_url` over verified TLS, so exploitation needs a compromised API response. The next caller that passes a link from an offer or contact note makes it directly attacker-reachable.

Fix: never route through `cmd`. Use `ShellExecuteW`, `rundll32 url.dll,FileProtocolHandler`, or `tauri-plugin-opener`; parse with `url::Url` and allow only `http`/`https` with a host.

### N17. Medium — the custom fee-rate field parses dot-only; comma-locale users get 10 to 100 times the fee

`satchel/ui/src/screens/WalletActions.tsx:245-247`; `pactd/src/main.rs:1955-1962`. Source trace.

The field strips everything but digits and `.`. On de-DE, `1,5` becomes 15 sat/vB and `1,08` becomes 108 sat/vB. Unlike the amount field it does not use the locale-aware sanitizer. There is no upper bound on the custom rate in the UI or in `sendtoaddress`; the confirm dialog's fee line is the only guard.

Fix: share the locale-aware sanitizer; add a hard ceiling and an extra confirm when the fee exceeds a percentage of the amount.

### N18. Medium — managed shutdown truncates the daemon's de-list and outbox flush

`satchel/src/main.rs:582-598` (`stop_managed`: send `stop`, sleep 800 ms, `kill`); `pactd/src/main.rs:2490-2524` (shutdown de-list plus outbox flush, time-boxed at 10 s). Source trace.

The 800 ms kill routinely cuts off the courtesy de-list and the final NIP-09 deletion, so "withdraw and exit" often never reaches the relay. Not a fund-safety issue, but it silently defeats a documented promise.

Fix: wait on the child for up to about 12 s after `stop`, kill only on timeout.

### N19. Medium — offer revocation can be turned into censorship of any offer

`pact-nostr/src/lib.rs:199-219`; `pactd/src/nostr_service.rs:128-147`; `store.rs:369`. Source trace.

The prior review recorded that the revocation cache discards author identity. The new point is the concrete attack. The event conversion does check that the deleter owns the coordinate. Downstream, everything keys on `swap_id` alone, the offer cache has no author column, and the `d` tag is the public `swap_id`. An attacker publishes their own compatible offer with `d = S` (replacing the victim's cached row), then deletes their own coordinate. Every node that sees the deletion tombstones `S` permanently; re-publishing cannot resurrect it.

Fix: carry `(author, identifier)` through the cache key, tombstone key and removal, including locally published offers.

### N20. Medium — dependency and release hygiene

Details and raw output in the evidence directory (`cargo-audit-*.txt`, `npm-audit-*.txt`).

- `nostr 0.44.3` and `nostr-relay-pool 0.44.1` in `pactd`, `pact-nostr`, `crier` and `relay-prober` are yanked, `nostr-relay-pool` is unmaintained, and eleven advisories are open, including RUSTSEC-2026-0224 (verification-cache poisoning lets forged events skip signature checks) and RUSTSEC-2026-0231 (AUTH challenge flood, unbounded memory). Integrity is mitigated because `pact-nostr` verifies every consumed event itself and the inner envelope carries its own signature; the NIP-44/NIP-04 panic advisories are unreachable because sealing is done in `pact-proto`. Residual exposure is denial of service and no future fixes. `cargo update -p nostr -p nostr-relay-pool -p nostr-sdk` is semver-compatible and clears it.
- `security.yml` runs weekly with `continue-on-error: true` on every step and audits only three of the seven lockfiles, so the advisories above went unnoticed since 2026-08-03. No `dependabot.yml`.
- Release and CI actions are pinned to mutable tags (`tauri-apps/tauri-action@v0` among them) in a workflow with `contents: write`; `ci.yml` has no `permissions` block; no cargo invocation uses `--locked`; the crier Dockerfile builds on a floating base image.
- Release artifacts carry no checksums or signatures. The updater is check-only, which limits impact.
- Tauri `csp` is `null`. No injection sink exists today, so this is defence in depth.
- `pact/Cargo.toml` release profile has no `overflow-checks`; no `rust-toolchain.toml`; no `#![forbid(unsafe_code)]` although the tree has zero `unsafe`.
- Core RPC credentials are passed on the `pactd` command line (`satchel/src/main.rs:508-518`) and stored in plaintext in `satchel.json`; on Linux `/proc/<pid>/cmdline` is world-readable. Cookie-file auth, the default, avoids this.

### Low

- **Consumed MuSig2 secret nonces stay on disk in plaintext** (`store.rs:1127-1129`). A stored secnonce plus its partial signature yields the per-leg swap key. Zero the column on consume.
- **Gift-wrap `created_at` is not randomised** (`pact-nostr/src/lib.rs:147-154`). NIP-59 recommends a randomised past timestamp; without it a relay can reconstruct message cadence between two mailboxes.
- **Sealed blobs are unpadded** (`pact-proto/src/seal.rs:259-267`). Ciphertext length reveals the message class to a relay or board.
- **Offers with `created: 0` never leave the Nostr cache** (`pact-nostr/src/lib.rs:61-73`, `store.rs:833`). They are un-takeable but persist forever as stale cards; a cheap cache-pollution vector alongside the unbounded cache.
- **Corkboard `list_offers` filters pair and network after `LIMIT 500`** (`corkboard/src/main.rs:158-179`). Honest offers can be buried behind 500 newer rows.
- **bdk user sends may spend foreign unconfirmed inputs** (wallet-btcx `lib.rs:545-611`). A send can chain on a third party's replaceable payment and be orphaned. Fundings are unaffected.
- **Nodeless steady-state sync has no gap lookahead** (sync.rs:70-97, 176-182). A sweep address minted by a sibling machine at a higher index is invisible to this wallet.
- **Electrum `get_raw_tx` does not verify the returned txid** (electrum-btcx `backend.rs:596-603`); `find_vout` trusts it. `MultiBackend::fetch_tx` verifies, this source should too.
- **Core `tx_confirmations` returns 0 for a mined transaction unknown to the local wallet** (`chain.rs:1037-1050`). A settlement adopted from a sibling machine loops bump, `-25`, reconcile, adopt every tick and never latches on a Core-only backend.
- **NSIS hooks embed `$INSTDIR` in single-quoted PowerShell** (`satchel/installer-hooks.nsh:45, 61, 73`). A username containing `'` breaks the script silently: the daemon is not stopped before an upgrade overwrites it.
- **`Params::u32` truncates** (`pactd/src/main.rs:485`): `t1 = 4294967306` becomes 10. Authenticated callers only.
- **No zeroization anywhere**: `WalletSeed`, mnemonic strings and the passphrase drop unwiped. Consistent with the hot-wallet threat model, worth stating in the handbook.
- **The nonce derivation omits the secret key** and a cloned live datadir on two machines reuses a secnonce (see P5).

## Performance findings

### F1. High (availability) — Core spend detection enumerates the whole mempool one RPC at a time, every tick, under the registry lock

`chain.rs:964-1020` (loop at 992-1005); callers `engine.rs:8870` (`adopt_settlement_winner`, every tick in all four v1 settlement states while our spend is unconfirmed), `8322`, `4722`, and the v2 driver arms `3573`, `2752-2790`, `2934`. Source trace with cost derived from code.

Once `gettxout` says "spent", the code calls `getrawmempool` and then `getrawtransaction` for every mempool txid, sequentially, each on a new TCP connection (`Connection: close`). On BTC mainnet with 50,000 to 150,000 transactions that is one to five minutes per call. `MultiBackend::find_spend_tx` joins all fan-out threads, so a fast Electrum view does not short-circuit the Core scan. When the spend is already mined the mempool walk still runs before the block scan, and the v2 driver arms then scan every block from the funding height with `getblock` verbosity 2 and no watermark (the reconcile path has one at `engine.rs:6797-6845`; the drivers bypass it). All of this holds the registry mutex, so every RPC and the UI stall for the duration, every 30 seconds, for the whole unconfirmed window of a settlement. BTCX's small mempool hid this; a dead pointer from N1 or N10 makes it permanent.

Fix: `gettxspendingprevout` (Core ≥ 24) replaces the walk with one call; fall back to `getrawmempool true` and its `depends`. Route driver scans through the watermark. Skip the Core scan when a script-index view has already answered.

### F2. Medium — `scantxoutset` runs every tick while a counterparty leg has no pointer

`engine.rs:4815-4820`, `2607`, `2677`, `3192`, `3202`, `6051`, `6474`; `chain.rs:888-923`. Source trace.

A lost `funded` message means a full UTXO-set scan (tens of seconds on BTC, holds `cs_main`) every 30 seconds. Core rejects a concurrent scan with `-8`, which the parallel fan-out over two coins or two swaps will hit.

Fix: bound to every N ticks, or use the wallet's own transaction list when the funding is ours, and serialise scans.

### F3. Medium — the seed is scrypt-decrypted or fetched from the OS keystore on every backend construction

`engine.rs:938` (`backend()` builds a fresh `MultiBackend`), `1069` (`nodeless_backend` calls `store.seed()`), `store.rs:498`; further per-tick `seed()` callers at `engine.rs:10370` (`sync_board`) and `8742` (`maybe_bump`). Source trace.

For a passphrase-protected seed each call is one scrypt with N = 2^15 (about 32 MB, tens of milliseconds); for a keyring seed it is one Credential Manager round trip. Tick arms call `backend()` two to four times per swap. All of it runs under the registry lock.

Fix: cache the derived `PactSeed` and `WalletSeed` in the engine and drop them on lock.

### F4. Low — `backend()` re-runs `verify_chain` on every call

`engine.rs:938-963`; Core `chain.rs:835-845` (`getblockhash 0` each time). A driven v2 swap constructs six to eight backends per tick plus a `getaddressinfo` from `v2_owns_redeem_payout` (`engine.rs:9483-9497`), roughly 12 to 16 RPCs per live v2 swap per tick on Core. Cache the verdict per URL and memoise the payout probe per record.

### F5. Low — full record lists are loaded twice per tick

`engine.rs:4885`, `4905`, `5110`, `5116`. `store.list()` and `list_adaptor()` each run twice per tick, deserialising every record's JSON including both transaction hexes. The swaps table is `(swap_id, record TEXT)` with no state column or index. At 10,000 records that is about 70 to 150 ms per tick, the residual the prior review measured with no coin backends attached.

### F6. Low — UI polls overlap with no in-flight guard, and `listcoins` probes nodes under the lock

`satchel/ui/src/AppContext.tsx:427, 435, 443`; `CorkboardScreen.tsx:232`; `pactd/src/main.rs:1197-1215`. When the registry lock is held, each 4-second tick queues four more blocking tasks per poll; on release they all fire at once. `listcoins` every 10 seconds runs a network probe per configured coin inside the lock. Skip a tick while the previous one is pending; move the probe to a cached engine-side value. The `AppCtx` value object is rebuilt on every provider render without `useMemo`, so every consumer re-renders on every poll.

### F7. Info — measured costs

| Item | Value |
| --- | --- |
| v1 initiator in `FundedA`, chain reads per tick | ~11 (about 22 Electrum calls with one view) |
| v1 settlement unconfirmed, chain reads per tick | ~8 plus one per mempool transaction on Core |
| Settled or aborted record, chain reads per tick | 0 (latch confirmed correct) |
| `open_envelope` on a blob not for us | ~41 µs; `messages::verify` ~61 µs (bench) |
| UI main bundle | 2,009 kB, 647 kB gzip, one chunk, 26 locales statically imported |

## What is sound

Listed because a review that only reports defects misrepresents the codebase. Each item was read and, where marked, executed.

- **v1 script and transactions**: byte-exact template, `OP_SIZE 32` preimage guard, time-based CLTV only, `nSequence 0xFFFFFFFD`, BIP143 `SIGHASH_ALL`, P2WSH only; `extract_preimage` verifies the hash rather than trusting position. Vectors pinned and passing.
- **v2 cryptography**: BIP327 key aggregation (rogue-key safe) in fixed order; BIP341 tweak pinned equal to rust-bitcoin's output key by test; aggregate adaptor signature verified against the tweaked key before `Signed`; leaf version `0xc0`, `SIGHASH_DEFAULT`, script-path sighash includes the leaf hash; sweep, amount, fee and outpoint bound in the sighash; `t` extraction validated by `reveal_secret`.
- **Nonce discipline**: CSPRNG nonces, written ahead of the public nonce, reused on resume, and a differing partial under a consumed nonce is refused. *Verified by test*: after a pointer change post-`Signed`, both `adaptor_sign` and `adaptor_assemble` refuse. Snapshots never carry nonce sessions. No engine-originated path re-runs a signing round.
- **Funding order and depth gates**: the participant builds leg B without broadcasting and commits only when `Signed` and leg A is `n_a` deep with exact script and value; `MultiBackend::get_txout` takes the minimum confirmations over agreeing views, halts on script or value disagreement, and needs the integrity quorum; every pre-money gate in v1 and v2 goes through it. Timelock constraints are enforced by the receiver against its own clock; the genesis check runs before any funding.
- **Refund readiness** uses the laggiest MTP; a refund refuses to compete with a visible counterparty spend; v1 fundings are confirmed-only on both backends; fee-bump nurses are bounded, Rule-4 aware, and repoint atomically; the settled latch (this branch) is correct for what it targets.
- **Wire format**: canonical JSON is deterministic and float-free; the signature covers version, type, swap id, sender and body; the inner envelope signature, not the transport, authenticates the sender, and the counterparty identity is pinned per swap; slip decoding is fail-closed; cursor poisoning is clamped.
- **Daemon and desktop**: loopback-only bind; 32-byte random cookie with constant-time compare; only `/health` is unauthenticated; JSON-only `POST /` with a 2 MiB limit and no CORS, so browsers cannot reach it; no seed-export RPC; `dumpswap` scrubs secrets; the webview never sees the cookie; Tauri capabilities are minimal; no raw-HTML sinks; offers carry no free text; the send path re-parses the address with network-typed params; the mnemonic is never copied to the clipboard; one daemon per datadir via an OS lock; the installer stops daemons by PID under the install directory only.
- **Keys and storage**: all-hardened derivation, disjoint branches, per-swap uniqueness, BIP84/86 vectors asserted; seed at rest under scrypt plus ChaCha20-Poly1305 with atomic writes and a never-overwrite guard; the handbooks are honest about keyring and Linux guarantees; no secret is logged anywhere.
- **Supply chain**: zero `unsafe`; clippy clean under `-D warnings`; security-critical crate versions current; single rustls story; all lockfiles tracked and identical to the shipped v1.0.0 tag; the btcx git dependencies pinned to one immutable revision; harness downloads sha256-pinned; secrets sweep clean.

## Prior findings

All twelve numbered findings and the four performance sections of the 2026-09-09 review are present at this revision. Three gained materially new evidence, noted here in one line each and not repeated above:

- Its "additional issue" (v2 Core funding not confirmed-only) is now reproduced and wider than recorded; see N7.
- Its #5 (nonce panic) is now shown to be reachable by any relay user and to brick the daemon; see N3.
- Its #7 (reconstruction trust) has a concrete consequence: a single lying view fabricating 64-byte-witness spends of both legs makes `reconcile_driven_v2` mark our still-locked leg `Completed`, tombstone it and stop refunding (`reconstruct.rs:120-126`, `chain.rs:2026-2051`, `engine.rs:7637-7745`).

## Suggested repair order

1. N1 and N2: state, role and uniqueness gates on `funding_ready`, `nonces`, `partial_sigs` and `init`. Small changes, close both theft paths.
2. N3: fixed-size validation in the sealer, poison-tolerant locking, `catch_unwind` per record. Closes the remote brick.
3. N4, N5, N6, N7, N8: claim-after-reveal never refuses on a clock; adopt-before-fund and adopt-after-redeem in the crash windows; `minconf` on every funding path; refuse an empty sweep.
4. F1 and F2: `gettxspendingprevout`, watermarked scans, bounded UTXO scans. These also remove the field stall class this branch was cut for.
5. N9 through N19 and the dependency bump in N20, then the Low items and the spec corrections P1 through P5.

## Coverage and limits

Source review covered every crate in the tree plus the pinned btcx checkout. Executed: 45 crypto and vector unit tests, `cargo audit` on all seven lockfiles, `npm audit`, `cargo clippy -D warnings` on the pact workspace, the UI production build, two in-process handshake reproductions, a library-level sealer probe, two regtest probes against a private Core v31 node, and a Windows command-injection probe. Not executed: the full end-to-end harness (unchanged since the prior run), a live lying-Electrum harness for N4 and N9, an on-chain reproduction of the N1 and N2 theft sequences beyond the record-corruption step, Core-to-nodeless payout migration, and dependency internals beyond advisory review. No production or mainnet funds were used.
