# Pact — swap engine

The trading core. Rust, cargo workspace, library-first. Implements `spec/`.
Core crates: `rust-bitcoin`, `rust-miniscript`, `bdk`, `electrum-client`,
`musig2` (v2 adaptor swaps).
References: COMIT `xmr-btc-swap` (Rust), decred `atomicswap` (protocol logic).

- `libswap` — HTLC construction/verification, per-swap state machine
  (offer → funded → redeemed/refunded), key + preimage derivation from the
  Pact BIP39 seed, chain monitoring. Two protocols: v1 hash-locked HTLCs
  and v2 Taproot/MuSig2 adaptor swaps (`pact-htlc-v2`, mainnet-gated)
- `pactd` — daemon: local JSON-RPC 2.0 over HTTP (bitcoind-shaped), SQLite
  persistence, automatic refund scheduling (runs as a service; signs
  refunds with no human present — hot by design, bounded exposure)
- `pact-cli` (binary `pact-cli`) — a thin JSON-RPC client for pactd:
  offer / accept / redeem / refund and a generic `call` passthrough; the
  first API client and the proof that the API is sufficient for every
  later front-end

## Key facts

- Own BIP39 seed: per-chain per-swap keys (PoCX, BTC, …), identity key,
  deterministic preimages. Seed + chain scan recovers in-flight swaps.
- Transit keys only — sweeps proceeds/refunds to the user's core wallet
  immediately. The main stash never leaves core.
- Chain backends: PoCX node via RPC/ZMQ (or PoCX Electrum); BTC via user's
  bitcoind or public Electrum servers. One Electrum client implementation,
  used symmetrically for both chains.
- One PoCX-specific consideration: network params (magic bytes, address
  prefixes, bech32 HRP) must be defined for `rust-bitcoin` — read them from
  `bitcoin-pocx`'s chainparams, never hardcode guesses.

## Layout & build

Cargo workspace: `libswap/` (library), `pactd/` (daemon), `pact-cli/`
(binary name `pact-cli`). `harness/` is a separate Python regtest harness
(not a workspace member) holding the end-to-end tests; see
`harness/README.md`. The relay-sealing and wire-format primitives live in
the repo-root `pact-proto` crate, which `libswap` depends on.

```sh
cargo build && cargo test          # unit + vector-regression tests (v1 + v2)
python harness/test_swap_e2e.py    # Phase 1 DoD test — GREEN since 2026-06-12
python harness/test_adaptor_swap.py # v2 adaptor-swap end-to-end test
```

Engine status (proven by the regtest e2e suite):

- Complete + refund paths end to end on regtest (Core-RPC backend), for
  both v1 HTLC swaps and v2 Taproot/MuSig2 adaptor swaps.
- **pactd is live**: a localhost **JSON-RPC 2.0 over HTTP** endpoint
  (POST `/`, bitcoind-shaped) over the same engine — methods include
  `getinfo`, `offer`, `acceptoffer`, `recv`, `fund`, `redeem`, `refund`,
  `abort`, `listswaps`/`getswap`, the `adaptor*` v2 lifecycle, the
  `board*`/private-offer methods, the merchant + seed-lifecycle methods,
  and `tick`. The scheduler auto-redeems when due, auto-refunds once
  MTP ≥ T, and **RBF fee-bumps** while a spend is unconfirmed
  (spec §7.4) — all with no human present. `pactd --once` runs a single
  pass (cron/tests).
- v2 adaptor swaps (`pact-htlc-v2`) run on regtest/testnet but are
  **refused on mainnet** until the adaptor security audit
  (`ADAPTOR_MAINNET_ENABLED`); v1 is the default everywhere else.
- Coins and merchants are config/registry-driven: a shipped string-id coin
  registry (`pocx`, `btc`, each with declared capabilities) plus a capability
  pair resolver — no hardcoded asset enum. pactd owns merchants
  (bitcoin-core-wallet-shaped: one data dir per merchant); chain backends
  come from repeatable `--coin <id>=<url[,url]>` launch flags.
- Refund transactions are signed at funding time and persisted
  (spec §6.3); fees come from `estimatesmartfee` with a fallback floor.
- **Multi-backend monitoring** (spec §10): RPC URLs are comma-separated
  lists; funded-output verification demands agreement across backends,
  spend-search takes the first positive answer, clocks/fees take the
  conservative value. Wallet ops go to the primary (first) URL.
- Seed encryption: a passphrase (set `PACT_PASSPHRASE`, or pass one to the
  `createseed`/`importseed` RPCs) encrypts the seed with
  scrypt + ChaCha20-Poly1305 (`PACTSEEDv1`); without one the seed is stored
  unencrypted. `unlock` decrypts an encrypted seed for the session.
- **Network policy**: regtest free; `--network testnet` allowed (spec §7.3
  timelock/confirmation minimums enforced — an unencrypted testnet seed is
  warned, not refused); mainnet refused pending external review.

- **Electrum backend**: `tcp://host:port` / `ssl://host:port` URLs in the
  comma-separated backend lists select the Electrum protocol — works
  against any BTC Electrum server and against
  [`electrs-pocx`](https://github.com/PoC-Consortium/electrs-pocx) (the
  explorer's indexer, which serves Electrum RPC alongside Esplora REST).
  Chain-data only: the primary backend stays a Core-RPC wallet URL. PoCX's
  286-byte signed headers are handled on raw bytes (hash excludes the
  generator signature) — pinned by unit vectors captured from a real node.
- **API auth**: pactd authenticates like bitcoind — HTTP Basic against an
  auto-generated per-run `<data_dir>/.cookie` (the zero-config local
  default) and/or `rpcuser`/`rpcpassword` from `<data_dir>/pact.conf`,
  required on everything except `/health`. The endpoint binds loopback
  only. Clients (`pact-cli`, Satchel) read the cookie from disk.

- **Relay privacy**: coordination envelopes are sealed to the recipient
  identity (ephemeral ECDH + ChaCha20-Poly1305) before they touch any
  board — operators see ciphertext only.
- **Wallet API**: `getbalance` / `getnewaddress` / `sendtoaddress` per
  configured coin (passthrough to the user's core-wallet backend, with a
  wrong-chain address guard). The pactd-seed light wallet for Electrum-only
  users is future bdk work.

Still open before real-money networks: a live-fire e2e against a running
electrs-pocx + BTC electrs pair (needs a Linux/CI host — RocksDB on
Windows is impractical), backup/restore tooling, Tor/SOCKS5 support for
board + Electrum connections, external audit (gates mainnet).

## Definition of done (Phase 1)

Two `pact-cli` clients complete and refund PoCX↔BTC swaps end to end on
regtest with a manual handshake — no Corkboard, no GUI.
