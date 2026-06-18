# Satchel — desktop app

Tauri wrapper around the Pact UI, bundling and supervising `pactd`. The
"wallet-adjacent" experience without touching the Qt core wallet. No swap
logic lives here, ever — Satchel renders pactd's JSON-RPC and owns its
lifecycle; the daemon owns key material, chain logic and protocol choice.

## Frontend: React + Vite + TypeScript + MUI

`satchel/ui/` is a Vite React-TS app (MUI, light/dark/system theme). It is a
thin client of pactd: its only data paths are
`invoke('pactd_rpc', { method, params })` (proxied through Satchel's Rust
side, which holds the cookie) — including the merchant RPCs
(`createmerchant` / `listmerchants` / `loadmerchant` / `getmerchantinfo`),
which pactd owns — plus a few Tauri commands for the machine-level config
Satchel itself keeps in `satchel.json`: coins (`save_coin` / `remove_coin` /
`list_coin_config`), board (`save_board`), and UI prefs (`get_ui_prefs` /
`set_ui_prefs`). The build output is `ui/dist`, which `tauri.conf.json`
points `frontendDist` at.

Screens (left Drawer nav, grouped): a **Public** group — **Corkboard**
(offers filtered to your supported pairs, take/withdraw) and **Post offer**
(post a public listing); a **Private** group for off-market bilateral slips
— **Create**, **Take**, and **Slips** (review/cancel); then **Swaps** (live
swap table + narration + scheduler tick) and **Wallets** (per-coin
balance/receive/send). **Settings** sits in the footer and holds the
**Coins** tab (per-coin node setup with genesis validation + derived pairs)
alongside appearance, network, and board config. The first-run wizard +
merchant manager handle seed create/import (mnemonic shown once), the
encryption choice, and per-session unlock — Satchel never persists a
passphrase or seed.

### Toolchain

You need:

- **Node ≥ 18 + npm** (built with Node 24 / npm 11) — for the Vite frontend.
- **Rust** + the **Tauri CLI** — `cargo install tauri-cli --version "^2"`
  gives the `cargo tauri` subcommand used for the HMR dev loop and bundling.
- A WebView2 runtime (ships with current Windows).

### Develop (hot reload)

```sh
cd satchel
cargo tauri dev
```

`tauri.conf.json`'s `beforeDevCommand` runs `npm run dev` (Tauri runs the
hook from the frontend dir `ui/`, so no `--prefix` — Vite dev server on
`http://127.0.0.1:5173`, the configured `devUrl`), and the Tauri window
loads it with HMR — edit anything under `ui/src/` and the window updates
live. First run will `npm install` if you haven't:

```sh
cd satchel/ui && npm install
```

### Build

```sh
# Production frontend bundle (also run automatically by `cargo tauri build`
# via beforeBuildCommand):
cd satchel/ui && npm install && npm run build   # → ui/dist

# Then the desktop binary:
cd satchel && cargo build            # plain executable (bundling off)
# or: cargo tauri build              # full bundle (runs the npm build first)
```

Note: `tauri::generate_context!()` embeds `frontendDist` (`ui/dist`) at
**compile time**, so a plain `cargo build` / `cargo run` needs `ui/dist`
to already exist — run `npm run build` first, or use `cargo tauri dev`
(which serves Vite directly and doesn't need the dist).

### Verify (no headless UI tests)

The UI isn't unit-testable; the pactd RPC surface it calls is covered by
`pact`'s `cargo test` + the e2e harness. For a manual click-through, run
`pact/harness/satchel_playground.py` (regtest nodes + Corkboard + a
headless counterparty "Bob"), pre-seed `%APPDATA%/org.pocx.satchel/satchel.json`
with the regtest connection details, launch Satchel (`cargo tauri dev`),
then: wizard → create a merchant → Settings ▸ Coins shows both chains
connected → take an offer on the Corkboard → watch it complete on Swaps.

## Build notes

- Dev server is pinned to IPv4 `127.0.0.1:5173` on **both** sides
  (`vite.config.ts` `server.host` + `tauri.conf.json` `devUrl`). Vite's
  default `localhost` binds IPv6 `::1` only on Windows while Tauri's
  health-probe hits IPv4 — the mismatch hangs `cargo tauri dev` at
  "Waiting for your frontend dev server". Keep them in lockstep.
- `Cargo.lock` pins `time 0.3.43` / `serde_with 3.12` / `plist 1.8`:
  `cookie 0.18` (a hard tauri/wry dependency) has a trait-coherence
  conflict with `time >= 0.3.44`. Re-check when bumping tauri.
- `icons/icon.ico` is a generated placeholder; bundling is off
  (`cargo build` produces a plain executable).
- `ui/node_modules` and `ui/dist` are git-ignored build artifacts.

## Doubles as a light BTC wallet

pactd already derives BTC keys, watches the BTC chain, and signs BTC
transactions to do swaps — Satchel adds a balance/receive/send tab on top.
A user can complete a Bitcoin PoCX↔BTC trade and hold/spend the BTC with no
other software (AtomicDEX product shape).

Guardrails:

- Spending wallet, not a vault — keys are on the hot Pact seed; nudge users
  to sweep sizable balances to cold storage
- Basic P2WPKH/P2TR only; no coin control, no Lightning
- Chain backend selectable: public Electrum servers (zero-setup default) or
  the user's own bitcoind
- With Bitcoin PoCX Electrum servers live, Satchel also serves the BTC-only
  newcomer buying Bitcoin PoCX with zero pre-existing PoCX infrastructure
