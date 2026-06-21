# Reference

A consolidated quick-reference for the whole handbook. Each entry points to the
chapter that covers it in full.

## Default ports & paths

| Item | Default | Notes |
|---|---|---|
| `pactd` JSON-RPC | `127.0.0.1:9737` | Loopback-enforced; `POST /`, `GET /health`. |
| Corkboard | `127.0.0.1:9780` | `--listen`; HTTP API. |
| Nostr relays | none (engine) | Default 6 live in Satchel; `pactd --nostr-relay` empty. |

Per-merchant data directory (flat root, or `merchants/<id>/` nested):

| File | Contents |
|---|---|
| `pact.sqlite` | Swap/offer/nonce/Nostr state. |
| `seed.mnemonic` | BIP39 seed (plaintext, or `PACTSEEDv1:<salt>:<nonce>:<ciphertext>` when encrypted). |
| `.cookie` | Per-run RPC cookie `__cookie__:<hex>` (data-dir root only). |
| `pact.conf` | Optional `rpcuser` / `rpcpassword`. |
| `merchants.json` | Merchant manifest (parent data dir, nested mode). |
| `logs/pactd.log.<date>` | Rolling daily log (data-dir root); secret-free; `RUST_LOG`, default `INFO`. |

`pact.sqlite` tables: `swaps`, `meta` (counters / `relay_cursor` / private
offers), `pending_takes`, `nonce_sessions`, `adaptor_swaps`, `nostr_outbox`,
`nostr_inbox`, `nostr_offer_cache`, `my_offers`.

Corkboard SQLite tables: `offers`, `relay`. See "Corkboard (self-hostable
board)".

## RPC method index

All methods are JSON-RPC over `POST /`. Grouped by area; see the named chapter
for params and return shapes.

**Node / info** (*"API: Node, Seed, Merchants, Coins"*): `getinfo`,
`walletstatus`, `stop`, `getfeepolicy`, `setfeepolicy`.

**Seed lifecycle** (same chapter): `createseed`, `generateseed`, `importseed`,
`unlock`.

**Merchants** (same chapter): `createmerchant`, `listmerchants`, `loadmerchant`,
`unloadmerchant`, `getmerchantinfo`.

**Coins / pairs** (same chapter): `listcoins`, `listpairs`, `validatecoin`.

**Wallet helpers** (same chapter): `getbalance`, `getnewaddress`,
`sendtoaddress`.

**Swaps — v1 HTLC** (*"API: v1 HTLC Swaps"*): `listswaps`, `getswap`,
`listpendingtakes`, `listmyoffers`, `offer`, `acceptoffer`, `recv`, `fund`,
`redeem`, `refund`, `abort`, `tick`.

**Swaps — v2 adaptor** (*"API: v2 Adaptor Swaps"*): `listadaptorswaps`,
`adaptorinit`, `adaptoraccept`, `adaptorrecv`, `adaptorfundingready`,
`adaptornonces`, `adaptorsign`, `adaptorassemble`, `adaptorfund`,
`adaptorredeem`, `adaptorrefund`.

**Board** (*"API: Board, Private Offers & Fees"*): `boardlistoffers`,
`boardstatus`, `boardpostoffer`, `boardtake`, `boardrevoke`.

**Private offers** (same chapter, and "Private (Off-Market) Offers"):
`makeprivateoffer`, `takeoffer`, `listprivateoffers`, `cancelprivateoffer`.

**Fees** (same chapter): `estimateswapfees`.

**Diagnostics** (*"API: v1 HTLC Swaps"*): `dumpswap` (secret-free per-swap
record + log bundle; works for v1 and v2).

## Error format

Failures return a JSON-RPC `error` object (not an HTTP error status):

```json
{ "jsonrpc": "2.0", "id": 1, "error": { "code": -1, "message": "<reason>" } }
```

`code` is always `-1`. Common messages: `unknown method '<m>'`,
`missing param '<name>'`, `no active merchant — create or load one first`. See
"JSON-RPC Conventions".

## BIP32 derivation paths

All keys derive from one BIP39 seed under purpose `7228'`. `coin(c)`: BTC = `0`,
PoCX = `0x504F4358`. See "HTLC v1 (Construction)" and "v2 Adaptor (Construction)".

