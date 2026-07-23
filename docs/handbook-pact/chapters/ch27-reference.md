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
| `.lock` | Exclusive daemon lock (data-dir root only); a second `pactd` on the same data dir refuses to start. |
| `machine.json` | Per-install derive scope for multi-machine seed partitioning (data-dir root only). |
| `pact.conf` | Optional `rpcuser` / `rpcpassword`. |
| `merchants.json` | Merchant manifest (parent data dir, nested mode). |
| `logs/pactd.log.<date>` | Rolling daily log (data-dir root); secret-free; `RUST_LOG`, default `INFO`. |

`pact.sqlite` tables: `swaps`, `meta` (counters / `relay_cursor` / private
offers), `pending_takes`, `nonce_sessions`, `adaptor_swaps`, `nostr_outbox`,
`nostr_inbox`, `nostr_offer_cache`, `my_offers`.

Corkboard SQLite tables: `offers`, `relay`. See "Corkboard (self-hostable
board)".

## RPC method index

All methods are JSON-RPC over `POST /` — **66 public methods**, grouped by
area; see the named chapter for params and return shapes. The daemon prints
the same catalog live: `help` (by category, plain text) and `listmethods`
(a name array).

**Node / info** (*"API: Node, Seed, Merchants, Coins"*): `getinfo`,
`walletstatus`, `help`, `listmethods`, `stop`, `getfeepolicy`,
`setfeepolicy`.

**Seed-only rescue / multi-machine** (same chapter; full mechanics in "Seeds,
Wallets & Merchants"): `restorefromrelay` (adopt rescuable relay snapshots),
`rescuestatus` (read-only preview + the two-machines warning), `takeover`
(adopt another machine's swap after confirming that machine is stopped).

**Seed lifecycle** (same chapter): `createseed`, `generateseed`, `importseed`,
`unlock`.

**Merchants** (same chapter): `createmerchant`, `listmerchants`, `loadmerchant`,
`renamemerchant`, `unloadmerchant`, `getmerchantinfo`.

**Coins / pairs** (same chapter): `listcoins`, `listpairs`, `validatecoin`,
`serverstatus`.

**Wallet helpers** (same chapter): `getbalance`, `getnewaddress`,
`estimatesendfee`, `sendtoaddress`, `bumpfee`, `listtransactions`.

**Swaps — v1 HTLC** (*"API: v1 HTLC Swaps"*): `listswaps`, `getswap`,
`listpendingtakes`, `listmyoffers`, `offer`, `acceptoffer`, `recv`, `fund`,
`redeem`, `refund`, `abort`, `tick`.

**Swaps — v2 adaptor** (*"API: v2 Adaptor Swaps"*): `listadaptorswaps`,
`adaptorinit`, `adaptoraccept`, `adaptorrecv`, `adaptorfundingready`,
`adaptornonces`, `adaptorsign`, `adaptorassemble`, `adaptorfund`,
`adaptorredeem`, `adaptorrefund`.

**Board** (*"API: Board, Private Offers & Fees"*): `boardlistoffers`,
`boardstatus`, `boardpostoffer`, `boardtake`, `boardrevoke`,
`revokeoffersforcoin`.

**Private offers** (same chapter, and "Private (Off-Market) Offers"):
`makeprivateoffer`, `takeoffer`, `listprivateoffers`, `cancelprivateoffer`.

**Fees** (same chapter): `estimateswapfees`.

**Diagnostics** (*"API: v1 HTLC Swaps"*): `dumpswap` (secret-free per-swap
record + log bundle; works for v1 and v2); `swapprogress` (live snapshot of every
active swap — confirmations and feerate; v1 and v2).

## Error format

Failures return a JSON-RPC `error` object (not an HTTP error status):

```json
{ "jsonrpc": "2.0", "id": 1, "error": { "code": -1, "message": "<reason>" } }
```

`code` is `-32601` for an unknown method (with a *did-you-mean* suggestion)
and `-1` for everything else. Common messages: `unknown method '<m>' — did
you mean '<nearest>'? (see 'help')`, `missing param '<name>'`,
`no active merchant — create or load one first`. See "JSON-RPC Conventions".

## BIP32 derivation paths

All keys derive from one BIP39 seed under purpose `7228'`. `coin(c)`: BTC = `0`
(all networks); PoCX = `0x504F4358` on mainnet, `1'` on testnet/regtest
(network-aware, for Bitcoin Core / Phoenix parity). See "HTLC v1 (Construction)"
and "v2 Adaptor (Construction)".

| Key | Path | Use |
|---|---|---|
| Identity | `m/7228'/0'/0'` | BIP340 x-only; signs envelopes; equals the Nostr npub. Never used in an HTLC. |
| Swap key, initiator (Alice) | `m/7228'/1'/coin(c)'/i'` | One compressed secp key per chain per swap (ECDSA in v1; reused as the MuSig2 x-only signer in v2), indexed by the local counter `i`. |
| Swap key, participant (Bob), anchored | `m/7228'/1'/coin(c)'/a'/b'/c'/d'` | Same key type; `a,b,c,d` are the first four masked-31-bit words of `TaggedHash("pact/swap-key-anchor/v1", anchor)` (`H` for v1, `T` for v2, spec §4.2) — no counter needed, re-derivable from the seed plus the anchor alone. |
| Preimage source | `m/7228'/2'/i'` | `s = TaggedHash("pact/htlc/preimage/v1", priv)`, `H = SHA256(s)` (v1, Alice-only). |
| Adaptor secret source | `m/7228'/2'/i'` | `t = TaggedHash("pact/adaptor/secret/v2", priv) mod n`, `T = t·G` (v2, Alice-only). |
| Refund key (v2), initiator | `m/7228'/3'/coin(c)'/i'` | x-only single-key CLTV tapleaf, independent of MuSig2. |
| Refund key (v2), participant, anchored | `m/7228'/3'/coin(c)'/a'/b'/c'/d'` | Same anchored derivation as the participant swap key, on branch `3'`. |

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
| `pact/swap-key-anchor/v1` | Participant's anchored swap/refund key derivation (over `H` or `T`, §4.2). |
| `pact/rescue/dtag/v1` | Opaque per-swap `d`-tag for a rescue snapshot event (over the `swap_id`). |

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
- **Rescue snapshot** — an encrypted-to-self Nostr event (kind `31512`) holding
  enough of an in-flight swap's state that a machine restored from the seed
  alone can adopt and finish it. Published at `accepted` (v1 and v2) and again
  at `signed` (v2 only); tombstoned on terminal states. See "Seeds, Wallets &
  Merchants".
- **`PRE_FUNDING_TIMEOUT_SECS`** — 15 minutes; the shared deadline for three
  self-healing behaviors: a v2 handshake stuck before either leg funds
  auto-aborts, a v1 swap stuck in `Created` with no accept auto-aborts, and a
  board `take` whose signed `taken_at` is older than this is silently dropped
  rather than served.
