# PoCX Trading — Architecture

Trustless P2P trading for PoCX via atomic swaps: a swap engine plus a dumb
noticeboard. No exchange, no custody, no fees, no matching engine.

Companion docs: [TRADING_ROADMAP.md](TRADING_ROADMAP.md) (phases, regulatory
strategy), [SATCHEL.md](SATCHEL.md) / [SATCHEL_UI.md](SATCHEL_UI.md) /
[SATCHEL_BACKEND.md](SATCHEL_BACKEND.md) (the desktop app),
[V2_ADAPTOR_SWAPS.md](V2_ADAPTOR_SWAPS.md) (Taproot/MuSig2 swaps),
[PRIVATE_OFFERS.md](PRIVATE_OFFERS.md) (off-market offers). Protocol spec:
[../spec/](../spec/). Component map: [../README.md](../README.md).

## Hard constraints

1. **Zero changes to bitcoin-pocx core.** The engine talks to a stock node
   exclusively via RPC + ZMQ (or the Electrum protocol). Anything the node
   can't do over RPC, the engine does itself — never the other way around.
2. **Keys never leave the user's machine.** Hosted components see signed
   offers and encrypted coordination messages only.
3. **One engine, many front-ends.** Swap logic exists exactly once, in the
   `libswap` crate behind `pactd`. Front-ends carry no swap logic.
4. **Non-custodial everywhere.** Pact's own BIP39 seed holds only hot transit
   keys for in-flight swaps; proceeds sweep to the user's core wallet.
5. **The noticeboard never matches, executes, custodies, or charges.** Humans
   pick offers.

## Bitcoin-Core shape

Pact mirrors Bitcoin Core's daemon/CLI/GUI split, and its auth and command
surface follow Core's conventions.

| Bitcoin Core | Pact | Role |
|---|---|---|
| `bitcoind` | **pactd** | daemon: JSON-RPC, holds keys + state, runs the refund scheduler |
| `bitcoin-cli` | **pact-cli** | thin JSON-RPC client; no embedded engine |
| `bitcoin-qt` | **Satchel** | Tauri GUI that manages the daemon and speaks JSON-RPC |
| `.cookie` | **`.cookie`** | random `__cookie__:<hex>`, HTTP Basic auth |
| ZMQ | (polling) | live updates by polling `listswaps`/`getinfo`; the GUI polls, no push channel |

## The stack

```
┌─────────────────────────────────────────────────────────────┐
│  FRONT-ENDS (thin clients, no swap logic)                    │
│                                                              │
│  pact-cli            Satchel              Discord bot        │
│  (scripting,         (Tauri desktop app,  (board front-end   │
│   power users)        light BTC wallet)    only, no keys)    │
└───────┬──────────────────┬─────────────────────┼────────────┘
        │   JSON-RPC 2.0 over HTTP (loopback,     │ offers /
        │   HTTP Basic via .cookie / pact.conf)   │ notify only
┌───────┴──────────────────┴───────┐             │
│  TRADING CORE — pactd ("Pact")    │             │
│                                   │             ▼
│  • HTLC construction & verify     │   ┌──────────────────┐
│  • Per-swap state machine         │   │  CORKBOARD       │
│    (offer→funded→redeemed/refund) ◄───┤  (hosted, dumb)  │
│  • Secret / preimage management   │   │                  │
│  • Automatic refund scheduling    │   │  • signed offers │
│  • Chain monitoring               │   │  • blind E2E msg │
│  • SQLite persistence             │   │    relay         │
│  • Coin-agnostic chain registry   │   │  • NO matching   │
│  • Encrypted msg relay client     │   │  • NO keys/funds │
└───────┬───────────────────────────┘   │  • NO fees       │
        │ per-coin backend(s)            └──────────────────┘
┌───────┴────────────────────────────────┐
│  Chain data backends (user's choice):   │
│  Core RPC (own pocx node / bitcoind) or │
│  Electrum server (electrs-pocx / BTC)   │
└─────────────────────────────────────────┘
```

## Components

### Trading core: `pactd` + `pact-cli` (component name: Pact)

A cargo workspace, built **library-first**: the `libswap` crate holds the
engine, `pactd` wraps it in a JSON-RPC daemon, `pact-cli` is the first
client.

- **Language: Rust** project-wide. `rust-bitcoin` + `rust-miniscript` for
  script/tx construction, `bdk` for wallet machinery, `electrum-client` for
  Electrum chain data, `tokio` for the daemon. References: COMIT's
  `xmr-btc-swap` (production swap engine), decred's Go `atomicswap`
  (protocol logic). Compiles to single static binaries for Win/Linux/Mac.