| Key | Path | Use |
|---|---|---|
| Identity | `m/7228'/0'/0'` | BIP340 x-only; signs envelopes; equals the Nostr npub. Never used in an HTLC. |
| Swap key | `m/7228'/1'/coin(c)'/i'` | One compressed secp key per chain per swap (ECDSA in v1; reused as the MuSig2 x-only signer in v2). |
| Preimage source | `m/7228'/2'/i'` | `s = TaggedHash("pact/htlc/preimage/v1", priv)`, `H = SHA256(s)` (v1, Alice-only). |
| Adaptor secret source | `m/7228'/2'/i'` | `t = TaggedHash("pact/adaptor/secret/v2", priv) mod n`, `T = t·G` (v2, Alice-only). |
| Refund key (v2) | `m/7228'/3'/coin(c)'/i'` | x-only single-key CLTV tapleaf, independent of MuSig2. |

## Tagged-hash tags

`tagged_hash(tag, msg) = SHA256(SHA256(tag) || SHA256(tag) || msg)`.

| Tag | Used for |
|---|---|
| `pact/msg/v1` | Envelope signing digest. |
| `pact/swapid/v1` | v1 `swap_id` (over `H`). |
| `pact/swapid/v2` | v2 `swap_id` (over `T`). |
| `pact/htlc/preimage/v1` | v1 preimage `s`. |
| `pact/adaptor/secret/v2` | v2 adaptor secret `t`. |
| `pact/relay/ecdh/v1` | Sealed-blob symmetric key (ECDH → ChaCha20-Poly1305). |

## Spec & vectors file map

| File | Contents |
|---|---|
| `spec/protocol.md` | HTLC v1 (`pact-htlc-v1`): scripts, tx templates, key paths, preimage/timelock rules, the §8 message handshake. |
| `spec/protocol-v2.md` | v2 (`pact-htlc-v2`): Taproot/MuSig2 adaptor swaps; specifies only what changes from v1. |
| `spec/vectors/htlc_v1.json` | v1 test vectors (pinned by `tests/vectors.rs`). |
| `spec/vectors/htlc_v2.json` | v2 test vectors (pinned by `tests/vectors_v2.rs`). |

## Glossary

- **HTLC** — Hash Time-Locked Contract; the v1 swap output: spendable by a hash
  preimage (redeem) or after a CLTV timelock (refund).
- **Adaptor signature** — a signature missing a secret scalar; completing it on
  one chain reveals that scalar, letting the counterparty complete the other.
  The basis of v2 swaps.
- **MuSig2** — a two-round Schnorr multisignature; the 2-of-2 aggregate that
  forms the v2 Taproot internal key.
- **Tapleaf** — a script leaf in a Taproot tree; v2 uses one, the single-key CLTV
  refund script.
- **CLTV** — `OP_CHECKLOCKTIMEVERIFY`; enforces an absolute locktime, used for
  the refund branch. Pact requires Unix-time locktimes (`≥ 500000000`).
- **MTP** — median time past; the 11-block median a node uses to evaluate
  time-based locktimes. Refund readiness is keyed on MTP.
- **RBF** — Replace-By-Fee; rebroadcasting a tx with a higher fee. v1 redeem and
  refund and the v2 single-key refund are RBF-bumpable.
- **CPFP** — Child-Pays-For-Parent; a child tx that bumps a stuck parent's
  effective fee. Used to accelerate the **unbumpable** v2 cooperative redeem.
- **Preimage** — the secret `s` whose hash `H = SHA256(s)` locks a v1 HTLC;
  revealing it on one chain unlocks the other.
- **Sweep address** — a fresh wallet-owned address a v2 cooperative redeem sweeps
  to (`alice_sweep_b` / `bob_sweep_a`).
- **Merchant** — one seed bound to one data directory; the engine's wallet
  analog. RPC calls target the active merchant.
- **Noticeboard** — the pluggable offer/relay transport behind the five-method
  `Noticeboard` trait (Corkboard over HTTP, or Nostr).
- **Slip** — a private (off-market) offer serialized as `pactoffer1:<base64url>`;
  the same signed `offer` envelope, never posted to a board.
