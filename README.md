# Satchel

Trustless, peer-to-peer trading for cryptocurrencies via atomic swaps. No
exchanges, no custody, no fees, no matching engine — a protocol, relays, and a
desktop app. Keys never leave your machine; what's hosted sees only signed
offers and encrypted blobs.

Swap directly with a counterparty: the chain enforces the deal, so neither side
can cheat and no third party ever holds your coins. The first supported pair is
**BTCX ↔ BTC**; more coins follow.

- Handbooks: [docs/handbook-pact/](docs/handbook-pact/) (developers & integrators) and [docs/handbook-satchel/](docs/handbook-satchel/) (users)
- Plan: [docs/TRADING_ROADMAP.md](docs/TRADING_ROADMAP.md)
- Protocol spec: [spec/](spec/)

> **Status:** live. Both swap protocols — v1 (hash-locked HTLC) and v2
> (Taproot/MuSig2 adaptor) — are reviewed, audited, and running on mainnet. v1
> is the default; v2 is selected automatically for Taproot-only pairs.

## How it works

Three moving parts, with a hard wall between them:

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

1. **Pact** (the engine) runs locally, holds your keys, builds and watches the
   swap transactions, and auto-refunds if a counterparty walks away.
2. A **transport** carries identity-signed offers and forwards encrypted
   coordination blobs. It never matches, executes, custodies, or charges, and
   operators see ciphertext only. Satchel speaks two, side by side:
   **Nostr relays** (the default — censorship-resistant, nothing to run) and
   **Corkboard** (a self-hostable noticeboard for communities that want their
   own).
3. **Satchel** (the desktop app) is the face — it renders the engine's RPC,
   manages seeds, and shows read-only per-coin balances (a full send/receive
   wallet arrives with the nodeless build).

Naming theme: the village market square. A **pact** is the trustless deal,
posted on the **corkboard**, settled into your **satchel**. Deliberately no
"exchange" / "DEX" branding.

## Components

| Folder | Name | What it is |
|--------|------|------------|
| [`spec/`](spec/) | — | Atomic-swap protocol spec (v1 HTLC + v2 adaptor) and deterministic test vectors, written so third parties can implement independently. |
| [`pact-proto/`](pact-proto/) | — | Wire format + crypto primitives: signed envelopes, canonical JSON, BIP340, recipient-sealed relay blobs (`PACTSEALED1`), private-offer slips. |
| [`pact-nostr/`](pact-nostr/) | — | Maps Pact envelopes ↔ Nostr events (public offer adverts kind `31510`, gift-wrapped relay messages kind `1059`). Pure mapping + crypto, no relay I/O. |
| [`pact/`](pact/) | **Pact** | The swap engine (Rust workspace): `libswap` (HTLC/adaptor logic + state machine), `pactd` (local JSON-RPC daemon, SQLite, auto-refund + RBF fee-bump scheduler), `pact-cli` (thin RPC client). |
| [`corkboard/`](corkboard/) | **Corkboard** | Self-hostable order board: a single axum + SQLite/Postgres binary that stores signed offers and blind-relays encrypted blobs. The alternative transport to Nostr (Bisq-style, many operators). |
| [`satchel/`](satchel/) | **Satchel** | Desktop app (Tauri shell + React/Vite/TypeScript/MUI frontend). Bundles and supervises `pactd`; shows read-only per-coin balances. Owns the GUI, never the swap logic. |
| [`tools/`](tools/) | — | Dev tooling (e.g. `relay-prober` for Nostr relay eligibility) and playground scripts. |
| [`docs/`](docs/) | — | The **Pact** and **Satchel** handbooks (Markdown → PDF), the GitHub **wiki** pages (staged for `satchel.wiki.git`), and the product roadmap. |

## Building

Everything is Rust (cargo); Satchel's frontend adds a Node/Vite layer.

### Prerequisites

- **Rust** (stable) + cargo.
- For Satchel: **Node ≥ 18 + npm** and the **Tauri CLI**
  (`cargo install tauri-cli --version "^2"`), plus a WebView2 runtime
  (ships with current Windows).
- For the end-to-end harness: **Python 3** and a `bitcoin-pocx` build for
  regtest.

### Engine — Pact

```sh
cd pact
cargo build && cargo test            # unit + protocol-vector tests (v1 + v2)
python harness/test_swap_e2e.py      # full BTCX↔BTC swap on regtest
python harness/test_adaptor_swap.py  # v2 adaptor-swap end to end
```

Run the daemon and drive it with the CLI:

```sh
cargo run -p pactd -- --coin btcx=<rpc-url> --coin btc=<rpc-or-electrum-url>
cargo run -p pact-cli -- getinfo
```

### Transport

The default transport is **Nostr** — no infrastructure to run; point Satchel at
relays and go. Communities that prefer their own noticeboard can self-host
**Corkboard**:

```sh
cd corkboard
cargo run -- --listen 127.0.0.1:9780 --db corkboard.sqlite
```

### Desktop app — Satchel

```sh
cd satchel
cargo tauri dev          # hot-reload dev loop (Vite on 127.0.0.1:5173)
```

For a production build:

```sh
cd satchel/ui && npm install && npm run build   # → ui/dist
cd ..        && cargo tauri build               # full bundle
#            or cargo build                       (plain executable, bundling off)
```

> `tauri::generate_context!()` embeds `ui/dist` at **compile time**, so a plain
> `cargo build`/`cargo run` needs `ui/dist` to exist first — run the frontend
> build, or use `cargo tauri dev` (which serves Vite directly). See
> [`satchel/README.md`](satchel/README.md) for build notes and a manual
> click-through.

### One-shot regtest playground

```sh
./tools/playground-cork.ps1    # regtest nodes + Corkboard + headless
                               # counterparties, then launches Satchel
./tools/playground-nostr.ps1   # same, but over a local Nostr relay (no board)
```

Each script builds and brings up the whole stack, then blocks on the Satchel
window — close it and everything is torn down automatically (`-Down` force-tears
a stale run; teardown is PID/port-only).

## Technology

Rust across all components (Tauri for Satchel, whose frontend is React + Vite +
TypeScript + MUI). Core engine crates: `rust-bitcoin`, `rust-miniscript`,
`bdk`, `electrum-client`, `musig2` (v2 adaptor swaps).

## License

See [LICENSE](LICENSE).
