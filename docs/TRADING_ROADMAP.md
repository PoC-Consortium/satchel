# PoCX Trading Roadmap — Trustless P2P Trading

This document describes the product strategy and regulatory position for
trustless peer-to-peer trading on PoCX: a slow, deliberate path to atomic-swap
trading with a non-custodial arranger that never touches funds.

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
4. The design constraints above are also the EU regulatory strategy (see the
   MiCA section at the bottom — they are load-bearing, not incidental).

## Technical foundation

- PoCX is a Bitcoin fork with Taproot `ALWAYS_ACTIVE` from genesis on mainnet
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

## Phase 2 — The arranger: order board (Corkboard) + blind relay

Corkboard is a web-based order board with a content-blind store-and-forward
relay for swap coordination. It never matches orders, never executes, never
custodies, and charges no fees.

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
- **No matching engine.** Humans pick offers. (Load-bearing for regulation.)

> **TODO:** Nostr relay transport alongside Corkboard (opt-in, engine
> untouched) — planned, not yet built. See the Nostr transport plan.

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

## EU regulation (MiCA) — summary, not legal advice

MiCA is in full EU-wide application; transition periods are over.

- **The coin itself:** no identifiable issuer (fair-launched, Bitcoin-like)
  → no whitepaper obligation. If a regulated platform lists it, *that
  platform* drafts the whitepaper.
- **The platform:** "operation of a trading platform" and "reception and
  transmission of orders" are licensed CASP activities. Recital 22 exempts
  services provided "in a fully decentralised manner without any
  intermediary" — but ESMA/EBA apply a **substance-over-form** test: an
  identifiable team controlling software, server, and user relationships can
  be a CASP even if non-custodial and fee-free.
- **The line that matters:** a *bulletin board* (signed offers, users contact
  each other) plus open-source swap software anyone can run is the strongest
  position. A *matching engine* looks like operating a trading platform,
  regardless of custody or fees. This is exactly why Corkboard is a
  noticeboard and never a matching engine.
- **Helps but doesn't immunize:** no fees (weakens "professional basis"),
  non-custodial (removes the custody service, not the platform question).
- **AML:** non-custodial / no-fiat is outside the obliged-entity perimeter
  today; the EU AML Regulation (from 2027) tightens this for CASPs.

**Action items**

- Keep the swap tooling as a standalone open-source project; aim for multiple
  independent operators of the order board (the Bisq structure — software
  plus community, no operating company — is the precedent).
- Decide who operates the board: core team vs. community.
- Obtain a legal opinion from a crypto-focused EU firm **before the order
  board goes live publicly** — the fully-decentralized exemption is exactly
  the gray zone where a short memo is worth the money.

### Sources

- [ESMA — MiCA](https://www.esma.europa.eu/esmas-activities/digital-finance-and-innovation/markets-crypto-assets-regulation-mica)
- [EBA/ESMA on DeFi and MiCA](https://news.bitcoin.com/we-are-defi-so-mica-does-not-apply-to-us-sorry-but-eba-and-esma-have-a-different-point-of-view/)
- [MiCA CASP licensing overview](https://blog.amlbot.com/mica-license-explained-casp-requirements-authorization-process-and-eu-passporting/)
- [MiCA 2026 timeline](https://sumsub.com/blog/crypto-regulations-in-the-european-union-markets-in-crypto-assets-mica/)

---

Phase 1 alone kills the "you send first" OTC pattern — OTC traders can swap
trustlessly using the `pact` CLI before touching any board or GUI.
