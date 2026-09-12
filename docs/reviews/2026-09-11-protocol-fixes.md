# Protocol remediation handoff

**Historical first-batch record.** The five-scenario verification below was insufficient: the independent merge-gate sweep found 20 failures. See [the corrective follow-up](2026-09-11-protocol-fixes-followup.md) for the revised code and full-sweep result.

Date: 2026-09-11. Base: `f670cd9cd3548294c744abd0a3211572fc392784`. Changes are uncommitted for independent review.

References: [original report](2026-09-10-protocol-to-implementation.md), [severity reassessment](2026-09-11-protocol-triage.md). This handoff records implemented changes and remaining acceptance criteria; it does not replace the independent review or certify release readiness.

## Implemented changes

| Findings | Change | Review focus |
| --- | --- | --- |
| N1, N2 | V2 funding messages enforce sender leg, phase, immutable outpoints and idempotent exact duplicates. Nonce/partial updates reject conflicting or out-of-order messages. Initial creation uses one atomic SQLite transaction across both protocol namespaces and persistent used-ID markers. | Replay and late-message regression; no reopening consumed signing sessions; signed outpoints stay immutable during recovery. |
| N3 | Removed unchecked settlement `output[0]` accesses in the engine. Existing nonce-length and witness-authentication defenses remain. | Original nonce entry point was already fixed at base. Current malformed adopted-transaction exploit was not independently reproduced. |
| N4, N6 | Both protocols persist claim bytes and state before broadcast and retry pending claims. Public-secret participant claims no longer stop at T1. V1 also authenticates and adopts a prior claim whose old state write was lost. Failed rebroadcast falls through to reconciliation/fee nursing; successful initial broadcast clears its retry marker immediately. | Faults at every write boundary; custody gates; first secret reveal still has safety gates. |
| N5 | V1 records a durable send attempt before wallet send, then records its txid immediately. Retry searches wallet history and chain evidence and refuses a second send when the first outcome is ambiguous. V2 leg A persists a built transaction before broadcast and resends those same bytes. Abort/timeout paths respect pending funding. | Ambiguous v1 send may require operator recovery; absence of evidence never authorizes another payment. |
| N7, N10 | V2 leg A uses a confirmed-input, non-RBF builder. Refund recovery can use an exact-value replacement funding output without changing the signed session's outpoint. | External replacement recovery, both roles and chains; full-RBF nodes need not honor signaling. Confirmed-only gaps were already addressed at base. |
| N8 | V2 refuses funding/broadcast without both sweep addresses and local ownership of its payout. | Offline initialization still permits placeholders; the guard is at commitment. Recovery of older funded fallback payouts is not added. |
| N9, P2 | Funding resolves an explicit rate bounded by fee policy; BDK wallet boundaries cap at 500 sat/vB. Funding rejects inadequate non-regtest settlement reserves and disproportionate estimated funding fees. | Funding transaction size estimate uses 200 vB, not exact selected-input cost. Reserve policy cannot guarantee future congestion economics. |
| P1 | Participant rejects a leg-A presigned fee below its local policy. Core can sign a CPFP child with explicit parent prevout data and submit the parent+child package when standalone claim relay fails. | Package-capable Core only; below-relay-floor rescue and a competing refund still need dedicated E2E coverage. Existing CPFP scenarios exercise accepted parents. |
| N11 | Vendored the pinned electrum-btcx crate: TLS handshake signatures are verified, persistent TOFU certificate pins reject changed identities, and raw transaction IDs are checked against requests. | First connection remains a trust decision. `tcp://` is still plaintext. See [vendor patch notes](../../vendor/electrum-btcx/PATCHES.md). No live hostile TLS handshake test yet. |
| N12 | Refund clocks accept one responding chain view; first-reveal quorum is retained. | Consensus CLTV remains authoritative; dishonest low time can delay a refund. |
| N13 | Corkboard relay writes require signed, fresh envelopes; retention and per-recipient/global quotas bound retained relay data. Offer quotas and expiry pruning added. Duplicate writes retain their ID even at quota. Offer filtering precedes LIMIT. | Quotas bound retained rows, not Sybil abuse, request CPU, WAL traffic, or all historical database allocation. |
| N14, N18 | Desktop single-instance guard; managed shutdown waits up to 12 seconds before kill. | Validate native behavior on all three platforms. Single-instance policy applies across networks. |
| N15 | Unix state roots repaired to 0700 and existing immediate regular files to 0600; private creation for cookie/config files. | Unix test exists but was not executed on Windows. Windows inherits directory ACLs. |
| N16, N17 | Windows URL opening avoids `cmd.exe` and validates HTTP(S). Custom fee input uses locale-aware amount parsing; server rejects invalid/out-of-range explicit fees. | Native URL launch and localized UI smoke test. |
| N19 | Nostr offer insertion, deletion, and active-cache selection scope addressable identity by author and d-tag. | New cross-author regression caught and fixed the active-selection query as well as insertion. Historical unscoped tombstone migration is not added. |
| N20 | All seven lockfiles audit without known vulnerability findings; compatible dependency updates, Discord gateway uses platform TLS to avoid its obsolete rustls dependency. CI audits every lockfile, enforces npm audit, pins action SHAs, uses locked Cargo builds, and adds Dependabot. Desktop passes Core arguments through a private file rather than process argv. Pact release overflow checks enabled. | Informational/unmaintained dependency warnings remain. Crier Linux builds need system OpenSSL (CI already installs it). Artifact signatures/attestation, CSP and broader key zeroization remain separate work. |
| P3, P4, P5 | Specs align timeout/public-secret claim behavior, derivation scope paths and recovery metadata, adaptor exchange ordering, fee scalar validation and CPFP capability. | Seed alone does not recover nonzero machine scope; preserve machine metadata/snapshots. Do not clone a live datadir to concurrent signers. |
| F1, F2 | Core uses direct mempool spender lookup; legacy scan has count/time limits. Block scans have bounded advancing cursors. UTXO scans serialize and throttle per script. RPC exchange has total read deadline/size cap and bounded socket connection attempts. | Global registry lock still covers network work; OS DNS is not bounded. This reduces stalls but does not close the cross-swap scheduler-latency release gate. |
| F3, F6 | Store caches the decrypted seed for its lifetime; UI pollers suppress overlapping calls. | Hot seed lifetime increases; cache drops with Store. Server coin health probes/global-lock scheduling still need isolation. F4/F5 optimizations remain. |