- `pactd` owns the full swap lifecycle: build/verify HTLCs, watch both
  chains, redeem, and **schedule refunds automatically** — it broadcasts the
  refund tx after timeout even if the GUI is closed (it can run as a
  service). State lives in a local SQLite file; everything is recoverable
  from the seed plus the state file.
- `pactd` binds loopback only (`127.0.0.1:9737` by default) — there is no
  auth model for cross-network exposure.

**`pact-cli` is a thin JSON-RPC client**, like `bitcoin-cli`: it holds no
swap logic and no engine. `pact-cli call <method> [params...]` is a generic
passthrough that prints JSON; the structured subcommands
(`offer`/`accept`/`recv`/`fund`/`redeem`/`refund`/`abort`/`status`/`board`)
wrap an RPC plus the file I/O of the manual handshake. Seed creation lives
daemon-side (`createseed`/`importseed`/`unlock` RPCs, or `pactd --auto-init`);
the CLI never touches the seed or SQLite.

#### JSON-RPC + cookie auth

The one command protocol is **JSON-RPC 2.0 over HTTP POST**; errors use RPC
error objects (code + message). Auth mirrors bitcoind exactly:

- On startup pactd writes `<datadir>/.cookie` containing `__cookie__:<hex>`,
  used as HTTP Basic credentials. Clients that can read the filesystem (the
  CLI, Satchel) authenticate from it; a browser cannot.
- Static `rpcuser`/`rpcpassword` from `<datadir>/pact.conf` are also accepted
  alongside the cookie (remote/managed setups), like Core.

**Method surface** (chain-neutral by design — `getbalance <chain>`, never
`getpocxbalance`):

| Group | Methods |
|---|---|
| Node | `getinfo` (name, version, protocol, network, identity, seed/lock status, configured coins), `stop` |
| Seed / wallet lifecycle | `walletstatus`, `createseed`, `importseed`, `unlock` |
| Merchants (pactd-owned) | `createmerchant`, `listmerchants`, `loadmerchant`, `unloadmerchant`, `getmerchantinfo` |
| Coins / pairs | `listcoins`, `listpairs`, `validatecoin` |
| Swaps (v1 HTLC) | `listswaps`, `getswap`, `offer`, `acceptoffer`, `recv`, `fund`, `redeem`, `refund`, `abort`, `tick` |
| Swaps (v2 adaptor) | `listadaptorswaps` |
| Corkboard | `boardlistoffers`, `boardpostoffer`, `boardtake`, `boardrevoke` |
| Private offers | `makeprivateoffer`, `takeoffer`, `listprivateoffers`, `cancelprivateoffer` |
| Fees | `estimateswapfees` (platform fee is always 0) |
| Wallet | `getbalance <chain>`, `getnewaddress <chain>`, `sendtoaddress <chain> <address> <amount>` |

`getinfo` reports `protocol` from `libswap::PROTOCOL_VERSION` (re-exported
from the `pact-proto` crate).

#### Coin-agnostic chain registry

Chains are **data, not a hardcoded enum**. The old `Asset` enum is gone: the
registry (`libswap/src/registry.rs`) is a table of `ChainDef`s, each keyed by
a stable lowercase string `coin_id` (`"btcx"`, `"btc"`) that drives RPC
routing, the wire `asset` field, and the BIP32 coin-type. A `ChainDef`
carries display metadata, per-network `ChainParams` (magic, prefixes, HRP,
genesis hash, spacing), the BIP32 coin-type, and **capability flags**
(`cltv`, `segwit_v0`, `taproot`).

- The shipped registry is exactly two coins (POCX, BTC), in-code and trusted;
  both have full UTXO + Taproot capabilities.
- The **pair resolver** derives which protocols a pair can run from the
  *intersection* of the two coins' capabilities — there is no curated pair
  list. Classic HTLC needs `cltv && segwit_v0` on both; adaptor needs
  `taproot` on both. `derive_pairs` folds in "both legs configured with a
  backend" and "is the protocol built" to report what is tradable now.

> **TODO:** user-added coins. The registry is shipped/trusted only; coins
> validated against a connected node (`validatecoin` proves the genesis check
> works for a proposed backend) are designed but not yet user-addable.

#### Chain-data backends

Each coin's chain data comes from one or more configured backends, supplied
at launch as `--coin <coin_id>=<url[,url]>` (repeatable; legacy
`--pocx-rpc`/`--btc-rpc` aliases remain for the harness). A `MultiBackend`
fans across the URL list; the URL scheme selects the implementation:

