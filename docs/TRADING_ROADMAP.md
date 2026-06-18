# Bitcoin PoCX Trading Roadmap — Trustless P2P Trading

This document describes the product strategy and regulatory position for
trustless peer-to-peer trading on Bitcoin PoCX: a slow, deliberate path to
atomic-swap trading with a non-custodial arranger that never touches funds.

The infrastructure built here is **not limited to Bitcoin PoCX**. The swap
engine is coin-agnostic — coins are configured by id and Electrum backend,
trading pairs are derived from a registry, and any Bitcoin-family UTXO chain
with the required script support (CLTV/CSV, SegWit, Schnorr/Taproot) drops in
without code changes. The coordination layer (Corkboard + Nostr) is likewise
coin-blind: it moves signed offer envelopes and opaque blobs regardless of
which chains a swap touches. Bitcoin PoCX is the first deployment of this
tooling, not the boundary of it.

## Problem

Community demand for trading is real, and OTC trades already happen with
"you send first" counterparty risk — the opposite of what crypto is for. The
answer is not a third-tier centralized exchange. It is a trustless trading
protocol (atomic swaps) plus a non-custodial coordination layer (web +
Discord front-end) that never custodies, matches, or executes.

## Guiding principles

1. **Separate the protocol from the platform.** The trust fix is the atomic
   swap protocol itself; the website and Discord bot are only coordination.
   The tooling comes first, the venue second.
2. **The platform is a noticeboard, not an exchange.** It never matches
   orders, never executes, never custodies, and charges no fees.
3. **Slow start.** Small per-trade caps, fixed-size offers, no partial fills,
   community-operated.
4. These constraints follow from the trustless architecture; the clean
   regulatory position is a consequence of that design, not its goal (see the
   MiCA section at the bottom).

## Technical foundation

- Bitcoin PoCX is a Bitcoin fork with Taproot `ALWAYS_ACTIVE` from genesis on mainnet
  (`bitcoin/src/kernel/chainparams.cpp`), with the full script set: CLTV, CSV,
  SegWit, Schnorr.
- Every modern atomic-swap technique works out of the box: classic HTLC swaps
  and Taproot adaptor-signature swaps (MuSig2).
- Chain data comes from the Electrum protocol on both chains (`electrs-pocx`
  on the PoCX side), so no changes to the stock node are required — RPC/ZMQ
  and Electrum only.

---

## Phase 1 — Swap protocol + CLI tooling

PoCX ↔ BTC/LTC swaps between UTXO chains, classic HTLC flow:

1. Alice locks PoCX in an HTLC spendable by Bob with secret `s` (hash `H`),
   refundable to Alice after timelock `T1`.
2. Bob verifies, locks BTC in an HTLC with the same `H`, refundable after
   `T2 < T1`.
3. Alice claims the BTC, revealing `s` on-chain; Bob uses `s` to claim the
   PoCX. Both legs complete or both refund — nobody sends first.

The protocol is specified independently of any implementation so third
parties can build their own clients: see [`../spec/protocol.md`](../spec/protocol.md)
(`pact-htlc-v1` — scripts, tx templates, key-derivation paths, preimage and
timelock rules, and the counterparty handshake) with deterministic test
vectors under [`../spec/vectors/`](../spec/vectors/).

The Pact engine implements this end to end:

- **`libswap`** — the swap engine: HTLC construction, the message handshake,
  the per-coin registry and capability/pair resolution, chain access over
  Electrum, and the swap state store.
- **`pactd`** — the daemon exposing a local RPC API and managing the hot
  transit seed (per-merchant data dirs, encrypted or unencrypted, with
  seed-lifecycle calls: status / create / import / unlock).
- **`pact` CLI** — the first API client: `offer`, `accept`, `recv`, `fund`,
  `redeem`, `refund`, `abort`, `status`, plus wallet/seed and coin-setup
  commands (`coins`, `pairs`, `validatecoin`) and a `board` subcommand
  (`post` / `offers` / `take` / `revoke` / `sync`).

The engine is coin-agnostic: coins are configured by id and Electrum backend
URLs (`--coin id=urls`), and supported trading pairs are derived from the
registry. Everything is proven on regtest first, with regtest/e2e suites and
pinned test vectors.

