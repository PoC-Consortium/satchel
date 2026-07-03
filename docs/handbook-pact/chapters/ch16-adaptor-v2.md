# v2 Taproot/MuSig2 Adaptor Swaps

The v2 protocol (`pact-htlc-v2`) settles the same trade as v1 but commits to
each leg as a *Taproot* (P2TR) output instead of a hash-locked P2WSH script.
The cooperative redeem is a single 64-byte Schnorr signature produced by a
2-of-2 MuSig2 key-path spend — on chain it is indistinguishable from an
ordinary single-key payment. There is no preimage and no shared hash linking
the two legs; instead a MuSig2 *adaptor signature* under a common point `T`
binds them, and broadcasting one redeem reveals the scalar `t` that unlocks the
other. The refund path is a single-key CLTV tapleaf — no MuSig2, no interactive
nonce — so it is safe to run unattended.

Roles are unchanged from v1 (`adaptor_swap.rs:5-8`): Alice funds leg A (refund
`T1`), Bob funds leg B (refund `T2 < T1`), Alice holds the adaptor secret `t`.

Both chains MUST support Taproot. PoCX has Taproot active from genesis; Bitcoin
since block 709632.

## Keys and secrets

v2 reuses the v1 paths but adds a refund-key branch and a new adaptor secret
(`keys.rs:91-135`, spec v2 §3):

| Material | Path | v2 type & use |
|---|---|---|
| *Swap key* | `m/7228'/1'/coin(c)'/i'` | secp256k1, used as a **BIP340 x-only key and MuSig2 signer** (was ECDSA in v1) |
| *Refund key* | `m/7228'/3'/coin(c)'/i'` | secp256k1 x-only; sole signer of the single-key CLTV refund tapleaf — a **new, separate** branch |
| *Adaptor secret source* | `m/7228'/2'/i'` | feeds the adaptor tagged hash (was the v1 preimage source) |

The swap key is the *same private key* as v1, encoded x-only instead of
compressed (`keys.rs:96-101`). The refund key being a distinct branch (`3'`) is
deliberate: it keeps the refund tapleaf single-signature and independent of the
MuSig2 aggregate, so the unattended refund path never needs an interactive
ceremony.

The adaptor secret and the swap id are deterministic (`keys.rs:118-142`,
spec v2 §3.1):

```text
k  = privkey at m/7228'/2'/i'                               (32 bytes)
t  = TaggedHash("pact/adaptor/secret/v2", k)  mod n         (a valid secp256k1 scalar, ≠ 0)
T  = t·G                                                    (the adaptor point, shared in init)
swap_id = hex( TaggedHash("pact/swapid/v2", T)[0..8] )      (16 hex chars)
```

`t` is the v2 analog of v1's preimage `s`: Alice-only, seed-re-derivable, never
disclosed off-chain. The point `T` is public and is what binds the two redeem
signatures together.

## The Taproot output

Each leg is one P2TR output (`taproot.rs:43-120`, spec v2 §4). Its two spend
paths are:

- **Key path** = the 2-of-2 MuSig2 aggregate of both parties' swap keys — the
  cooperative redeem.
- **Script path** = a single tapleaf holding a single-key CLTV refund script.

The internal key `P` is the MuSig2 aggregate, with a **fixed key order
`[funder, counterparty]`** (`adaptor_swap.rs:86-90`). The single refund tapleaf
is (`taproot.rs:72-81`):

```text
<T> OP_CLTV OP_DROP <funder_refund_xonly> OP_CHECKSIG
```

encoded as a TapScript leaf (leaf version `0xc0`). The output key is `P`
tweaked by the leaf's merkle root (`taproot.rs:83-98`), and the address is
bech32m.

The leg assignments mirror v1 (`adaptor_swap.rs:85-90`, and leg B symmetric):

| Leg | MuSig2 internal key order | refund tapleaf key | `T` |
|---|---|---|---|
| **A** (Alice funds) | `[alice_swap_a, bob_swap_a]` | `alice_refund_a` | `t1` |
| **B** (Bob funds) | `[bob_swap_b, alice_swap_b]` | `bob_refund_b` | `t2` |

> **Note** — Because the key path is a 2-of-2 MuSig2 spend, a successful v2
> redeem is on-chain indistinguishable from any other single-key Taproot
> payment. There is no hash, no script, and nothing linking leg A to leg B
> visible to a chain observer. Privacy is a side effect of the construction,
> not a bolt-on.

## Cooperative redeem (key path)

The happy-path redeem spends via the key path (`taproot.rs:154-179`). It is a
version-2, 1-in / 1-out transaction with `nLockTime = 0` (no timelock on the
happy path) and an RBF-signalling sequence. It sweeps `value − fee` to the
claimer's **fresh core-wallet sweep address** — `alice_sweep_b` /
`bob_sweep_a`, exchanged in `init`/`accept` (empty falls back to a key-derived
address). The witness is a **single 64-byte aggregate Schnorr signature**
(`SIGHASH_DEFAULT`), attached by `attach_keypath_signature`
(`taproot.rs:173-179`):

```text
witness = [ 64-byte aggregate Schnorr sig ]
```

Worst-case vsize: `KEYPATH_REDEEM_VSIZE = 111` (`taproot.rs:40`).

> **Warning** — The cooperative key-path redeem is **NOT RBF-bumpable**: its
> fee is sealed into the pre-signed adaptor sighash and cannot be re-signed
> unilaterally. It commits at the live market rate at init (`committed_mult`,
> default 1) and is dragged through by a CPFP child that spends the redeem's own
> sweep output if the market rises. See the chapter "Fees, Fee-Bumping &
> Auto-Refund".

