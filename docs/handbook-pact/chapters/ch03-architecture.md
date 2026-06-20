# Architecture & Trust Boundaries

Pact's architecture is built around one rule: **the things that can lose your
money never leave your machine, and the things that leave your machine can
never lose your money.** Understanding where that wall sits is the single most
important thing for anyone integrating against, operating, or auditing the
system.

## The three moving parts

A running swap involves three categories of component, with a hard trust wall
between the first two (yours) and the third (hosted and untrusted):

1. **Your machine — fully trusted.** This is the stack you run and control:

   - **Satchel** (or any RPC client) — the face. It renders the engine's RPC
     and never touches swap logic.
   - **`pactd`** — the swap engine. It owns the BIP39 *seed*, derives every key,
     builds and broadcasts the swap transactions, and runs the scheduler that
     auto-refunds if a counterparty disappears.
   - Your **coin backends** — a BTCX node and a BTC node (or Electrum backend),
     reached over their own RPC. These hold the actual coins and the funding
     wallet.

2. **Hosted, untrusted transport.** A **Nostr relay** (the default) or a
   **Corkboard** instance (self-hostable). Its only job is to carry
   identity-signed offers and forward sealed coordination blobs between
   counterparties. It is replaceable, runs no swap logic, and is assumed
   hostile.

> **Note** — "Three moving parts" counts the transport as one part even though
> you can point Pact at several relays or boards at once. They are
> interchangeable couriers, not authorities.

## The hard wall

The boundary is precise and worth stating in operational terms:

- **Keys and refunds are local, always.** The seed, every derived key, the
  preimage (v1) and adaptor secret (v2), the pre-signed refund transactions, and
  the entire decision to redeem or refund live inside `pactd` on your machine.
  None of this is ever sent to a transport.
- **The transport sees only signed offers and sealed blobs.** What leaves your
  machine is (a) an *identity-signed offer* — an advert that you are willing to
  trade, signed by your BIP340 identity key so it cannot be forged or altered —
  and (b) *recipient-sealed coordination blobs* (`PACTSEALED1`), encrypted to
  the counterparty so the relay operator sees ciphertext only. A relay cannot
  read your coordination, cannot move your funds, and cannot match, execute,
  custody, or charge.
- **The RPC is loopback-only.** `pactd` binds `127.0.0.1` by default and
  **enforces** that its listen address is a loopback address — a non-loopback
  `--listen` aborts boot. The JSON-RPC surface is for local clients (the CLI,
  Satchel) reading the on-disk cookie; it is never meant to face a network.

> **Warning** — Because the RPC has no remote-access design, do not put `pactd`
> behind a reverse proxy or expose its port. Anything that can reach the RPC and
> read the `.cookie` can drive your swaps and your wallet. Keep it loopback.

## Data flow of an offer

At a high level, posting and discovering an offer looks like this:

1. You call an offer-posting RPC on `pactd` with the coins, amounts, and
   timelocks.
2. `pactd` builds an offer envelope and **signs it with your identity key**
   (the BIP340 key at `m/7228'/0'/0'`, which never appears in any on-chain
   script).
3. `pactd` hands the signed envelope to each configured transport, which
   publishes it (Nostr kind `31510`, or a Corkboard row).
4. A counterparty's `pactd`, browsing that transport, sees the signed advert,
   verifies the signature, and — if they want it — *takes* it, which kicks off
   the coordination flow.

The transport stored and forwarded a signed advert. It never learned a key.

## Data flow of a swap

Once an offer is taken, the two daemons coordinate to build and settle the swap:

1. The takers exchange a short sequence of **sealed messages** through the
   transport — funding details, public keys, sweep addresses, and (for v2) the
   MuSig2 nonces and adaptor signatures. Each message is sealed to its recipient
   (`PACTSEALED1`), so the relay forwards opaque ciphertext.
2. Each side independently **reconstructs the on-chain output** (the HTLC P2WSH
   for v1, or the Taproot output for v2) and byte-compares it before any funds
   move — neither side trusts the other's claim about the script.
3. The maker funds first; the taker verifies the funding on-chain, then funds
   its leg. Redemption on one chain reveals the secret (the preimage for v1, the
   adaptor secret `t` for v2) that lets the counterparty claim the other chain.
4. Throughout, each `pactd` scheduler tick watches both chains and, if a
   deadline passes with the swap unfinished, broadcasts the pre-signed (v1) or
   re-derived (v2) **refund** — no human and no transport required.

The detailed message sequence, the exact scripts, and the timelock margins are
covered in the protocol part of this handbook — *see the chapters "What Pact Is"
and the swap-protocol chapters*. The point here is structural: all of step 2
through step 4 happen between two local engines and two sets of nodes; the
transport only relayed sealed blobs in step 1.

## The picture

```text
  ┌─────────────────────────────┐         ┌──────────────────────────┐
  │  Your machine (trusted)     │         │  Hosted (untrusted)      │
  │                             │         │                          │
  │  Satchel (desktop GUI) /    │ signed  │  Nostr relays            │
  │  any RPC client             │ offers  │   (default transport)    │
  │      │ JSON-RPC (loopback)  │   +     │                          │
  │      ▼                      │ sealed  │  ...or a Corkboard       │
  │  pactd (swap engine)────────┼────────►│   instance               │
  │      │ owns BIP39 seed,     │  blobs  │   (self-hostable)        │
  │      │ keys, refunds        │         │                          │
  │      ▼                      │         └──────────────────────────┘
  │  BTCX node + BTC backend    │
  └─────────────────────────────┘
```

Everything left of the arrow is yours and trusted. Everything right of it is
hosted, replaceable, and assumed hostile. The arrow carries only signed adverts
and sealed ciphertext — never a key, never custody, never a fee.
