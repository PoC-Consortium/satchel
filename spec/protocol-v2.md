# Pact Swap Protocol v2 — Taproot/MuSig2 adaptor swaps Bitcoin PoCX ↔ BTC

`pact-htlc-v2`. Companion to [`protocol.md`](protocol.md) (v1) — this
document specifies only what v2 changes. Everything not restated here
(roles, transport, persistence philosophy, swap index `i`, the identity
key) is inherited from v1 verbatim.

Implementation plan and rationale: the [Pact handbook](../docs/handbook-pact/) (the v2 protocol chapters).

## 1. What changes from v1

| | v1 (`pact-htlc-v1`) | v2 (`pact-htlc-v2`) |
|---|---|---|
| Output | P2WSH HTLC script | **P2TR** (Taproot): key-path = 2-of-2 MuSig2; script-path = CLTV refund leaf |
| Cooperative spend | reveal SHA256 preimage `s` | **MuSig2 key-path** signature, looks like an ordinary payment |
| Secret | preimage `s`, `H = SHA256(s)` shared on both legs | **adaptor scalar `t`**, point `T = t·G`; nothing shared on-chain links the legs |
| Leg link | same `H` on both chains | the **same adaptor point `T`** encrypts both redeem signatures; broadcasting one reveals `t` |
| Refund | timeout branch of the HTLC script | **single-key** CLTV tapleaf, key-path-free |
| Swap-key crypto | ECDSA | **BIP340 Schnorr / MuSig2** |

The structural timelock rule (`T2 < T1`), the funder/claimer roles, and the
safety model (timelocks protect funds; chain backends are untrusted) are
unchanged.

## 2. Chain requirements

Both chains MUST support Taproot (BIP340 Schnorr + BIP341 + BIP342). Bitcoin
PoCX has Taproot ALWAYS_ACTIVE from genesis; Bitcoin since block 709 632. A pair
where either leg lacks Taproot cannot run v2 (the engine's capability
resolver, `registry::protocols_for`, already encodes this).

## 3. Keys and secrets

The Pact seed and BIP32 purpose (`7228'`) are unchanged. Path re-use, new
key *types*:

| Material | Path | v2 key type |
|---|---|---|
| Identity key | `m/7228'/0'/0'` | BIP340 x-only (unchanged) |
| Swap key, chain *c*, swap index *i* | `m/7228'/1'/coin(c)'/i'` | secp256k1, used as a **BIP340 x-only key and MuSig2 signer** (was ECDSA) |
| Refund key, chain *c*, swap index *i* | `m/7228'/3'/coin(c)'/i'` | secp256k1 x-only — signs the single-key CLTV refund tapleaf |
| Adaptor-secret source, swap index *i* | `m/7228'/2'/i'` | feeds §3.1 (was the preimage source) |

`coin(c)`, swap-index rules, and "one swap key per chain per swap" are
inherited from v1 §4.1–4.2. The refund key is a **new, separate** key
(`branch 3'`) so the refund tapleaf is single-sig and independent of the
MuSig2 aggregate.

### 3.1 Deterministic adaptor secret

Alice (only) derives, for her swap index `i`:

```
k  = private key at m/7228'/2'/i'                     (32 bytes)
t  = TaggedHash("pact/adaptor/secret/v2", k)  mod n   (a valid secp256k1 scalar, ≠ 0)
T  = t·G                                              (the adaptor point)
```

`t` is the swap secret. Like v1's `s` it is seed-derived, so losing the
state DB never loses it (§ recovery). `t` MUST NOT be disclosed; it becomes
public only when Alice's leg-B redeem is broadcast (§6), at which point Bob
extracts it. `T` is shared in `init` and is not secret.

### 3.2 Nonces — use-once, never seed-derived (normative)

MuSig2 secret nonces MUST be generated from a CSPRNG, bound per BIP327 to
the signer's key, the message, and the aggregated key (the `musig2`
`SecNonce` construction). They MUST NOT be derived deterministically from
the Pact seed, and a given secret nonce MUST be used to produce **at most
one** partial signature. Implementations MUST persist the secret nonce to
durable storage *before* releasing the corresponding public nonce, and on
restart MUST resume from persisted state rather than regenerating
(`store` `nonce_state`: `none → committed → revealed → consumed`). Reusing a
nonce across two messages leaks the signing key. See the Pact handbook chapter
"v2 Taproot/MuSig2 Adaptor Swaps" (nonce-safety).