## Refund (script path)

The refund spends via the script path (`taproot.rs:184-234`): a single-key
Schnorr signature over the CLTV tapleaf, with `nLockTime = T` (valid only once
MTP ≥ T). The witness reveals the leaf and its control block:

```text
witness = [ sig, refund_script, control_block ]
```

This path uses **no MuSig2 and no interactive nonce** — it is a plain BIP340
single-signature spend, signed deterministically from the refund key
(`taproot.rs:219-226`). That is what makes the refund **unattended-safe**: a
daemon recovering from a crash, with no live nonce state at all, can still
refund from the seed alone.

Worst-case vsize: `SCRIPTPATH_REFUND_VSIZE = 140` (`taproot.rs:41`). As in v1,
the engine refuses any spend whose output would fall below
`DUST_LIMIT_SAT = 546` (`taproot.rs:130-133`).

## The adaptor mechanism

The binding between the two legs is built before either party funds
(spec v2 §6, `adaptor_engine.rs`):

1. The parties run a MuSig2 **adaptor** session over the **leg-B** redeem
   transaction under the point `T`, producing adaptor signature `σ_B`.
2. They run a second adaptor session over the **leg-A** redeem under the
   **same** `T`, producing `σ_A`.
3. **Both adaptor signatures are verified against `T` before any funding.**
   This is the `Signed` state in the lifecycle: reaching it guarantees that the
   cooperative redeems work and that Alice's broadcast will reveal `t`.
4. Alice (who knows `t`) **adapts** `σ_B` into a valid signature and broadcasts
   the leg-B redeem. The completed on-chain 64-byte signature now encodes `t`.
5. Bob **extracts `t`** by subtracting his adaptor contribution from the
   broadcast signature, adapts `σ_A`, and claims leg A before `T1`.

The symmetry to v1 is exact: instead of a preimage `s` published in a witness,
the secret `t` is published *inside* the aggregate signature. Bob's daemon
recovers it from chain B the same way it would scan for a v1 preimage.

## Nonce safety

MuSig2 is catastrophically broken by nonce reuse — repeating a secret nonce
across two different messages leaks the private key. Pact makes reuse
structurally impossible (`spec v2 §3.2`):

- **Fresh from a CSPRNG.** Secret nonces are generated per BIP327 from the
  operating system's randomness, **never derived from the seed**. (The
  long-term keys and `t` are seed-derived; nonces are not.)
- **Write-ahead persisted.** A nonce is written to disk *before* it is used.
- **A monotonic state machine.** Each `(swap, leg)` nonce slot advances
  `none → committed → revealed → consumed` (the `nonce_sessions` store).
  Overwriting an already-used slot is **refused**, so the same nonce can never
  sign two messages.
- **Lost state falls back to refund.** If nonce state is lost (e.g. a crash
  before a session completes), the engine does **not** improvise a fresh nonce
  to finish the cooperative redeem — it lets the leg time out and takes the
  single-key timelock refund instead. Refund never needs a nonce, so this path
  is always available.

> **Warning** — The cardinal rule: a lost or ambiguous nonce state always
> resolves to the timelock refund, never to nonce reuse. Correctness is
> preserved at the cost of falling back from the cooperative path to the
> refund path.

## The recovery contract

The two roles reconstruct their long-term keys differently, mirroring how they
index them (spec v1 §4.2, inherited by v2 §3): **Alice** (initiator) always
reconstructs every long-term key and the adaptor secret `t` from the seed plus
her local swap index `i` (`keys.rs:118-128`, spec v2 §9). **Bob** (participant)
has no counter to fall back on — his swap and refund keys are instead
*anchored* to the adaptor point `T`, so the seed plus `T` alone re-derives
them, with no session state needed either. Therefore:

- Either party can **always refund** via the single-key tapleaf using only the
  seed (plus, for Bob, the anchor `T` he learned at `init`) — no live session
  state required.
- Alice can **always re-derive `t`** to complete a redeem, even with an empty
  state DB.
- A machine rebuilt from the seed alone (no local record at all) still
  reconstructs both parties' long-term keys correctly once it has the anchor —
  which is what makes the encrypted relay-snapshot rescue (chapter "Seeds,
  Wallets & Merchants") possible: the snapshot supplies the anchor and any
  session-specific data (assembled adaptor signatures) that isn't derivable,
  and the seed supplies everything else.

The only thing that cannot be reconstructed from the seed is a *consumed*
MuSig2 nonce — and that is by design, because reusing one would be the unsafe
act. Losing nonce state costs you the cooperative path, not your funds.

## Taproot tweak parity

The MuSig2 key aggregation applies the BIP341 taproot tweak via
`KeyAggContext::with_taproot_tweak`, so the signing session produces a
signature that verifies under the **tweaked output key**, with the parity
handling that BIP340 x-only keys require. The engine's tests confirm a
key-path signature attached this way verifies under the output key
(`taproot.rs:341-378`), proving the tweak plumbing the aggregate signature
plugs into is correct.

## vsize reference

| Constant | Value (vB) | Transaction |
|---|---|---|
| `KEYPATH_REDEEM_VSIZE` | 111 | cooperative key-path redeem |
| `SCRIPTPATH_REFUND_VSIZE` | 140 | single-key CLTV script-path refund |
| `CPFP_CHILD_VSIZE` | 150 | the CPFP redeem-bump child (see "Fees, Fee-Bumping & Auto-Refund") |

The key-path redeem is the smaller spend precisely because its witness is a
single signature with no script or control block to reveal.
