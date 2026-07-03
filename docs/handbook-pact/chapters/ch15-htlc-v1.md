# v1 HTLC Swaps

The v1 protocol (`pact-htlc-v1`) settles a trade with two *hash-timelock
contracts* (HTLCs), one per chain. A single SHA256 preimage `s` unlocks both:
the initiator reveals `s` to claim her leg, which publishes `s` on chain so the
participant can claim his. Each leg also has a timeout branch that refunds the
funder after an absolute Unix-time deadline. This chapter walks the protocol
end to end — key derivation, the witness script, funding, redeem, refund, and
preimage extraction — reproducing the exact byte-level constructions from
`libswap`.

Roles are fixed (see the chapter "The Swap Lifecycle"): *Alice* is the
`Initiator`, holds the preimage, locks chain A, and refunds at `T1`; *Bob* is
the `Participant`, locks chain B, and refunds at `T2 < T1`. The maker funds
first.

## Key derivation

All key material derives from one BIP39 seed via BIP32, under the Pact
*purpose* `7228'` (`7228` is "PACT" on a phone keypad, `keys.rs:20-21`). Every
level is hardened.

| Material | Path | Type & use |
|---|---|---|
| *Identity key* | `m/7228'/0'/0'` | BIP340 x-only; signs handshake envelopes. **Never** appears in an HTLC. |
| *Swap key, initiator (Alice)* | `m/7228'/1'/coin(c)'/i'` | compressed secp256k1, ECDSA; the redeem/refund key, indexed by her local counter `i`. |
| *Swap key, participant (Bob), anchored* | `m/7228'/1'/coin(c)'/a'/b'/c'/d'` | Same key type; `a,b,c,d` are the first four masked-31-bit words of `TaggedHash("pact/swap-key-anchor/v1", H)` (spec §4.2) — no counter, re-derivable from the seed plus `H` alone (which sits in both on-chain HTLC scripts). |
| *Preimage source* | `m/7228'/2'/i'` | feeds the preimage tagged hash (initiator only). |

One swap key per chain per swap either way: on the chain where a party locks
funds it is their refund key, on the chain where they claim it is their redeem
key.

> **Note** — The two roles index their keys differently because they learn the
> swap's identity at different times. Alice's counter `i` is the *root* of the
> swap's identity — `H` (and through it `swap_id`) derives from the preimage at
> index `i`, so it cannot itself be derived from the swap. Bob learns the
> public anchor `H` from the `init` message before deriving any key, so his
> keys need no counter — and two machines sharing one seed can never issue Bob
> the same key for two different swaps, since the anchor is swap-specific. This
> is also what makes Bob's participant-side rescue possible after a data-loss
> restore (see "Seeds, Wallets & Merchants"): his keys re-derive from the seed
> plus the on-chain script alone, with no local state needed.

The `coin(c)` index identifies the **asset**, not the network
(`keys.rs:23-25`):

| Asset | `coin(c)` |
|---|---|
| BTC | `0` |
| PoCX (BTCX) | `0x504F4358` (ASCII `"POCX"`) |

The preimage and the swap identifier are deterministic, so both survive loss
of the state database (`keys.rs:83-89`, `keys.rs:137-142`):

```text
s        = TaggedHash("pact/htlc/preimage/v1", privkey at m/7228'/2'/i')   (32 bytes)
H        = SHA256(s)                                                        (the hashlock)
swap_id  = hex( TaggedHash("pact/swapid/v1", H)[0..8] )                     (16 hex chars)
```

Only Alice can derive `s` (she owns the seed and the index `i`); Bob only ever
learns `H` until Alice reveals `s` on chain.

> **Note** — The swap key is ECDSA in v1. The *same* derivation path is reused
> in v2 as a BIP340 x-only MuSig2 signer — same private key, different public
> encoding. See the chapter "v2 Taproot/MuSig2 Adaptor Swaps".

## The HTLC witness script

Each leg is a P2WSH output committing to this witness script
(`htlc.rs:62-87`). The script is identical on both chains; only the keys and
the locktime `T` differ per leg.