### 3.3 Swap identifier

`swap_id = hex( TaggedHash("pact/swapid/v2", T) [0..8) )` — same shape as v1
but over the adaptor point `T` rather than `H`.

## 4. The Taproot output

Each leg is a single P2TR output `Q = P + TapTweak(P, m) · G` where:

- **Internal key `P`** = MuSig2 aggregate of the two parties' swap keys for
  that chain (`KeyAggContext::new([maker_swap, taker_swap])`, x-only). This
  is the cooperative **key-path** spend.
- **Merkle root `m`** = a single tapleaf (BIP342, leaf version 0xc0), the
  **refund script**:

  ```
  <T_leg> OP_CHECKLOCKTIMEVERIFY OP_DROP <funder_refund_pubkey> OP_CHECKSIG
  ```

  i.e. the funder, and only the funder, can sweep the output back to
  themselves after the absolute time-lock `T_leg` (Unix time; height
  locktimes are forbidden as in v1 §5). The tweak commits to this leaf so
  the refund path exists without ever appearing on the happy path.

Key ordering for `KeyAggContext` is fixed: `[funder_swap_pubkey,
counterparty_swap_pubkey]` for that leg. Both parties MUST aggregate in this
order or the aggregate key differs.

## 5. Transactions

- **Funding** (one per leg, built by that leg's funder's core wallet): a
  normal send creating the P2TR output of §4. Estimated like v1 §6.1.
- **Cooperative redeem** (key-path): 1-in/1-out, spends the funding output
  by the **MuSig2 aggregate Schnorr signature**, sweeping to the claimer's
  **fresh core-wallet sweep address** (the `*_sweep_*` address exchanged in
  `init` / `accept`, §7; empty falls back to a deterministic key-derived
  address). nSequence signals RBF; nLockTime 0. The aggregate signature is
  produced as an *adaptor* signature under `T` (§6) and only becomes valid
  once adapted by `t`.
- **Refund** (script-path): 1-in/1-out, spends via the §4 tapleaf with the
  funder's refund-key signature + the control block; nLockTime = `T_leg`,
  valid only once MTP ≥ `T_leg`. This is an ordinary single-key Schnorr
  signature — **no MuSig2, no interactive nonce** — so the unattended
  auto-refund never touches the reuse-prone primitive.

BIP341 sighashes (key-path = `SIGHASH_DEFAULT`; script-path includes the
leaf hash) via `SighashCache::taproot_*`.

## 6. The adaptor mechanism (leg link)

Roles as v1: **Alice funds leg A** (refund time `T1`), **Bob funds leg B**
(refund time `T2`), `T2 < T1`. Alice holds `t`.

Both redeem signatures are MuSig2 *adaptor* signatures under the **same**
adaptor point `T`:

1. The parties run a MuSig2 adaptor session over **leg B's redeem** (Alice
   claims B). Result: an `AdaptorSignature` `σ_B` valid only when adapted by
   `t`. Alice knows `t`, so only Alice can complete and broadcast it.
2. The parties run a second MuSig2 adaptor session over **leg A's redeem**
   (Bob claims A), under the same `T`. Result: `σ_A`. Bob does **not** know
   `t` yet.
3. Alice broadcasts the adapted `σ_B` to claim leg B. The on-chain signature
   `σ_B^final` now publicly reveals `t = reveal_secret(σ_B, σ_B^final)`.
4. Bob extracts `t`, adapts `σ_A`, and claims leg A before `T1`.

If Alice never claims B, both refund after their respective timelocks. The
`T2 < T1` gap guarantees Bob can still claim A after Alice reveals `t` by
claiming B (he has until `T1`, and Alice cannot refund A before `T1`).

## 7. Message handshake

Transport, encoding, and the signed envelope are inherited from v1 §8.1–8.2
(end-to-end-encrypted, identity-key-signed JSON envelopes). Protocol string
in `init` is `pact-htlc-v2`; a party receiving an unknown protocol string
MUST `abort` (v1 §12).

Because the redeem transactions spend not-yet-broadcast funding outputs,
funding txids are exchanged **before** funding is broadcast (xmr-btc-swap
pattern), so both redeems can be pre-signed. Message sequence:

1. **`init`** (A→B): `{protocol, swap_id, coin_a, coin_b, amount_a,
   amount_b, t1, t2, alice_swap_pubkey_a, alice_swap_pubkey_b,
   alice_refund_pubkey_a, adaptor_point T, alice_sweep_b}`, where
   `alice_sweep_b` is Alice's fresh core-wallet address on chain B (where she
   redeems leg B). Empty selects the deterministic key-derived fallback.
