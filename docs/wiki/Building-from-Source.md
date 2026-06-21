# Building from Source

Everything is Rust (cargo); Satchel's frontend adds a Node/Vite layer. This page echoes the repo README's build section concisely. For deeper notes see the handbooks: **Pact** <https://github.com/PoC-Consortium/satchel/tree/master/docs/handbook-pact> and **Satchel** <https://github.com/PoC-Consortium/satchel/tree/master/docs/handbook-satchel>.

## Prerequisites

- **Rust** (stable) + cargo.
- For **Satchel**: **Node ≥ 18 + npm** and the **Tauri CLI** (`cargo install tauri-cli --version "^2"`), plus a WebView2 runtime (ships with current Windows).
- For the **end-to-end harness**: **Python 3** and a `bitcoin-pocx` regtest build.

## Engine — Pact

```sh
cd pact
cargo build && cargo test            # unit + protocol-vector tests (v1 + v2)
python harness/test_swap_e2e.py      # full BTCX↔BTC swap on regtest
python harness/test_adaptor_swap.py  # v2 adaptor swap end to end
```

Run the daemon and drive it with the CLI (see [Running pactd](Running-pactd) and [pact-cli](pact-cli)):

```sh
cargo run -p pactd -- --coin btcx=<rpc-url> --coin btc=<rpc-or-electrum-url>
cargo run -p pact-cli -- getinfo
```

## Transport — Corkboard

The default transport is Nostr (no infrastructure). To self-host a board:

```sh
cd corkboard
cargo run -- --listen 127.0.0.1:9780 --db corkboard.sqlite
```

See [Self-Hosting a Corkboard](Self-Hosting-Corkboard).

## Desktop app — Satchel

```sh
cd satchel
cargo tauri dev          # hot-reload dev loop (Vite on 127.0.0.1:5173)
```

Production bundle:

```sh
cd satchel/ui && npm install && npm run build   # → ui/dist
cd ..        && cargo tauri build               # full bundle
#            or cargo build                       (plain executable, bundling off)
```

> **Warning — `ui/dist` embedding gotcha.** Tauri embeds `ui/dist` into the Rust binary at **compile time**. `cargo tauri build` re-runs the frontend build for you, but a plain `cargo build` / `cargo run` does **not** — it will ship a stale (or missing) UI unless you run `npm run build` first. Use `cargo tauri dev`, which serves Vite directly.

> **Note — sidecar staging.** The Tauri config lists `pactd` and `pact-cli` as `externalBin` sidecars, so a dev/build run refuses to start unless `satchel/binaries/<name>-<host-triple>.exe` exist. The playground scripts stage these by copying fresh debug binaries.

> **Note — install locations (Windows).** Satchel stores its config and seed under **`%LOCALAPPDATA%`** (machine-bound — the seed must not roam; it was previously under Roaming `%APPDATA%`). The per-user installer also appends the install dir to your user **PATH**, so `pact-cli`/`pactd` run from any terminal — open a **new** terminal after installing for the change to take effect.

## One-shot regtest playground

```sh
./tools/playground-cork.ps1    # regtest nodes + Corkboard + headless
                               # counterparties, then launches Satchel
./tools/playground-nostr.ps1   # same, but over a local Nostr relay (no board)
```

Each script brings up the whole stack and blocks on the Satchel window — close it and everything is torn down automatically (`-Down` force-tears a stale run; teardown is PID/port-only).

## See also

- [Running pactd](Running-pactd) · [Self-Hosting a Corkboard](Self-Hosting-Corkboard) · [Satchel User Guide](Satchel-User-Guide)
