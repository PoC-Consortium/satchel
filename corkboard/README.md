# Corkboard — order board

Hosted, deliberately dumb. A noticeboard, not an exchange — this is
load-bearing for the regulatory position. Single Rust binary (axum) +
SQLite/Postgres,
easy to self-host; multiple independent community operators is the goal
(Bisq model).

## Status: implemented (v1)

Single axum + SQLite binary (`cargo run -- --listen 127.0.0.1:9780 --db
corkboard.sqlite`). Exercised end to end by the harness: a maker and a
taker pactd complete a swap purely through the board (offer → take →
relay handshake → swap completes).

## Does

- Store **BIP340-signed offer envelopes** (identity-signed, TTL-bounded,
  signed revocation → listings can't be forged). Every write endpoint
  requires a valid envelope signature. Proof-of-funds is the *client's*
  job: the board never talks to any chain.
- **Blind relay** of swap coordination blobs: content-blind
  store-and-forward; polls are signed so only the identity owner reads
  their mail. Blobs are **E2E-encrypted client-side** (ephemeral-key ECDH
  against the recipient identity + ChaCha20-Poly1305, `PACTSEALED1`
  format) — an operator sees only ciphertext addressed to a pubkey; the
  harness verifies this against the board's own database. Transport
  swappable — Nostr relays as v2.

## HTTP surface

- `GET  /health`
- `POST /v1/offers` — submit a signed offer envelope
- `GET  /v1/offers` — list active offers (filters: `give`, `get`, `network`)
- `POST /v1/offers/revoke` — signed revocation (same identity)
- `POST /v1/relay` — `{to, blob}` store-and-forward, content-blind
- `POST /v1/relay/poll` — signed poll → sealed blobs since a cursor

## Does NOT (by design — do not add)

- Match orders, execute trades, hold keys or funds, charge fees, require
  accounts/KYC. Humans pick offers.

## Front-ends

Web UI (primary), Crier Discord bot (secondary). Offers are fixed-size, no
partial fills, small caps in v1.