2. **`accept`** (B→A): Bob's `{bob_swap_pubkey_a, bob_swap_pubkey_b,
   bob_refund_pubkey_b, bob_sweep_a}` — enough for both parties to derive both
   P2TR addresses; `bob_sweep_a` is Bob's fresh core-wallet address on chain A
   (where he redeems leg A), same empty-fallback rule.
3. **`funding_ready`** (each→other): the funder announces its funding
   **txid + vout** (built, not yet broadcast) so the redeem txs are
   determined.
4. **`nonces`** (each→other): public nonces for both MuSig2 adaptor sessions.
5. **`partial_sigs`** (each→other): partial adaptor signatures for both
   sessions. Each party verifies the aggregate `AdaptorSignature` against
   `T` (`musig2::adaptor::verify_single`) before proceeding.
6. **`funded`** (funder→counterparty): funding broadcast + confirmed (as v1
   §8.5). Bob funds B only after Alice's A is seen and the adaptor sigs
   verify.
7. **`redeemed`** (A→B, courtesy): Alice claimed B; `t` is now on-chain. Fully
   optional on both ends, as in v1 §8.6 — the reference implementation neither
   sends nor consumes it; Bob extracts `t` from the on-chain leg-B spend, which
   is authoritative. It only speeds a status update, never safety or completion.
8. **`abort`** (either): as v1.

A party MUST NOT broadcast its funding until it holds a verified
`AdaptorSignature` for the redeem of the leg it is *claiming* (so it can
always make progress) and the §4 refund path is constructed (so it can
always recover).

## 8. Timelocks, fees, adversarial model

Inherited from v1 §7, §6.4, §10 unchanged, with one substitution: "reveal
the preimage" becomes "broadcast the adapted redeem, revealing `t`". The
action-deadline arithmetic (`T2 < T1`, MTP lag, redeem-before-counterparty-
refund) is identical because the timelock structure is identical.

**Confirmation depth (§7.3).** `N_a`/`N_b` are per-leg local safety policy
(not consensus, not exchanged): a party derives each from its own per-coin
setting (else the network default). The initiator MUST NOT reveal `t` until
the counterparty's funded leg is `N_b` deep, so a reorg cannot undo the funding
under the reveal.

**Fee-bumping (§7.4).** v1's "MUST fee-bump aggressively" applies, but the two
v2 spend types diverge — the one real deviation from "inherited unchanged":

- the single-key **CLTV refund** is fully RBF-bumpable: it is re-signable with a
  deterministic nonce and spends with an RBF-signalling sequence, so the
  scheduler escalates its fee until it confirms (as in v1).
- the cooperative **MuSig2 key-path redeem** is **not** RBF-bumpable: its fee is
  committed in the pre-signed adaptor signature's sighash, so changing it would
  require a fresh interactive signing round. An implementation MUST instead pick
  a sufficiently generous redeem fee at signing time, rely on a wide redeem
  window, and rebroadcast the byte-identical tx while it is unconfirmed.
  Bumpable cooperative redeems (a pre-signed fee ladder, or a CPFP anchor in the
  redeem template) are a `pact-htlc-v3` consideration since they change the
  signed transaction set.

## 9. Recovery

Seed + swap index re-derive every long-term key (swap, refund) and `t`
(Alice). This is always enough to take the **refund** path via the §4
tapleaf. Completing an in-flight **cooperative** redeem additionally needs
the persisted MuSig2 session state (nonces are not seed-derived, §3.2); if
that state is lost mid-session, the swap falls back to the timelock refund —
never to nonce reuse.

## 10. Versioning

`pact-htlc-v2` covers §3–§8 above. Any incompatible change bumps to
`pact-htlc-v3`. v1 and v2 are distinct protocol strings negotiated in
`init`; an implementation MAY support both and selects per the capability
resolver (`registry::select_protocol`).

## 11. Test vectors

`spec/vectors/htlc_v2.json`, regenerated by
`cargo run -p libswap --example gen-vectors-v2` and pinned by
`pact/libswap/tests/vectors_v2.rs` (mirrors the v1 vector discipline, §13).
Vectors fix: the derived `t`/`T`, both swap + refund pubkeys, the aggregated
internal key `P`, the P2TR addresses, and the refund tapleaf script — the
deterministic, transcript-independent values.
