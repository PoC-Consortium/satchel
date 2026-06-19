# Satchel & the multi-chain model

Satchel is the desktop app a normal person installs to trade: a first-run seed
wizard, multi-coin setup, capability-derived trading pairs, a legible
noticeboard, and a simple per-coin wallet. It is a Tauri shell wrapping a
React + Vite + TypeScript + MUI frontend (`satchel/ui/`) over a thin Rust
bridge (`satchel/src/main.rs`). All swap logic lives in pactd; Satchel only
renders it and drives it over JSON-RPC.

The load-bearing decision underneath Satchel is that the engine is
**coin-agnostic**: chains are data, not a hardcoded enum, and trading pairs are
derived from capabilities rather than a curated list. Both the backend and the
frontend render that model.

## Principles (load-bearing — do not erode)

- **pactd owns key material, chain logic, and protocol selection. Satchel
  renders and drives it over JSON-RPC. No swap logic in the UI, ever.**
- **One Pact seed = a hot *trading identity* (a "merchant").** Main funds stay
  in the user's core wallet. The funding wallet is pluggable (core-RPC now;
  Ledger/PSBT later); the HTLC transit keys are *always* the hot seed
  (auto-refund must sign unattended — it cannot live on hardware).
- **Chains are data, not code. Pairs are derived from capabilities, not a
  curated list.**
- **Shipped chain definitions are trusted; user-added coins are later work and
  must be validated against the connected node** (a wrong genesis/HRP
  misdirects funds).
- **Satchel is stateless about secrets — no OS keystore, ever.** It never
  persists a passphrase or seed. An encrypted merchant is unlocked per session
  (passphrase held in memory for that run, forgotten on exit); the mnemonic is
  shown once at creation and Satchel keeps no recovery copy.

## The chain model

Two layers — a shipped, trusted definition versus the user's runtime config.

### ChainDef (shipped, trusted)

`libswap/src/registry.rs` carries each coin as data (`ChainDef`): a stable
string `id` that drives RPC routing, the wire `asset` field, and the BIP32
coin-type; per-network params (magic, address prefixes, bech32 HRP, genesis
hash); and a `Capabilities` set (`cltv`, `segwit_v0`, `taproot`). The shipped
`REGISTRY` is exactly two coins, both in-code and trusted:

- **Bitcoin PoCX** (`btcx`, symbol `BTCX`, coin-type 1347371864 = `0x504F4358` "POCX") — Taproot ALWAYS_ACTIVE
  from genesis, so all three capabilities are true.
- **Bitcoin** (`btc`) — `cltv`, `segwit_v0`, `taproot` all true.

### ChainConfig (user runtime — what "setting up a coin" produces)

Setting up a coin produces a per-coin connection entry: the `coin_id`, the
`chain_data` backend URL(s) (own node over RPC and/or an Electrum/Esplora
endpoint, assembled into a `MultiBackend`), and a `funding_wallet` kind
(`core-rpc` today; `ledger` / watch-only PSBT later). The network
(`mainnet` / `testnet` / `regtest`) is global per Satchel instance, not
per coin.

### Pair resolver (derived, not curated)

For each unordered pair of *configured* coins on the same network, the
available swap protocols are the intersection of their capabilities
(`registry::protocols_for`):

- **classic HTLC** (`pact-htlc-v1`) needs `cltv && segwit_v0` on both
  (≈ every Bitcoin-like UTXO coin);
- **adaptor / MuSig2** (`pact-htlc-v2`, see [V2_ADAPTOR_SWAPS.md](V2_ADAPTOR_SWAPS.md))
  needs `taproot` on both.

A pair is offered if at least one protocol is supported. The capability rule
is pure — a taproot pair reports `Adaptor` as available even on networks where
the v2 engine path is gated; `select_protocol` applies the build/mainnet gates
(`ADAPTOR_MAINNET_ENABLED` stays `false` until the v2 audit signs off).
"Coin-agnostic" means Bitcoin-like UTXO chains; non-UTXO chains (ETH/Monero)
are out of scope — different mechanics, not a config entry.

## Backend: the engine and pactd

- **Chain registry.** The engine is keyed on a string `coin_id` into
  `REGISTRY`, not a hardcoded asset enum. `ChainRef`, swap params, wire
  messages, the store, and key derivation all route through the registry; the
  BIP32 coin-type comes from `registry::bip32_coin_type`.
- **Per-coin backends.** pactd builds a `MultiBackend` per configured coin from
  the coins config passed at launch (one entry per coin → its `chain_data`).
