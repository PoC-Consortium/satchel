# Architecture

Satchel is a small stack of components with **one hard wall** between your machine and anything hosted. Everything that touches keys, seeds, and refunds runs locally; the transports only ever see signed offers and sealed ciphertext.

## Components

| Folder | Name | What it is |
|---|---|---|
| `spec/` | — | The atomic-swap protocol spec (v1 HTLC + v2 adaptor) and deterministic test vectors, written so third parties can implement independently. |
| `pact-proto/` | — | Wire format + crypto primitives: signed envelopes, canonical JSON, BIP340 signing, recipient-sealed relay blobs (`PACTSEALED1`), private-offer slips. |
| `pact-nostr/` | — | Pure mapping between Pact envelopes and Nostr events (public offers kind `31510`, gift-wrapped messages kind `1059`). No relay I/O of its own. |
| `pact/` | **Pact** | The swap engine (Rust workspace): `libswap` (HTLC/adaptor logic + state machine), `pactd` (local JSON-RPC daemon, SQLite, auto-refund + fee-bump scheduler), `pact-cli` (thin RPC client). |
| `corkboard/` | **Corkboard** | Self-hostable order board: a single axum + SQLite/Postgres binary that stores signed offers and blind-relays sealed blobs. The alternative transport to Nostr. |
| `satchel/` | **Satchel** | Desktop app (Tauri shell + React/Vite/TypeScript/MUI). Bundles and supervises `pactd`, manages seeds, offers per-coin wallet cards (balances, send/receive; activity for Electrum coins). Owns the GUI, never the swap logic. |

## The trust boundary

```
  ┌─────────────────────────────┐         ┌──────────────────────────┐
  │  Your machine               │         │  Hosted (untrusted)      │
  │                             │         │                          │
  │  Satchel (desktop GUI)      │ signed  │  Nostr relays            │
  │      │ JSON-RPC (loopback)  │ offers  │   (default transport)    │
  │      ▼                      │   +     │                          │
  │  pactd (swap engine)────────┼────────►│  ...or a Corkboard       │
  │      │ owns BIP39 seed,     │ sealed  │   instance               │
  │      │ keys, refunds        │  blobs  │   (self-hostable)        │
  │      ▼                      │         │                          │
  │  BTCX node + BTC backend    │         └──────────────────────────┘
  └─────────────────────────────┘
```

The wall is concrete, not aspirational:

- **Keys, the BIP39 seed, and refund transactions never leave `pactd`.** The engine derives all swap keys from the seed, signs everything locally, and broadcasts refunds itself.
- **The transports are blind.** They carry identity-signed *offers* (intentionally public) and forward *sealed* coordination blobs. Operators see only ciphertext — there is no plaintext downgrade. A relay can withhold a message but cannot read or alter it, and cannot touch funds (timelocks protect those).
- **The RPC is loopback-only.** `pactd` refuses to bind to a non-loopback address; Satchel's webview never even sees the RPC cookie — it talks through a single in-process proxy.

For the full treatment, see the **Pact Developer Handbook**, chapter *Architecture & Trust Boundaries* — <https://github.com/PoC-Consortium/satchel/tree/master/docs/handbook-pact>. Related: [Security Model](Security-Model) · [Transports](Transports).
