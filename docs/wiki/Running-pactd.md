# Running pactd

`pactd` is the Pact swap engine: a local JSON-RPC daemon (`bitcoind`-style) that holds your seed, builds and watches swap transactions, and auto-refunds if a counterparty walks away. Satchel bundles and supervises it for you — you only run it by hand if you want to drive swaps from the command line or integrate the engine yourself.

This page is a quick reference. For the full flag table, RPC-auth details, and data-layout, see the **Pact handbook "Running pactd" chapter**: <https://github.com/PoC-Consortium/satchel/tree/master/docs/handbook-pact>.

## Synopsis

```text
pactd [--data-dir <DIR>] [--coins-file <coins.toml>] [--coin <id>=<url[,url]> ...]
      [--coin-confs <id>=<N> ...] [--listen <addr:port>] [--network <net>]
      [--board-url <url[,url]>] [--nostr-relay <wss://…[,…]>] [--auto-fund]
      [--tick-secs <s>] [--once] [--auto-init] [--merchants]
```

## Most-used flags

| Flag | Default | Meaning |
|---|---|---|
| `--data-dir <DIR>` | platform default | Data directory: seed, SQLite, `.cookie`, optional `pact.conf`. In `--merchants` mode this is the *parent* of `merchants/<id>/`. Default (bitcoind-style): `%APPDATA%\Pact` on Windows, `~/Library/Application Support/Pact` on macOS, `~/.pact` elsewhere — mainnet at the root, `testnet`/`regtest` nested beneath. `pact-cli` autodiscovers the same location. |
| `--coin <id>=<url[,url]>` | none | Per-coin chain backend, repeatable. An `http://` first URL = wallet-qualified Core-RPC primary (the node wallet funds swaps); extra URLs may be Electrum (`tcp://`/`ssl://`) chain views. An **Electrum-only list** (no `http://`) makes the coin **nodeless** — the wallet lives on the Pact seed; mainnet requires ≥ 2 servers. See [Configuring Coins](Configuring-Coins). The coin `id` must exist in the registry. |
| `--coin-confs <id>=<N>` | network default | Per-coin confirmation depth (reorg finality, N≥1); gates auto-redeem and completion. See the default heuristic in [Configuring Coins](Configuring-Coins). |
| `--listen <addr:port>` | `127.0.0.1:9737` | JSON-RPC listen address. **Loopback only** — a non-loopback address aborts boot. |
| `--network <net>` | `regtest` | `regtest` \| `testnet` \| `mainnet`. |
| `--board-url <url[,url]>` | none | Corkboard base URL(s), comma-separated (HTTP transport). |
| `--nostr-relay <wss://…[,…]>` | none (empty) | Nostr relay URL(s), comma-separated. Runs *alongside* `--board-url`; empty disables Nostr. |
| `--tick-secs <s>` | `30` | Background scheduler interval in seconds; `0` disables the loop. |
| `--merchants` | off | Nested `merchants/<id>/` layout (one seed = one trading identity). Without it: flat single-seed layout in the data-dir root. |
| `--auto-init` | off | Create seed + state on first run (flat layout). No-op if a seed already exists. |
| `--auto-fund` | off | Auto-fund our leg of swaps. The CLI flag is opt-in, but **Satchel always launches with it on** — auto-funding is the single always-on behaviour. (v2 adaptor swaps auto-fund regardless, via the autopilot.) |

> **Note** — the default RPC port is `9737`. Coin P2P/RPC ports live in each coin's chain params and are unrelated to this.

> **Note** — `pactd` writes a rolling daily log to `<data-dir>/logs/pactd.log.<date>` (in addition to stdout), honouring `RUST_LOG` (default `INFO`). It is **secret-free** — seeds, the v1 preimage, and MuSig2 nonces are never logged — so it is safe to share. The `dumpswap` RPC pulls the lines for a single swap out of these files.

## RPC authentication

`pactd` uses HTTP Basic auth, exactly like `bitcoind`:

- **Cookie (always on):** at startup it writes `<data-dir>/.cookie` containing `__cookie__:<32-byte hex>`, and removes it on clean shutdown. A new cookie is generated per run.
- **`pact.conf` (optional):** a `<data-dir>/pact.conf` with `rpcuser = …` / `rpcpassword = …` lines (`#` comments allowed) adds stable credentials alongside the cookie.

`GET /health` is unauthenticated. To open an encrypted seed at boot, set the `PACT_PASSPHRASE` environment variable.

## Environment overrides

- **`PACT_PASSPHRASE`** — opens an encrypted seed at boot (above).
- **`RUST_LOG`** — log verbosity (default `INFO`).
- **`SATCHEL_DATA_DIR`** — overrides the OS app-data base directory Satchel uses. It cascades to the `pactd` instance Satchel manages and to all config and merchants underneath, so pointing it elsewhere gives you a fully isolated stack — handy for playground/tester instances that mustn't touch your real data dir.

## Regtest example

```sh
pactd --data-dir ./alice \
      --network regtest \
      --coin btcx=http://__cookie__:<hex>@127.0.0.1:19443/wallet/alice \
      --coin btc=http://__cookie__:<hex>@127.0.0.1:19543/wallet/alice \
      --board-url http://127.0.0.1:9780 \
      --auto-init --tick-secs 5
```

Then drive it with [pact-cli](pact-cli):

```sh
pact-cli --data-dir ./alice getinfo
```

## Mainnet note

Point `--coin` at your mainnet node RPCs and pass `--network mainnet`. Both swap types run on mainnet (v1 HTLC and v2 Taproot/MuSig2 adaptor); the protocol and implementation are **reviewed**. You alone hold your keys — run `pactd` on a trusted machine and keep `--listen` on loopback.

## See also

- [pact-cli](pact-cli) · [JSON-RPC API](JSON-RPC-API) · [Configuring Coins](Configuring-Coins) · [Transports](Transports)