- **Seed lifecycle RPCs.** `walletstatus` reports `{seed_exists, encrypted,
  locked}`; `createseed {passphrase?}` returns the mnemonic *once* and encrypts
  at rest if a passphrase is given (BIP39 + scrypt/ChaCha20-Poly1305,
  `PACTSEEDv1`), otherwise stores it plaintext; `importseed {mnemonic,
  passphrase?}` imports an existing seed and echoes the derived identity;
  `unlock {passphrase}` opens an encrypted seed for the session. Both encrypted
  and unencrypted modes are supported, like Bitcoin Core. The engine only
  *warns* on a plaintext seed off mainnet; the mainnet plaintext block stays
  (a separate audit gate). An unencrypted seed keeps unattended auto-refund
  working across reboots; an encrypted seed pauses auto-refund after a restart
  until re-unlocked.
- **Merchants owned by pactd.** A *merchant* is one seed = one trading identity
  = one data dir, the Bitcoin-Core-wallet analog. The registry lives in pactd
  (`pactd/src/merchants.rs`): one pactd is launched at a **parent** data dir
  and owns a `merchants/` subdir plus a `merchants.json` manifest (each
  merchant's non-secret metadata — `id`, `label`, `identity` pubkey, `created`,
  `encrypted` — and which is `active`). The RPC surface is
  `createmerchant` / `listmerchants` / `loadmerchant` / `unloadmerchant` /
  `getmerchantinfo {id?}`; switching is in-process (no relaunch). The surface
  is deliberately merchant-scoped so a later phase can load several merchants
  concurrently without an API break. A **flat mode** is preserved for the e2e
  harness and `pact-cli`: if the data dir itself holds a seed (or
  `--auto-init` created one) it *is* a single synthetic `default` merchant.
- **Coin / pair RPCs.** `listcoins` returns the shipped registry plus, per coin,
  whether it is configured and a live connection probe (`ok` /
  `unconfigured` / `error: <reason>` + tip height). `listpairs` returns the
  derived pair availability for the current setup. `validatecoin {coin_id,
  chain_data}` builds an ephemeral backend and genesis-checks a *proposed*
  node before Satchel saves it (engine config is untouched). Wallet methods
  (`getbalance` / `getnewaddress` / `sendtoaddress`) take any `coin_id`.

The full RPC contract and the backend/frontend work split are in
[SATCHEL_BACKEND.md](SATCHEL_BACKEND.md).

## Satchel: the desktop app

Satchel is a thin client. Its only data paths are
`invoke('pactd_rpc', {method, params})` and a small set of daemon-level
config commands. `satchel/src/main.rs` owns:

- **NodeManager** — launches/adopts/stops a managed pactd, reads its cookie,
  applies config changes by relaunching as needed.
- **Machine-level config in `satchel.json`** — the per-coin connections
  (`coins`), the network, the noticeboard URL(s) (`board_urls`), and UI
  preferences (`theme` / `language` / `nav_open`). Commands: `list_coin_config`,
  `save_coin`, `remove_coin`, `save_board`, `get_ui_prefs`, `set_ui_prefs`.
  Node connections are machine-level (shared across merchants), so switching
  merchant never re-enters node setup.

The merchant registry is **not** in `satchel.json` — it moved into pactd. The
UI drives merchants directly through the pactd merchant RPCs.

The frontend (`satchel/ui/`, React + Vite + TS + MUI, dark theme, left-drawer
nav: Corkboard / Swaps / Wallets / Settings) covers:

- **First-run wizard + merchant manager** — create or import a merchant, show
  the mnemonic once with a backup acknowledgement, choose encrypted
  (passphrase) or unencrypted, select/switch merchants, and an unlock dialog
  for a locked encrypted merchant on boot.
- **Corkboard** — offers filtered to the pairs the current setup supports, with
  amounts, implied rate, timelocks, and age; post / take / withdraw. The board
  is a noticeboard, not an exchange: no matching, execution, custody, fees, or
  accounts.
- **Swaps** — the live swap table with plain-language narration per
  (role, state) and state-gated fund / redeem / refund / abort / cancel
  actions; v2 (Taproot adaptor) swaps surface here too.
- **Coins** — coin setup that validates against the node (genesis check) before
  saving, with a per-coin connection-status glyph.
- **Wallets** — per-configured-coin **read-only** balances (one card per coin
  with a hot-seed sweep nudge). Send/receive are deliberately out of the
  node-backed app — the balance is the node's own core wallet — and arrive
  only with the nodeless (bdk + Electrum) build.

The detailed screen catalog and first-run polish requirements are in
[SATCHEL_UI.md](SATCHEL_UI.md).

## Out of scope here

Ledger / PSBT funding wallets, user-added (non-shipped) coins, and the Nostr
board transport are future work. Related designs:
[V2_ADAPTOR_SWAPS.md](V2_ADAPTOR_SWAPS.md) (Taproot/MuSig2 adaptor swaps) and
[PRIVATE_OFFERS.md](PRIVATE_OFFERS.md) (off-market offers). For phasing and the
regulatory position see [TRADING_ROADMAP.md](TRADING_ROADMAP.md); for the
overall design see [ARCHITECTURE.md](ARCHITECTURE.md), the protocol spec under
[../spec/](../spec/), and the component map in [../README.md](../README.md).