**v2 — adaptor-signature swaps.** Taproot/MuSig2 adaptor swaps
(`pact-htlc-v2`) are implemented in `libswap` (functional MuSig2 adaptor API,
write-ahead use-once nonce store, proven end to end over a chain backend) and
surfaced in Satchel. On-chain these look like ordinary single-key payments:
better privacy, smaller transactions, no swap script revealed. v2 swaps are
**mainnet-gated until audited**. Route, rationale, and protocol delta:
[V2_ADAPTOR_SWAPS.md](V2_ADAPTOR_SWAPS.md) and
[`../spec/protocol-v2.md`](../spec/protocol-v2.md).

## Phase 2 — The arranger: order boards + blind relay (Corkboard + Nostr)

Coordination rides two interchangeable transports behind one engine boundary —
the `Noticeboard` trait (`libswap/src/board.rs`): a self-hosted HTTP order
board (**Corkboard**) and the public **Nostr** relay network. Both carry the
same signed offer envelopes and the same sealed relay blobs; the swap engine is
identical regardless of which is used, and an operator can run either, both, or
neither. Neither transport matches orders, executes, custodies, or charges
fees.

### Corkboard (self-hosted HTTP board)

Corkboard is a web-based order board with a content-blind store-and-forward
relay for swap coordination.

- **Signed offers.** Offers are posted as signed envelopes; listings can't be
  faked and are provably tied to a key. The board verifies the envelope
  signature and stores the offer.
- **Blind relay.** `POST /v1/relay` is store-and-forward keyed by recipient;
  the relayed blob is opaque to the board (a serialized envelope today, an
  encrypted blob with client-side E2E encryption). `POST /v1/relay/poll`
  takes a signed poll and returns messages since a cursor. The board never
  inspects relayed payloads, and the relay enforces a size cap — it is for
  coordination, not bulk data.
- **Swap wizard, client-side.** Users browse offers and contact each other;
  the actual swap runs through Pact on both sides via a swap wizard. The
  server only relays signed coordination messages.
- Trust comes from atomicity alone. There are no accounts, no KYC, no
  custody, and no fees.
- A reference price may be displayed for information only — never executed
  against.
- **No matching engine.** Humans pick offers — there is nothing to route or
  execute.

### Nostr transport (decentralized relay network)

A second transport carries the same offers and relay messages over the public
**Nostr** relay network, alongside Corkboard rather than replacing it
(`pact-nostr` + `pactd`'s `nostr_service`). It is **opt-in**: with no relays
configured nothing is published, so a swap never touches a public relay until
the operator adds one.

- **No board to operate.** Offers and coordination traffic ride commodity Nostr
  relays run by many independent operators; there is no central server, account,
  or operator that controls the network. If a dedicated relay is wanted, running
  one is running stock relay software — it is blind to its payloads.
- **Identity is the user's key.** A Pact identity *is* a Nostr key (BIP340
  Schnorr secp256k1); a maker's `npub` equals the `from` of their signed offer.
  No accounts, no registration.
- **Same blindness as Corkboard.** Public offers are signed addressable events
  (kind `31510`, NIP-40 expiry). Private coordination rides NIP-59-style gift
  wraps signed by an ephemeral key: a relay sees the recipient and ciphertext,
  never the sender or the contents.
- **Liveness, not safety.** A relay that withholds or delays events affects
  liveness only; funds are protected by timelocks regardless — identical to the
  Corkboard relay.

Wire mapping and design: [NOSTR_TRANSPORT.md](NOSTR_TRANSPORT.md) and
[`../spec/protocol.md`](../spec/protocol.md) §8.8.

**Known failure modes the design accounts for**

- *Free-option problem:* a second mover can stall and complete only if price
  moves in their favor. Mitigated by short timelocks (hours, not days) and
  small per-trade caps.
- *Refund UX:* a user must reliably reclaim funds after timeout even if
  offline — the swap flow schedules and broadcasts refund transactions
  automatically.
- Offers are fixed-size with no partial fills in v1.

## Phase 3 — GUI: Satchel

Satchel is the desktop application that puts swaps in reach of non-technical
holders. It is a Tauri app: a Rust bridge (`satchel/src/main.rs`) over a
React + MUI front-end (`satchel/ui/`). It contains no swap logic — it is a
thin client of `pactd`'s local API.

- Merchant manager and first-run wizard: one seed = one data dir, with the
  seed-provisioning flow over the daemon's seed-lifecycle calls.
- Per-coin configuration (validate-genesis-then-save) and a per-coin wallet
  view.
- Corkboard view: offer cards filtered to supported pairs, with implied rate,
  freshness, and timelocks; plus a noticeboard configuration.
- Swaps view: active and historical swaps, including v2 adaptor swaps marked
  with a "Private (Taproot)" badge.