- `http://…` → **Core RPC** backend — the user's own pocx node / bitcoind. A
  wallet-qualified URL points at the user's core wallet on that node.
- `tcp://…` / `ssl://…` → **Electrum** backend — used for either chain
  symmetrically, so a user needs no foreign-chain full node.

Both backends are implemented behind one `ChainBackend` trait. All backend
data is an untrusted hint: scripts and amounts are verified against locally
reconstructed bytes, and refund scheduling is purely clock-driven. A lying
backend can withhold or delay, never steal. Every backend must pass a genesis
-hash `verify_chain` check before any funding decision.

PoCX has its own Electrum server,
[electrs-pocx](https://github.com/PoC-Consortium/electrs-pocx)
(Blockstream-electrs lineage), which powers the block explorer and serves the
Electrum RPC protocol alongside an Esplora REST API. This lets pactd use one
chain-data protocol (Electrum) for both chains, and lets a BTC holder buy
PoCX with zero pre-existing PoCX infrastructure.

### Front-ends

All front-ends are thin JSON-RPC clients of pactd; none carries swap logic.

| Front-end | What it is | Notes |
|-----------|-----------|-------|
| `pact-cli` | ships with the daemon | scripting, OTC power users |
| Satchel | Tauri desktop app | manages pactd + speaks JSON-RPC; doubles as a light BTC wallet |
| Discord bot (Crier) | board front-end only | browse/post offers, notifications, deep-link to the local app; never does key operations |

A native Qt-wallet integration is explicitly out — it would mean changes to
core. Satchel fills that role.

**Satchel** owns pactd's lifecycle (the phoenix-pocx `NodeManager` pattern):
it spawns a managed pactd, or adopts one already listening, or attaches to an
external one (**Managed vs External** is a first-class setting). It reads the
`.cookie` for auth and proxies every UI call through to pactd; it never asks
the user to paste a token. The GUI assets live in Satchel — pactd serves no
HTML. Merchant ownership (one merchant = one identity = one data dir) lives
in **pactd**, the Bitcoin-Core-wallet analog; Satchel launches a single pactd
and selects the active merchant over RPC.

**Satchel doubles as a light BTC wallet.** pactd already derives BTC keys,
watches the BTC chain, and signs BTC transactions for swaps, so a
balance/receive/send tab is a thin layer on top. Guardrails: a spending
wallet, not a vault (the UI nudges users to sweep sizable balances to cold
storage); basic P2WPKH/P2TR only — no coin control, no Lightning. Chain
backend is the user's choice (own node or public Electrum servers).

> **TODO:** an Electrum plugin (`pact-electrum/`, Python — Electrum's plugin
> API is Python) that talks to the local pactd API, giving Electrum-PoCX
> users swap capability. Designed, not built.

### Corkboard (the noticeboard — hosted, deliberately dumb)

A single Rust binary (axum + SQLite/Postgres), easy for anyone to self-host;
multiple independent operators is the goal (Bisq model). It depends only on
the wire format, not the engine. It:

- stores **signed offers** (`signmessage` from a funded address → listings
  can't be faked and are provably funded);
- provides a **blind end-to-end-encrypted message relay**: counterparties
  exchange swap-coordination blobs signed + encrypted client-side, so the
  transport is swappable (a Nostr-relay transport is planned as an
  alternative — see the roadmap);
- does **NOT** match, execute, hold keys or funds, charge fees, or require
  accounts. It is a noticeboard; humans pick offers. This is load-bearing for
  the MiCA position and must not become a matching engine.

## Swap protocols (versioned + capability-driven)

The protocol stays versioned and capability-driven rather than replacing one
mechanism with another:

- **`pact-htlc-v1`** — classic CLTV hash-timelock swap (P2WSH + CLTV, shared
  hash `H` on both legs). The universal fallback: works on any UTXO chain
  with CLTV + segwit. This is the default whenever both legs support it.
- **`pact-htlc-v2`** — Taproot/MuSig2 adaptor swaps (private, look like
  ordinary payments). Selected per swap only when a pair lacks a classic-HTLC
  option, gated on the `taproot` capability of both legs. The v2 engine is
  built (`libswap` `musig`/`taproot`/`adaptor_swap`); on **mainnet** it stays
  disabled until the crypto audit signs off (see V2_ADAPTOR_SWAPS.md).
  Regtest and testnet run v2 freely. See V2_ADAPTOR_SWAPS.md for the design.

## Key infrastructure

The swap system adds **one new HD seed, owned by pactd**, separate from the
user's core wallet.

**What one swap needs:** a funding input (a normal send from the core
wallet), a refund key (home chain), a redeem key (foreign chain), the secret
preimage, and an identity key for signing offers.

**pactd's BIP39 seed derives, by BIP32 path** (spec §4;
`libswap/src/keys.rs`, purpose `7228'` = "PACT" on a phone keypad):

| Material | Path |
|---|---|
| Identity key (BIP340 Schnorr) | `m/7228'/0'/0'` |
| Swap key (chain c, swap i) — v1 ECDSA redeem/refund and v2 MuSig2 signer | `m/7228'/1'/coin(c)'/i'` |
| Preimage / adaptor-secret source (swap i) | `m/7228'/2'/i'` |
| v2 refund key (chain c, swap i) — single-key CLTV refund tapleaf | `m/7228'/3'/coin(c)'/i'` |

- `coin(c)` is the chain's BIP32 coin-type from the registry (SLIP-44 where
  it exists: BTC = 0, PoCX = 20559 = `0x504F` "PO"). Per-swap keys mean users
  need no foreign-chain *wallet*, only chain data.
