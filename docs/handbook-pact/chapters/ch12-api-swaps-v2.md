# API: v2 Adaptor Swaps

This chapter documents the **v2 (Taproot/MuSig2 adaptor)** swap RPCs. A v2
swap settles on the cooperative key-path (indistinguishable from an ordinary
Taproot spend) and uses a MuSig2 two-of-two with an adaptor signature instead
of an on-chain hashlock. The cryptographic rationale — why an adaptor, how the
MuSig2 nonce exchange binds the secret — is covered in the protocol chapter
"v2 Taproot/MuSig2 Adaptor Swaps". For shared RPC conventions see the chapter
"JSON-RPC Conventions"; for the simpler HTLC route see "API: v1 HTLC Swaps".

> **Note** — v2 adaptor swaps are **live on every network, including mainnet**.
> *v1 (HTLC) and v2 (Taproot/MuSig2 adaptor) swaps are reviewed and
> live on mainnet.*

## Swap state machine

A v2 swap is an `AdaptorSwapRecord` whose `state` advances through this enum
(snake_case in JSON):

```text
created → accepted → nonces_exchanged → signed
        → funded_a → funded_b → redeemed_b → completed
```

| State | Meaning |
|---|---|
| `created` | Initiator created the adaptor swap; terms set. |
| `accepted` | Counterparty accepted the terms. |
| `nonces_exchanged` | MuSig2 nonces exchanged for both legs. |
| `signed` | Adaptor + partial signatures produced. |
| `funded_a` | The first leg's funding tx is confirmed. |
| `funded_b` | The second leg's funding tx is confirmed. |
| `redeemed_b` | The B-side has been redeemed (adaptor secret revealed). |
| `completed` | Both legs settled successfully. |

Terminal failure states: `refunded`, `aborted`.

## Handshake order

The methods below are **not** interchangeable — they must be driven in this
order to complete the MuSig2 / adaptor handshake:

```text
adaptorinit → adaptoraccept → adaptorrecv → adaptorfundingready
            → adaptornonces → adaptorsign → adaptorassemble
            → adaptorfund → adaptorredeem | adaptorrefund
```

`init`/`accept`/`recv` establish terms; `fundingready` registers the funding
outpoint; `nonces`/`sign`/`assemble` complete the MuSig2 adaptor exchange;
`fund` broadcasts; and the swap settles with `redeem` (cooperative path) or
`refund` (timeout path). See "v2 Taproot/MuSig2 Adaptor Swaps" for why each
step is required.

> **Note** — The cooperative key-path **redeem is NOT RBF-bumpable**; it is
> handled by fee over-provisioning plus a CPFP child. The single-key
> **refund IS bumpable**. Budget fees accordingly when settling near a
> deadline.

## Read method

| Method | Params | Returns | Mutates |
|---|---|---|---|
| `listadaptorswaps` | — | `[AdaptorSwapRecord]` | no |

## Handshake & lifecycle methods

`give`/`get` use the same `coin:amount` string convention as v1 (e.g.
`"btcx:1.0"`); `t1`/`t2` are the two relative timelocks in blocks.

| Method | Params | Returns | Mutates |
|---|---|---|---|
| `adaptorinit` | `give`, `get`, `t1`, `t2` | `{ record, envelope }` | yes |
| `adaptoraccept` | `envelope` | `{ record, envelope }` | yes |
| `adaptorrecv` | `envelope` | `{ record }` | yes |
| `adaptorfundingready` | `swap_id`, `txid`, `vout` | `{ envelope }` | yes |
| `adaptornonces` | `swap_id` | `{ envelope }` | yes |
| `adaptorsign` | `swap_id` | `{ envelope }` | yes |
| `adaptorassemble` | `swap_id` | `{ record }` | yes |
| `adaptorfund` | `swap_id` | `{ envelope }` | yes (broadcasts) |
| `adaptorredeem` | `swap_id` | `{ record }` | yes (broadcasts) |
| `adaptorrefund` | `swap_id` | `{ record }` | yes (broadcasts) |

- `adaptorinit` — initiate a v2 swap with the given terms; returns the offer
  `envelope`.
- `adaptoraccept` — counterparty accepts; returns a reply `envelope`.
- `adaptorrecv` — ingest a counterparty envelope into the local record.
- `adaptorfundingready` — register the funding outpoint (`txid`, `vout`) once
  the funding UTXO exists; returns an envelope for the counterparty.
- `adaptornonces` — produce and exchange the MuSig2 nonces.
- `adaptorsign` — produce the adaptor and partial signatures.
- `adaptorassemble` — assemble the partials into the final signature set.
- `adaptorfund` — broadcast our funding transaction.
- `adaptorredeem` — settle on the cooperative key-path, revealing the adaptor
  secret.
- `adaptorrefund` — reclaim our funded leg via the single-key timeout path.