Design references: [SATCHEL.md](SATCHEL.md), [SATCHEL_UI.md](SATCHEL_UI.md),
[SATCHEL_BACKEND.md](SATCHEL_BACKEND.md).

The Discord front-end (**Crier**) is a Corkboard front-end only — browse/post
offers and notifications, deep-linking into the user's local Satchel/Pact to
execute. It never touches keys or funds.

> **TODO:** Crier (Discord bot) — scaffolded only; not yet implemented.

## Phase 4 — External venues, on our terms

> **TODO:** External-venue listings — genuinely future work, not yet started.

- **Komodo Wallet (formerly AtomicDEX)** listing — a P2P atomic-swap DEX built
  for UTXO coins; integration is mostly a coin-config PR.
- Bisq as a philosophically aligned model/venue to study.
- **Avoid wrapped-token bridges** — they reintroduce the custodian we are
  trying to avoid.
- Still no third-tier centralized exchanges.

---

## EU regulation (MiCA)

MiCA is in full EU-wide application. Which, if any, component is a Crypto-Asset
Service Provider (CASP) follows from what each part of the system technically
holds and controls — keys, funds, order flow, the user relationship — not from
how it is labelled. ESMA/EBA apply a **substance-over-form** test, so the
analysis below is by substance.

- **The coin.** Bitcoin PoCX has no identifiable issuer (fair-launched, Bitcoin-like),
  so no whitepaper obligation attaches to it. If a regulated platform later
  lists it, *that platform* carries the whitepaper duty.
- **The engine and clients (`libswap`/`pactd`/`pact`, Satchel).** Keys and
  funds never leave the user's machine; the engine only builds, signs, and
  broadcasts the user's *own* transactions, and atomicity is enforced by the
  on-chain timelocks, not by any operator. This is open-source software a user
  runs against their own keys — no order reception, no execution, no custody,
  no service relationship. None of the licensed CASP activities are present.
- **The transport — by construction not a venue.** Both transports are
  content-blind message buses, not order systems:
  - *Nostr* is a public, general-purpose relay network. A relay stores and
    forwards opaque events; it does not parse offers, match, route, or hold
    keys, and identity is a user-held key rather than an account. There is no
    intermediary that controls the network or the user relationship — the
    decentralisation is a real property of the transport (Recital 22's
    "fully decentralised" case), the same way SMTP or BitTorrent is.
  - *Corkboard* is a single self-hosted HTTP deployment of the same thing:
    it verifies offer signatures, stores signed offers, and forwards opaque
    blobs — and nothing else. It is interchangeable with a Nostr relay and can
    be replaced entirely by pointing clients at Nostr.
- **The two licensed activities, applied.** "Operation of a trading platform"
  requires bringing together multiple third-party orders so they can be
  *matched* into a contract; "reception and transmission of orders" requires
  receiving an order and passing it on for execution. Neither component does
  either: there is no matching engine (humans read a board and contact each
  other), no order routing, and no execution — the relay never sees an "order"
  it could transmit, only a signed advert or an opaque blob. Non-custody and
  zero fees are additional facts (no custody service; no "professional basis"
  consideration), but the load-bearing point is the absence of matching and
  execution in the code itself.
- **AML.** Non-custodial and no-fiat keeps the relays and clients outside the
  obliged-entity perimeter today; the EU AML Regulation (from 2027) tightens
  this for entities that *are* CASPs — which these are not.

**Operating posture**

- Keep the swap tooling a standalone open-source project anyone can run, so the
  software exists independently of any one operator (the Bisq precedent —
  software plus community, no operating company).
- Prefer the Nostr transport as the default coordination layer: it has no
  operator to be, and a self-run relay is commodity infrastructure blind to its
  payloads. A self-hosted Corkboard is an optional convenience, replaceable by
  Nostr at any time.

### Sources

- [ESMA — MiCA](https://www.esma.europa.eu/esmas-activities/digital-finance-and-innovation/markets-crypto-assets-regulation-mica)
- [EBA/ESMA on DeFi and MiCA](https://news.bitcoin.com/we-are-defi-so-mica-does-not-apply-to-us-sorry-but-eba-and-esma-have-a-different-point-of-view/)
- [MiCA CASP licensing overview](https://blog.amlbot.com/mica-license-explained-casp-requirements-authorization-process-and-eu-passporting/)
- [MiCA 2026 timeline](https://sumsub.com/blog/crypto-regulations-in-the-european-union-markets-in-crypto-assets-mica/)

---

Phase 1 alone kills the "you send first" OTC pattern — OTC traders can swap
trustlessly using the `pact` CLI before touching any board or GUI.
