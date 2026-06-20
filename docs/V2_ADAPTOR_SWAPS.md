# v2 — Taproot / MuSig2 adaptor swaps (`pact-htlc-v2`)

The v2 swap protocol. Each chain leg is a **2-of-2 MuSig2 Taproot output**
instead of v1's CLTV hash-timelock P2WSH script, and the swap secret is
carried by a **Schnorr adaptor signature** rather than a hash preimage. v2 is
implemented in `libswap` and wired through `pactd` and Satchel; it is
**selectable on regtest and testnet and refused on mainnet** until an external
crypto audit signs off (see [The mainnet gate](#the-mainnet-gate)).

- Normative protocol: [../spec/protocol-v2.md](../spec/protocol-v2.md).
- Roadmap context: [TRADING_ROADMAP.md](TRADING_ROADMAP.md).
- Architecture / recovery model: [ARCHITECTURE.md](ARCHITECTURE.md).

## What v2 is

Atomic swaps where each leg is a single **P2TR** output:

- the **key-path** is the 2-of-2 MuSig2 aggregate of both parties' swap keys —
  the cooperative redeem, spent by one aggregate BIP340 Schnorr signature;
- the **script-path** is a single tapleaf, a **single-key CLTV refund** that
  only the leg's funder can take after its absolute time-lock.

The cross-leg link is a **Schnorr adaptor signature**: both redeem signatures
are pre-signed ("encrypted") under the same adaptor point `T = t·G`.
Broadcasting one completed redeem publishes the full signature, from which the
counterparty extracts the scalar `t` and uses it to complete and broadcast the
other leg's redeem. There is no hashlock, no preimage, and no revealed script
on the cooperative path.

v2 uses Taproot key-spend + MuSig2 + Schnorr adaptor signatures because the
cooperative spend is **indistinguishable from an ordinary single-key Taproot
payment** — no swap fingerprint, no shared on-chain hash linking the two legs,
smaller transactions, no script disclosure. That privacy is the entire point
of v2; Schnorr's linearity is what makes the adaptor construction clean.

Both Bitcoin PoCX (Taproot ALWAYS_ACTIVE from genesis) and BTC support Taproot, so the
launch pair runs v2 natively. A pair where either leg lacks Taproot cannot run
v2 and falls back to v1 — the capability resolver
(`registry::protocols_for`) encodes this.

## The mainnet gate

Two flags in `libswap/src/registry.rs` govern availability:

- **`ADAPTOR_BUILT`** (`true`) — the v2 engine path (`musig`, `taproot`,
  `adaptor_swap`, `adaptor_engine`) is built, so `select_protocol` may return
  `Protocol::Adaptor` for a Taproot pair that has no classic-HTLC option.
- **`ADAPTOR_MAINNET_ENABLED`** (`false`) — v2 is **not** permitted on mainnet
  until the external crypto audit passes.

`adaptor_allowed(network)` returns
`ADAPTOR_BUILT && (network != Mainnet || ADAPTOR_MAINNET_ENABLED)`. The engine
calls this through `ensure_adaptor_supported` (both legs must have Taproot and
both networks must be allowed) when initiating or accepting a v2 swap: a
mainnet leg is refused with a clear "gated on mainnet pending the security
audit" error, while regtest and testnet run freely. `board_offer_protocol`
applies the same rule, so a PoCX↔BTC offer advertises `pact-htlc-v2` on
regtest/testnet and `pact-htlc-v1` on mainnet. Flipping
`ADAPTOR_MAINNET_ENABLED` is a deliberate, auditable one-line change — the
mainnet boundary is explicit, not implicit.

> **TODO:** external crypto audit (MuSig2 nonce handling, adaptor soundness,
> x-only parity) — required before `ADAPTOR_MAINNET_ENABLED` is flipped.

## Keys and secrets

The Pact seed and BIP32 purpose (`7228'`) are unchanged. v2 reuses the swap
path with new key *types* and adds a separate refund key (spec v2 §3):

| Material | Path | v2 key type |
|---|---|---|
| Swap key (chain *c*, index *i*) | `m/7228'/1'/coin(c)'/i'` | secp256k1, used as a BIP340 x-only key and MuSig2 signer |
| Refund key (chain *c*, index *i*) | `m/7228'/3'/coin(c)'/i'` | secp256k1 x-only — sole signer of the single-key CLTV refund tapleaf |
| Adaptor-secret source (index *i*) | `m/7228'/2'/i'` | feeds the deterministic adaptor secret `t` |

The refund key is a **new, separate** branch (`3'`) so the refund tapleaf is
single-sig and fully independent of the MuSig2 aggregate. Alice (initiator)
derives the swap secret `t` deterministically from `m/7228'/2'/i'` and shares
the point `T = t·G` in `init`; `t` itself becomes public only when she
broadcasts her leg-B redeem. The swap identifier is
`swap_id = TaggedHash("pact/swapid/v2", T)[0..8)` (`keys::swap_id_v2`).

## The Taproot output and transactions

`libswap/src/taproot.rs` builds the per-leg output and spends. A `TaprootLeg`
holds the MuSig2 aggregate internal key, the funder's x-only refund key, and
the absolute Unix-time locktime `T` (height locktimes are rejected):

- **Output** — `spend_info` / `script_pubkey` / `address`: a P2TR output whose
  internal key is the aggregate and whose single tapleaf is the refund script
  `<T> OP_CLTV OP_DROP <refund_xonly> OP_CHECKSIG`.
- **Key-path redeem** — `build_keypath_redeem` returns the unsigned 1-in/1-out
  redeem tx and its BIP341 key-path sighash (the message the MuSig2 session
  signs); `attach_keypath_signature` installs the single 64-byte aggregate
  signature as the witness. nLockTime 0.
- **Script-path refund** — `build_refund_tx` builds and **fully signs** the
  refund tx with a plain single-key Schnorr signature over the CLTV tapleaf
  (witness = `[sig, script, control_block]`), nLockTime = `T`. No MuSig2, no
  interactive nonce.

## The MuSig2 / adaptor crypto

`libswap/src/musig.rs` integrates the **`musig2` crate** (pure-Rust BIP327
MuSig2 with a built-in `adaptor` module). The `musig2` crate carries its own
`secp256k1` separate from this crate's `bitcoin`/`secp256k1`, so the two type
universes never mix; `musig.rs` crosses the boundary by bytes only — 33-byte
compressed pubkey, 32-byte x-only, 32-byte scalar (`pubkey_to_point`,
`seckey_to_scalar`, `point_to_xonly`) — and `aggregate_2of2` produces the
2-of-2 aggregate as a Taproot x-only internal key.

`libswap/src/adaptor_swap.rs` orchestrates the engine-independent crypto and
transaction flow:

- `AdaptorSwapParams` holds everything both parties learn after `accept`
  (amounts, `t1`/`t2`, both parties' swap pubkeys, both refund x-only keys,
  the adaptor point `T`) and reconstructs both legs deterministically.
- Key order for each leg's `KeyAggContext` is fixed `[funder, counterparty]`.
- `tweaked_ctx_for_leg` applies the BIP341 taproot tweak over the refund-leaf
  merkle root, so the aggregate the MuSig2 session signs for equals the leg's
  on-chain P2TR **output** key (x-only parity included). A test pins this
  against rust-bitcoin's computed output key.
- `lifted_to_bitcoin` converts a finalized `musig2` signature into a
  rust-bitcoin Schnorr signature.

`libswap/src/adaptor_engine.rs` is the reusable daemon driver, wired to the
`ChainBackend` trait, the use-once nonce `Store`, and the `musig2` functional
adaptor API: `session_nonce` (load-or-create the use-once secret nonce),
`session_partial` (partial adaptor sign + mark consumed), `aggregate_adaptor`
(combine the two partials into the leg's `AdaptorSignature`), and
`reveal_from_onchain` (recover `t` from an adaptor signature plus the broadcast
64-byte signature). The full flow is proven end to end in-process against a
mock backend: fund both legs, persist + exchange nonces, partial-sign,
aggregate + verify, Alice broadcasts the adapted leg-B redeem, Bob reads the
on-chain witness and recovers `t`, Bob redeems leg A, plus the single-key
refund path.

## Nonce-safety design

The catastrophic v2 risk is **MuSig2 nonce reuse**: signing two different
messages with the same secret nonce `r` lets anyone solve
`x = (s1−s2)/(e1−e2)` and recover the signing key — and in a swap the
counterparty already sees your nonces and partial signatures. Because `pactd`
is a hot, auto-signing daemon, restart/resume/concurrency are realistic ways
to reach reuse. Nonce-use-once is therefore a **structural property of the
engine**, not a function of how the operator runs it. Four decisions:

1. **Split the paths — the dangerous primitive is off the unattended path.**
   Interactive MuSig2 runs **only** on the cooperative key-path redeem (both
   parties online). The auto-refund is a **single-key script-path spend**
   (`build_refund_tx`) signed with the deterministic per-swap refund key — no
   MuSig2, no interactive nonce. A single-key BIP340 signature uses a
   deterministic nonce, so re-signing the same refund tx reproduces the
   identical signature — safe by construction. The unattended daemon never
   touches the reuse-prone primitive.

2. **Nonces are fresh CSPRNG (BIP327), never seed-derived.** Each MuSig2 secret
   nonce is built with fresh aux randomness bound to the signer's key, the
   message, and the aggregated key (`musig2` `SecNonce` construction).
   Long-term keys stay deterministic from the seed; only nonces are random. A
   re-derivable nonce would let a restore-from-seed reproduce and reuse one.

3. **Use-once via write-ahead persistence.** Each signing session carries a
   monotonic `nonce_state` in SQLite — `none → committed → revealed →
   consumed` (`store.rs` `nonce_sessions` table). The secret nonce is written
   to durable storage **before** its public nonce leaves the process
   (`nonce_commit`, write-ahead), so there is no window where a public nonce
   was sent but its secret nonce was not persisted. On resume the engine
   reloads the persisted secret nonce and reuses it (`session_nonce`); a
   `consumed` session re-sends the stored partial signature rather than signing
   again. `nonce_commit` refuses to overwrite an existing `(swap, leg)` slot —
   that overwrite is exactly the key-leaking reuse — so replay becomes *safe*
   instead of forbidden.

4. **One session per `(swap, leg, spend-path)`.** A DB primary-key uniqueness
   constraint on `(swap_id, leg)` plus an in-process lock prevents two
   concurrent sessions ever producing two nonces/signatures for the same
   key+message, on top of the `musig2` BIP327 two-nonce defence.

### Recovery contract

Seed + chain scan always recovers the **long-term keys** (swap and refund) and,
for Alice, the adaptor secret `t` — always enough to **refund via the timelock**
(the script-path leaf needs only the deterministic refund key). An in-flight
**cooperative** session additionally depends on the persisted SQLite nonce
state; if that state is lost mid-session the swap falls back to the timelock
refund, **never to nonce reuse**. This is a deliberate, bounded degradation:
v2 trades "complete an in-flight cooperative redeem from seed alone" (which
would require unsafe re-derivable nonces) for "lose at most time, never the
key." ARCHITECTURE.md carries the same contract.

## The pactd handshake

Transport, encoding, and the signed/encrypted envelope are inherited from v1.
The protocol string negotiated in `init` is `pact-htlc-v2`; a party that does
not recognise it aborts. Because the redeem transactions spend not-yet-broadcast
funding outputs, funding txid+vout are exchanged **before** funding is
broadcast, so both redeems can be pre-signed.

State persists as `store::AdaptorSwapRecord` (a JSON blob in the
`adaptor_swaps` table); the lifecycle is `AdaptorState`
(`Created → Accepted → NoncesExchanged → Signed → FundedA/FundedB →
RedeemedB → Completed`, plus `Refunded`/`Aborted`). The `pactd` RPC surface
(`pactd/src/main.rs`) dispatches to the engine:

| RPC | Engine | Role |
|---|---|---|
| `adaptorinit` | `adaptor_init` | Alice builds `InitV2` (terms, her keys, `T`); records `Created` |
| `adaptoraccept` | `adaptor_accept` | Bob verifies `InitV2`, records, replies `AcceptV2` (his keys) |
| `adaptorrecv` | `recv_adaptor` | ingest a counterparty envelope (pins identity, absorbs handshake material) |
| `adaptorfundingready` | `adaptor_funding_ready` | announce a built (not yet broadcast) funding txid+vout |
| `adaptornonces` | `adaptor_nonces` | emit this party's public nonces for both redeem sessions |
| `adaptorsign` | `adaptor_sign` | emit partial adaptor signatures for both sessions |
| `adaptorassemble` | `adaptor_assemble` | aggregate + verify both `AdaptorSignature`s against `T` → `Signed` |
| `adaptorfund` | `adaptor_fund` | broadcast this party's funding tx |
| `adaptorredeem` | `adaptor_redeem` | adapt + broadcast the key-path redeem (Alice reveals `t`; Bob extracts it) |
| `adaptorrefund` | `adaptor_refund` | broadcast the single-key script-path refund after the timelock |
| `listadaptorswaps` | `store.list_adaptor` | enumerate v2 swaps (separate from v1 `listswaps`) |

Both sides rebuild identical `AdaptorSwapParams` from the `InitV2`/`AcceptV2`
bodies (`messages.rs`), so every leg, sighash, and aggregate is reconstructed
deterministically on each end.

The scheduler step `adaptor_tick_one` (run from the daemon's `tick`, as in v1)
auto-redeems when safe and auto-refunds after the timelock, and keeps an
unconfirmed spend moving until it confirms. The board autopilot drives the
whole handshake through the blind relay for Taproot offers off-mainnet
(`board_offer_protocol`, `handle_relay_envelope` / `drive_adaptor_relay`).

### Confirmation-depth gate (reorg safety)

Each leg carries an `n_a`/`n_b` confirmation depth (`AdaptorSwapRecord`),
resolved at `adaptor_init`/`adaptor_accept` from the **per-coin** depth via
`Engine::confirmations_for` — the operator's setting if present, else the
network/spacing `default_confirmations` heuristic (regtest 1; fast chain 10;
slow 3). Depth is **local safety policy, not consensus**: each side sets its
own from its own config, so the two records need not match and nothing is
exchanged on the wire. It is configurable per coin in Satchel's Coins setup
page → `satchel.json` → pactd `--coin-confs <coin_id>=<N>` → the engine.

`adaptor_tick_one` uses it as the **reveal gate**: the initiator does not
publish `t` (redeem leg B) until Bob's leg-B funding is `n_b` confirmations
deep, so a shallow funding cannot reorg out from under the reveal (spec v2 §8 /
v1 §9.5). The participant claims leg A as soon as `t` is on chain with no depth
gate — once `t` is public it stays valid even if that spend later reorgs, so
racing to redeem A is always correct. The initiator's `RedeemedB → Completed`
transition is likewise gated on the redeem reaching `n_b` depth.

### Keep-the-spend-moving (fee-bump / rebroadcast)

Spec v2 §8 inherits v1 §7.4's "MUST fee-bump aggressively." The two v2 spend
types differ in what is possible, and `adaptor_keep_moving` reflects that:

- the single-key **CLTV refund** is RBF-bumped (`adaptor_bump_refund`): rebuilt
  at ~50% higher fee and re-signed with the deterministic refund key — safe by
  construction (no MuSig2, deterministic nonce; the unattended-safe path), with
  a plain rebroadcast fallback once a higher fee would dust the output. The
  refund already spends with a BIP125-signalling sequence (`0xFFFFFFFD`), so the
  replacement is mempool-accepted. This mirrors v1's `maybe_bump`.
- the cooperative **MuSig2 key-path redeem** is **rebroadcast only**: its fee is
  sealed into the pre-signed adaptor signature's sighash, so it cannot be
  re-fee'd without a fresh interactive signing round (new nonces, counterparty
  online, touching the reuse-prone primitive). Rebroadcast still recovers from a
  dropped mempool entry. This matches the reference adaptor-swap engine
  (COMIT `xmr-btc-swap`), which likewise pre-signs at a fixed fee, never RBFs a
  cooperative spend, and leans on generous timelocks.
  - **Interim mitigation (M2, shipped).** The redeem feerate is no longer a
    hardcoded 2 sat/vB: the initiator fixes `redeem_feerate_a`/`redeem_feerate_b`
    in the signed `init` from her live estimators (6-block estimate ×3,
    over-provisioned because it is unbumpable), per chain; the participant
    bounds-checks them (≤ 500 sat/vB) and stores the same values, so both build
    byte-identical redeem txs. Regtest keeps the legacy 2 sat/vB. This removes
    the "guaranteed-stuck below the mempool floor" failure mode but does **not**
    make the redeem bumpable after signing — if the fee market spikes between
    `init` and the redeem it can still underpay. The designs below are the full
    fix and remain the reason v2 stays mainnet-gated.

### Making the cooperative redeem bumpable

**STATUS: design #1 (CPFP) is IMPLEMENTED (v2+, commit 340657d).** The
redeem-nurse arm of `adaptor_keep_moving` now re-anchors the parent and bumps it
with a self-funded child spending the redeem's own wallet-owned sweep output
(`adaptor_cpfp_bump` / `cpfp_child_fee`; the wallet signs the child via
`ChainBackend::wallet_sign_send`). Because M2 keeps the parent relayable, it sits
in the mempool and a **plain CPFP child** suffices — `submitpackage` is NOT used,
and is only needed for the sub-floor extreme (a future enhancement). Proven by
`harness/test_adaptor_swap.py::test_adaptor_redeem_cpfp`. The original design
analysis follows.

Three candidate designs to lift the "cooperative redeem can't be fee-bumped"
limitation, in order of preference:

**1. CPFP via the redeem's own output + package relay — preferred (IMPLEMENTED).** The redeem
already sweeps to an output the **claimer** alone controls, so the claimer can
bump it unilaterally at broadcast time: build a high-fee child spending that
output and submit `[low-fee redeem, high-fee child]` together via
`submitpackage`, so miners accept the pair at the package feerate even when the
parent alone is below the mempool minimum. Properties:

- **No protocol or template change.** The signed redeem tx is byte-identical
  (both sides still build it deterministically; the adaptor signature still
  validates). The CPFP child is built unilaterally, purely in the engine — so
  this is implementable on the *current* v2 wire format, not a `pact-htlc-v3`
  negotiation. Engine-side, it slots into `adaptor_keep_moving` (the redeem arm)
  for a backend that advertises `submitpackage`.
- **Preserves v2 privacy.** No anchor output and no `nVersion=3` — the redeem
  stays an ordinary single-key Taproot spend (v2's whole point). The child is a
  normal spend of the claimer's funds.
- **Pinning is a non-issue here.** The redeem's output is single-sig to the
  claimer, so no third party can attach a descendant to it — the usual pinning
  vectors need a shared/anyone-can-spend output, which this design lacks.
- **Availability.** Bitcoin PoCX nodes are **Bitcoin Core v30+**, so TRUC/v3, P2A, and
  `submitpackage` are native on the PoCX leg. The BTC leg uses a recent Core we
  run; the bumper acts unilaterally, so the *counterparty's* node version is
  irrelevant to bumping our own redeem.
- **Residual dependency (the one real caveat).** P2P **package-relay
  propagation to miners on the BTC network** is still maturing, so a deeply
  underpriced redeem reaching a miner is not 100% guaranteed unless peered with
  package-relay-capable nodes/pools (or submitted directly). On the PoCX leg
  (v30, smaller network, rarely congested) this is not a concern.
- Pairs naturally with the **fresh-sweep-address** work (now shipped, commit
  e4f9a08): the cooperative redeem sweeps to a fresh claimer-controlled
  core-wallet address (exchanged as `alice_sweep_b` / `bob_sweep_a` in
  `init` / `accept`, spec-v2 §5/§7), so the CPFP child attaches cleanly and the
  redeem doesn't link to a static placeholder key.

**2. Pre-signed fee ladder — fallback.** During the single signing round,
pre-sign N adaptor signatures over N redeem variants at escalating fixed fees;
escalate by broadcasting the next variant (no new interaction). Preserves
privacy and needs zero node features, but only **bounds and discretizes** the
margin — the ceiling is fixed at signing, so a spike above the top rung
reproduces the same failure. Useful as a fallback where BTC-side package
propagation is unreliable. Requires agreement at signing time (a `pact-htlc-v3`
change to the signed tx set).

**3. CPFP via a dedicated TRUC/ephemeral anchor — rejected for the private
redeem.** The Lightning end-state (a `nVersion=3` redeem with a P2A/ephemeral
anchor) gives truly unbounded bumping, but reintroduces an on-chain
**fingerprint** — the v3 version byte plus a recognisable anchor output — which
defeats v2's "indistinguishable from an ordinary payment" goal. Only worth
considering if the privacy property is being relaxed anyway.
>
> **TODO:** v2 reorg handling is still thinner than v1's beyond the reveal gate
> above — it lacks v1's pre-funding re-verify guard and the `reorg-alert` when a
> *verified* funding output later vanishes (v2 records funding pre-broadcast and
> has no post-verification funded state to anchor that alert to).

## Satchel

The Satchel UI folds `listadaptorswaps` into the Swaps ledger alongside v1
swaps (`adaptorToSwap`), surfacing the v2-only handshake states, and labels v2
pairs/offers as **"Private (Taproot)"**. The front-end holds no swap logic —
it is a thin client of pactd's RPC.

## Verification status

- **In-process / offline proofs are green:** MuSig2 2-of-2 key aggregation and
  key-path signing, the taproot tweak matching rust-bitcoin's output key,
  adaptor sign/adapt/reveal across both legs, the single-key script-path
  refund, the use-once nonce store (commit/reveal/consume, restart-resume,
  overwrite-refusal), and the full `adaptor_engine` flow over a mock
  `ChainBackend`. Test vectors are pinned in `spec/vectors/htlc_v2.json` via
  `pact/libswap/tests/vectors_v2.rs`.
- **Live regtest swap (happy path) — confirmed.** A full v2 swap has been run
  end to end through Satchel against real PoCX + BTC regtest nodes, exercising
  the chain-touching glue (`adaptor_fund`, `adaptor_redeem`, `tick`) and the
  MuSig2/adaptor crypto on-chain.
- **Automated harness / refund path** — `pact/harness/test_adaptor_swap.py`
  covers the happy path, the refund path, the **refund fee-bump** path
  (`test_adaptor_refund_feebump`: a stuck refund is RBF-escalated then
  confirms), the **reveal depth gate** (`test_adaptor_depth_gate`: with
  `--coin-confs btc=2` the reveal is withheld at 1 conf and fires at 2), and
  board-driven autopilot — all through pactd's RPC against real regtest nodes.
  In-process unit tests (`engine.rs`) cover `Engine::confirmations_for`
  (per-coin override vs default, the ≥1 floor), `n_a`/`n_b` resolution at
  init/accept, and the `AdaptorSwapRecord` migration defaults.

  > **TODO:** run the automated harness end to end (needs `POCX_BITCOIND` /
  > `BTC_BITCOIND` node binaries) as a repeatable regression — the refund,
  > fee-bump, and depth-gate paths in particular, which the manual happy-path
  > swap does not exercise. Recommended, alongside the external audit, before
  > the mainnet gate is opened.