## Verification

Windows local runs (test logs beside this file are ignored by Git):

- Pact workspace tests and clippy with warnings denied.
- Satchel: 21 Rust tests, clippy with warnings denied, UI production build and ESLint.
- Corkboard: two security regressions covering tampering, stale/future messages, blob bounds, retention, recipient quota and retry identity; clippy with warnings denied.
- Vendored TLS pin persistence/change-rejection unit test.
- Private regtest `BroadcastRecoveryV1`: simulate lost funding response/state, retry without a second payment; reconstruct lost claim state; participant claims after T1.
- Private regtest `AdaptorSwap`, `AdaptorCorkboardSwap`, Bitcoin and Litecoin `AdaptorRedeemCpfp` scenarios.
- Cargo audit across pact, pact-proto, pact-nostr, corkboard, satchel, crier and tools/relay-prober. JSON results: `2026-09-11-audit-*.json`.
- Workflow YAML parses; `git diff --check` passes. Hosted workflows and signed installers were not run.

Final local results: pact workspace **212 tests passed**, desktop **21 passed**, Crier **17 passed**, Corkboard **2 passed**. Workspace, desktop, Crier and Corkboard clippy passed with warnings denied. All seven Cargo audits returned exit 0 (informational warnings remain). BroadcastRecoveryV1 passed again after rebuilding the daemon. Relay-prober checks and Crier clippy also passed. A final CPFP regression exposed a retained retry marker that delayed fee nursing; successful initial claim broadcasts now clear it immediately. After that fix, workspace tests/clippy passed again and both rebuilt CPFP scenarios passed (Bitcoin 13s, Litecoin 21s).

## Compatibility and operational details

Corkboard and clients must be upgraded together: `/v1/relay` now expects a signed `relay_post` envelope with `{to, blob, created}` body. Nostr transport still carries sealed blobs through its existing transport adapter.

V1 funding uses send-intent reconciliation rather than a universally available build-before-send wallet API. A node error after the durable marker may leave funding blocked when no transaction can be found; this is deliberate duplicate-payment prevention. Wallet history lookup is bounded to 1000 transactions. A reviewed recovery tool for these markers remains desirable.

TLS certificate rollover needs out-of-band verification before a pin is removed. Pins are shared per user/endpoint; authenticated first contact is not bootstrapped by TOFU. Existing plaintext endpoints are not silently migrated. The vendored patch must be maintained or upstreamed.

The public `ChainBackend::wallet_build_funding` signature now accepts an explicit fee; `Store::seed()` returns a cached `Arc<PactSeed>`. Downstream consumers outside this repository need adaptation.

## Remaining review / release gates

1. P1: prove rejected-parent package rescue through a sustained fee spike and refund race; define an equivalent supported path or restrict v2 exposure for nodeless/backends without package submission. The current estimate check alone is insufficient.
2. F1: measure and enforce a total scheduler budget while an unrelated refund is due. Remaining global-lock network work and OS DNS mean this is not yet established.
3. N1/N2/N5/N6: existing regression coverage establishes invariant/recovery fixes, not exhaustive power-loss/database-failure coverage or a reproduced complete theft prevented end to end.
4. N10: independently exercise refund-only recovery after external replacement on both chains. N15: run permissions/umask and existing-install migration checks on Linux/macOS. N11: test hostile TLS handshake proof, changed pin and legitimate rotation.
5. Unnumbered recovery/privacy items remain: scoped-wallet gap lookahead, Core-only foreign-settlement finality without txindex, consumed nonce erasure/WAL/backups, gift-wrap timestamp randomization, sealed-message padding, zero-created/unbounded Nostr cache, ordinary user-send unconfirmed-input policy, NSIS apostrophe handling, and comprehensive secret zeroization. `Params::u32` truncation and raw Electrum txid checking were fixed here.

This is a substantial repair batch, not closure of every original report item. In particular, the package-rescue and scheduler-liveness acceptance gates remain open for release purposes.