```text
OP_IF
    OP_SIZE 32 OP_EQUALVERIFY
    OP_SHA256 <H> OP_EQUALVERIFY
    OP_DUP OP_HASH160 <hash160(redeem_pubkey)>
OP_ELSE
    <T> OP_CHECKLOCKTIMEVERIFY OP_DROP
    OP_DUP OP_HASH160 <hash160(refund_pubkey)>
OP_ENDIF
OP_EQUALVERIFY
OP_CHECKSIG
```

- The **`OP_IF` (hash) branch** is the redeem path: present a 32-byte item
  whose SHA256 equals `H`, plus a signature from `redeem_pubkey`. The
  `OP_SIZE 32 OP_EQUALVERIFY` guard forbids the empty-push trick that would
  otherwise let `OP_SHA256` run on a zero-length item.
- The **`OP_ELSE` (timeout) branch** is the refund path: after the absolute
  locktime `T` matures (enforced by `OP_CHECKLOCKTIMEVERIFY` against BIP113
  median-time-past), a signature from `refund_pubkey` spends it.

The scriptPubKey is the standard P2WSH wrapper (`htlc.rs:89-92`):

```text
OP_0 <SHA256(witness_script)>
```

### Per-leg keys and locktimes

The two legs assign the redeem/refund roles in mirror image
(`swap.rs:106-124`):

| Leg | `redeem_pubkey` | `refund_pubkey` | `T` |
|---|---|---|---|
| Chain **A** (Alice funds) | `bob_redeem_a` | `alice_refund_a` | `t1` |
| Chain **B** (Bob funds) | `alice_redeem_b` | `bob_refund_b` | `t2` |

On leg A, Bob redeems (with `s`) and Alice refunds at `T1`. On leg B, Alice
redeems (with `s`) and Bob refunds at `T2`. Both legs share the same hashlock
`H`.

> **Warning** — The locktime `T` MUST be a **Unix timestamp ≥ 500000000**;
> block-height locktimes are rejected at construction (`htlc.rs:28-29`,
> `htlc.rs:48-52`). HTLCs are time-locked, not height-locked, so a deadline
> means the same wall-clock instant on both chains regardless of their block
> cadence.

> **Warning** — A party MUST reconstruct the counterparty's witness script
> locally from the agreed `SwapParams` and **byte-compare** it before funding.
> Never trust an address or script handed to you over the wire — derive it.

## Funding

Funding is an ordinary core-wallet send to the leg's exact-amount P2WSH
address (spec §6.1). The funder's node performs coin selection, change, and fee
estimation as for any payment; Pact only supplies the destination and value.
The funder then sends a `funded(txid, vout)` message, and the receiver
**verifies the output on chain** (correct script, correct amount, sufficient
confirmations) rather than trusting the message. The engine can also discover
funding by watching the chain directly if the message is withheld.

The funding vsize used for fee previews is `FUND_TX_VSIZE = 160`
(`swap.rs:34`) — a 1-input / 2-output segwit send is a sensible mid-point; a
real wallet selecting more inputs may differ.

## Redeem

The redeem transaction spends the HTLC via the hash branch
(`swap.rs:178-205`). It is a version-2, 1-in / 1-out transaction with
`nLockTime = 0` and an RBF-signalling input sequence. The witness stack is:

```text
[ sig‖0x01, redeem_pubkey (33), s (32), 0x01, witness_script ]
```

| Item | Bytes | Purpose |
|---|---|---|
| `sig‖0x01` | DER + `SIGHASH_ALL` byte | BIP143 signature over the spend |
| `redeem_pubkey` | 33 | the compressed redeem key, hashed in the script |
| `s` | 32 | the preimage; SHA256 must equal `H` |
| `0x01` | 1 | the `OP_IF` selector — a non-empty item takes the hash branch |
| `witness_script` | var | the full script, revealed for P2WSH |

Key parameters (`swap.rs:127-205`):

