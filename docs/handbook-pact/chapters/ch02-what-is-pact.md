# What Pact Is

**Pact** is the swap engine at the heart of the Satchel project: the software
that builds, signs, broadcasts, and watches the on-chain transactions that make
a trustless cross-chain trade *atomic*. It is the component that holds your
keys, enforces the timelocks, and refunds you if a counterparty walks away. Everything
else in the project — the desktop app, the transports — exists to feed offers
to Pact and render what Pact reports back.

An *atomic swap* is a trade between two chains in which either both legs settle
or neither does. Pact implements this two ways: *v1*, a classic hash-locked HTLC
swap, and *v2*, a Taproot/MuSig2 *adaptor* swap. Both are covered in detail in
the protocol part of this handbook.

## The crate map

Pact is a small Rust workspace plus two supporting crates. Knowing which crate
owns what is the fastest way to navigate the codebase:

| Crate | Role |
|---|---|
| `libswap` | The engine proper: the HTLC and adaptor logic, the on-chain script construction, the key derivation, and the two swap **state machines**. All swap intelligence lives here; everything else is a shell around it. |
| `pactd` | The local daemon. Exposes `libswap` over **JSON-RPC 2.0**, persists state in **SQLite**, and runs the background scheduler that auto-refunds, auto-redeems, and RBF-fee-bumps with no human present. |
| `pact-cli` | A thin RPC client — a hand-rolled HTTP caller that maps subcommands (and a generic `call`) onto `pactd` methods. The integrator's command line. |
| `pact-proto` | The wire format and crypto primitives: signed envelopes, canonical JSON, BIP340 signatures, recipient-sealed relay blobs (`PACTSEALED1`), and private-offer slips. |
| `pact-nostr` | Pure mapping between Pact envelopes and Nostr events (public offer adverts as kind `31510`, gift-wrapped relay messages as kind `1059`). Mapping and crypto only — no relay I/O. |

> **Note** — `libswap` is where you look to verify how a swap is constructed;
> `pactd` is where you look to verify how it is *driven* and *persisted*. The
> two state machines (`swap::State` for v1, `AdaptorState` for v2) are defined
> in `libswap` and stepped by the engine's scheduler tick inside `pactd`.

`libswap`'s coin-generic plumbing — chain params and registry, wallet key
derivation, seed-at-rest storage, the Electrum connection layer, and the BDK
on-chain wallet — lives in the extracted **`btcx`** crates
(`github.com/PoC-Consortium/btcx`), consumed as rev-pinned git dependencies.
The extraction is behavior-preserving; all swap intelligence stays in
`libswap`.

## The bitcoind analogy

If you have run Bitcoin Core, the shape of Pact will be familiar — it is
deliberately the same:

- `pactd` ≈ `bitcoind` — a headless daemon you run and leave running, with a
  data directory, a `.cookie` file for RPC auth, and a JSON-RPC endpoint.
- `pact-cli` ≈ `bitcoin-cli` — a thin command-line client that reads the cookie
  and talks to the daemon.
- **Satchel** ≈ `bitcoin-qt` — the optional desktop GUI that bundles and
  supervises the daemon and renders its RPC. (Satchel is documented in its own
  handbook.)

The data layout, the cookie auth, the HTTP Basic credentials, and the
loopback-only listener are all modelled on Core. If you know how to run and
script `bitcoind`, you already know most of how to run and script `pactd`.

## What Pact deliberately is *not*

Pact is a swap *executor*, and nothing more. It is important to be precise about
the boundaries:

- **No custody.** Pact never holds your coins on your behalf in any account.
  Your funds are either in your own node's wallet or locked in a swap output
  that only you or your counterparty can spend — enforced by the chain, not by
  Pact.
- **No matching engine.** Pact does not pair buyers with sellers. It posts an
  offer to a transport and takes an offer from one; *finding* the offer is the
  transport's job (see the chapter *Architecture & Trust Boundaries*).
- **No fees.** There is no platform fee anywhere. The `platform_fee_sat` field
  reported by `estimateswapfees` is hard-wired to `0`; the only costs in a swap
  are the on-chain miner fees of its transactions.
- **No order book of record.** Offers live on transports (Nostr relays or a
  Corkboard), which are untrusted and replaceable. Pact derives which trading
  *pairs* are possible from coin capabilities; it does not curate a market.

## The market metaphor

The project's naming follows a village-market-square theme, and it maps cleanly
onto the components: a *pact* is the trustless deal itself, *posted on the
corkboard* (the transport), and *settled into your satchel* (the app and its
balances). There is deliberately no "exchange" or "DEX" branding — Pact is a
protocol and a daemon, not a venue.

## The first pair

The first supported trading pair is **BTCX ↔ BTC**, where *BTCX* is
Bitcoin-PoCX. More coins follow — Litecoin (LTC) was the first added third coin
— and new Bitcoin-Core-compatible chains can be added as data without
recompiling the engine (see the chapter *Coins, Pairs & Capabilities*). For the
shipped BTCX ↔ BTC pair the engine selects the v1 HTLC protocol by default; the
v2 adaptor protocol is used for Taproot pairs that lack an HTLC option.