- The identity key is stable across swaps and is **not** the proof-of-funds
  key.
- The v1 preimage is `TaggedHash("pact/htlc/preimage/v1", key at m/7228'/2'/i')`;
  the v2 adaptor secret `t` is `TaggedHash("pact/adaptor/secret/v2", same key)`
  as a valid secp256k1 scalar, with adaptor point `T = t·G`. Both are
  re-derivable from the seed alone, so losing the SQLite state never loses the
  secret — seed + chain scan recovers even in-flight swaps.

**The core wallet keeps a boring role:** it funds HTLCs with a normal send,
receives sweeps at fresh addresses, and signs one `signmessage` per offer as
proof of funds (requested by pactd over RPC). The main stash never leaves
core.

**Hot-wallet reality:** the pactd seed must be hot — automatic
refund-after-timeout means signing with no human present (the Lightning-node
model). The mitigation is bounded exposure, not cold storage: swap keys are
transit keys, swept to core immediately on redeem/refund, with near-zero
balance between swaps; per-trade caps bound in-flight value.

**User backup story:** the existing core-wallet backup (unchanged) plus one
new seed phrase. The SQLite state file is a convenience, not a single point
of loss.

| Key material | Lives in | Hot/cold | Exposure if stolen |
|--------------|----------|----------|--------------------|
| Main funds | core wallet | user's choice | unchanged from today |
| Swap refund/redeem keys | pactd seed | hot | in-flight swaps only |
| Preimages / adaptor secrets | derived from pactd seed | hot | in-flight swaps only |
| Identity key | pactd seed | hot | identity, not funds |

## Trust boundaries

| Component | Sees keys? | Sees funds? | Hosted by whom |
|-----------|-----------|-------------|----------------|
| pocx node, bitcoind, Electrum server | the node owner's wallet | yes (own node) | user / community |
| pactd + front-ends | yes (swap keys/secrets) | builds txs locally | user |
| Corkboard | no | no | community, multiple operators |
| Discord bot (Crier) | no | no | community |

## Workspace boundaries and the intended repo split

The codebase is a cargo workspace with four boundaries that double as the
seams of an eventual multi-repo split:

- **`pact-proto`** — the wire format + crypto, no engine/chain/daemon:
  `Envelope`, canonical JSON, BIP340 sign/verify, relay `seal`/`open` (ECDH),
  `OfferBody`/receipt bodies, `swap_id`, tagged hashes, `PROTOCOL_VERSION`.
  Carries **no chain identity** — the wire `Envelope.body` is opaque JSON.
  Both the engine and the board depend on it symmetrically; it is what a third
  party reads to build an independent implementation. Natural home of the
  spec.
- **`pact`** — `libswap` (logic) + `pactd` (daemon) + `pact-cli` (client),
  plus the integration harness and playground. The standalone OSS swap
  tooling.
- **`corkboard`** — the axum + SQLite noticeboard; depends on `pact-proto`
  only.
- **`satchel`** — the Tauri shell + GUI + pactd `NodeManager`; a pure
  JSON-RPC client.

> **TODO:** repo split — designed, still a monorepo. The plan splits these
> four workspaces into four independently releasable repos (workspace = repo
> = release unit) at go-live: `pact-proto` → `pact-protocol`, plus `pact`,
> `corkboard`, `satchel`. Because the workspace boundaries already match the
> future repo seams, the split is a near-pure lift (path-deps → pinned
> git-deps, plus the harness locating `corkboard` as a sibling). It also
> reinforces the MiCA posture: the engine ships as standalone tooling and the
> board as a separate self-hostable project, with no single product bundling
> them.