- `nSequence = 0xFFFFFFFD` (`HTLC_SPEND_SEQUENCE`, `swap.rs:17`) — signals RBF
  so the redeem fee can be bumped (see the chapter "Fees, Fee-Bumping &
  Auto-Refund").
- `nLockTime = 0` — the hash branch has no timelock.
- Signature is BIP143 `SIGHASH_ALL` (`swap.rs:158-169`).
- Worst-case vsize for fee math: `REDEEM_TX_VSIZE = 155` (`swap.rs:25`).

The spend sweeps `htlc_value − fee` to the claimer's destination; the engine
refuses to build a spend whose output would fall below the dust limit
(`DUST_LIMIT_SAT = 546`, `swap.rs:19-20`, guarded at `swap.rs:138-141`).

## Refund

The refund transaction spends the HTLC via the timeout branch
(`swap.rs:210-235`). The witness stack uses an **empty** item to select the
`OP_ELSE` branch:

```text
[ sig‖0x01, refund_pubkey (33), <empty>, witness_script ]
```

| Item | Bytes | Purpose |
|---|---|---|
| `sig‖0x01` | DER + `SIGHASH_ALL` byte | signature from the refund key |
| `refund_pubkey` | 33 | the compressed refund key |
| `<empty>` | 0 | the `OP_ELSE` selector — an empty item is false, taking the timeout branch |
| `witness_script` | var | the full script |

Parameters:

- `nLockTime = T` — set to the leg's absolute locktime, so the transaction is
  only valid once the chain's median-time-past reaches `T`. Broadcasting
  earlier is rejected by the node (not fatal; the engine retries).
- `nSequence = 0xFFFFFFFD` — RBF-signalling, so the refund is also bumpable.
- Worst-case vsize: `REFUND_TX_VSIZE = 146` (`swap.rs:26`).

> **Note** — The v1 refund is **signed and persisted at funding time**
> (`rec.refund_tx_hex`). The instant a party funds a leg, the fully-signed
> refund transaction already exists on disk, ready to broadcast the moment its
> timelock matures — even if the daemon has been restarted since. v2 differs:
> it re-derives the refund from the seed on each call. See the chapter "Fees,
> Fee-Bumping & Auto-Refund".

## Preimage extraction

When Alice redeems leg B, her witness publishes `s` on chain B. Bob's daemon
recovers it by scanning the spend's witness for a 32-byte item whose SHA256
equals the known `H` (`htlc.rs:104-114`):

```rust
for item in witness_items {
    if item.len() == 32 && SHA256(item) == hash_h {
        return Some(item);   // this is s
    }
}
```

The candidate is **verified against `H`**, never taken positionally on faith —
backend data is untrusted, and a malicious node cannot feed Bob a bogus
preimage that passes the SHA256 check. Once Bob has `s`, he redeems leg A with
it before `T1`.

## Message flow

The end-to-end handshake (spec §8) is:

1. `init` — Alice proposes the swap (amounts, `H`, `T1`/`T2`, her pubkeys).
2. `accept` — Bob agrees and supplies his pubkeys. Both sides can now
   reconstruct both HTLCs (`SwapParams`, `swap.rs:74-91`).
3. `funded(A)` — Alice funds chain A and announces it; Bob verifies on chain.
4. `funded(B)` — Bob funds chain B and announces it; Alice verifies on chain.
5. `redeemed` — Alice redeems leg B (revealing `s`); the courtesy message tells
   Bob, but his daemon would find it by chain-watching anyway.
6. Bob extracts `s` and redeems leg A. The swap is `Completed`.

From step 3 onward the daemon's scheduler drives the swap automatically: it
funds, redeems, and (if a deadline passes with a leg unspent) refunds on a
clock. See the chapters "The Swap Lifecycle" and "Timelocks & Action
Deadlines".

## vsize reference

| Constant | Value (vB) | Transaction |
|---|---|---|
| `FUND_TX_VSIZE` | 160 | funding (preview estimate) |
| `REDEEM_TX_VSIZE` | 155 | hash-branch redeem |
| `REFUND_TX_VSIZE` | 146 | timeout-branch refund |

These worst-case vsizes turn a feerate into an absolute fee before the witness
exists. The fee floor for any HTLC spend is `MIN_SPEND_FEE_SAT = 1000`
(`swap.rs:38`).
