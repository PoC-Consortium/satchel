# Running pactd

`pactd` is the swap daemon — the long-lived process that exposes the `libswap`
engine over JSON-RPC 2.0, persists state in SQLite, and runs the scheduler that
auto-redeems, auto-refunds, and RBF-fee-bumps with no human present. This
chapter is the operator's reference: the full command line, every flag, RPC
authentication, and the scheduler model.

## Synopsis

`pactd` is a JSON-RPC 2.0 daemon served over **HTTP POST `/`** (plus an
unauthenticated `GET /health`), loopback-only:

```text
pactd [--data-dir <DIR>] [--coins-file <coins.toml>] [--coin <id>=<url[,url]> ...]
      [--coin-confs <id>=<N> ...] [--listen <addr:port>] [--network <net>]
      [--board-url <url[,url]>] [--nostr-relay <wss://…[,…]>] [--auto-fund]
      [--tick-secs <s>] [--once] [--auto-init] [--merchants]
```

No flag is required. With no `--coin`, the daemon starts but cannot move funds
on any chain until a backend is attached.

## Flags

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `--data-dir` | path | platform default | Data directory holding the seed, SQLite state, `.cookie`, and `pact.conf`. In `--merchants` mode this is the *parent* of `merchants/<id>/`. Defaults bitcoind-style to `%APPDATA%\Pact` (Windows), `~/Library/Application Support/Pact` (macOS), `~/.pact` (elsewhere), with mainnet at the root and `testnet`/`regtest` nested beneath — `pact-cli` autodiscovers the same location. |
| `--coins-file` | path | none | A `coins.toml` adding coins beyond the two built-ins (`btcx`, `btc`); merged at startup. A file coin whose id collides with a built-in is dropped. A bad file logs and falls back to the built-ins rather than refusing boot. |
| `--coin` | repeatable `id=url[,url]` | none | Per-coin chain backend. An `http://` first URL is the wallet-qualified Core-RPC primary that funds swaps, and any further Electrum URLs (`tcp://` / `ssl://`) are light chain views. A list with **no** `http://` URL (Electrum-only) runs the coin **nodeless** — a bdk wallet on the Pact seed, with the first Electrum URL serving the wallet (see the chapter *Coins, Pairs & Capabilities*). The coin id must be in the registry; the last `--coin` for a given id wins. |
| `--coin-confs` | repeatable `id=N` | network/spacing default | Per-coin confirmation depth (reorg finality). Clamped into the `[2, default]` band on mainnet/testnet — the default heuristic is also the maximum; regtest floors at 1. Gates auto-redeem and completion in v1 and v2. Omitted coins use the default (see the chapter *Coins, Pairs & Capabilities*). |
| `--listen` | `addr:port` | `127.0.0.1:9737` | JSON-RPC listen address. **Must be loopback** — this is enforced; a non-loopback address aborts boot. |
| `--network` | string | `regtest` | One of `regtest`, `testnet`, `mainnet`. |
| `--board-url` | string | none | Corkboard base URL(s), comma-separated (the HTTP transport). |
| `--nostr-relay` | string | none | Nostr relay `wss://…` URL(s), comma-separated. Runs **alongside** any `--board-url`; empty or absent disables it. |
| `--auto-fund` | flag | false | Auto-fund our leg of swaps (the engine mechanism). The standalone CLI flag is opt-in; **Satchel always launches `pactd` with it on**. v2 (adaptor) swaps auto-fund regardless via the autopilot. The end-to-end harness uses this flag to drive manual-vs-auto funding. |
| `--tick-secs` | u64 | `30` | Scheduler interval in seconds; `0` disables the background loop entirely. |
| `--once` | flag | false | Run a single scheduler pass (`sync_board` + `tick`), print the resulting events as JSON, and exit. Exit code is `1` if any event has `action == "error"`. |
| `--auto-init` | flag | false | Create the seed + state on first run (flat layout). No-op if a seed already exists. |
| `--merchants` | flag | false | Use the nested `merchants/<id>/` layout with an in-process registry. Without it, the legacy **flat** layout (one seed in the data-dir root) is used. Ignored once a flat seed already exists. |

> **Note** — The default `--network` is `regtest`. For a real deployment you
> must set `--network mainnet` (or `testnet`) explicitly; the network selects the
> chain params, the timelock-margin profile, and the confirmation-depth
> defaults.

> **Warning** — `--listen` is loopback-enforced by design. Do not try to bind a
> routable address to share a daemon — it will refuse to start. The RPC has no
> remote-access model; see the chapter *Architecture & Trust Boundaries*.

> **Note** — The standalone `--auto-fund` flag is opt-in (off by default), but
> **Satchel always launches `pactd` with it on** — auto-funding is the single,
> always-on behaviour for app users. It is safe as a default because offers are
> one-shot: a taken offer cannot be re-taken, so a maker's exposure is bounded by
> the size of the offers they posted. (v2 adaptor swaps auto-fund regardless, via
> the autopilot.)

## The data-dir lock and machine identity

Two further artifacts live at the data-dir root from the first run:

- **`.lock`** — on startup `pactd` takes an **exclusive** advisory OS lock on
  `<data-dir>/.lock` (Bitcoin-Core-style) and refuses to start if another
  daemon already holds it. One data directory is one daemon, never two.
- **`machine.json`** — this install's random per-install **derive scope**,
  generated on first run. It partitions key derivation between machines
  sharing one seed, so a failover or standby install never derives the same
  swap secrets as another machine. See the chapter "Seeds, Wallets &
  Merchants" and `docs/MULTI_MACHINE_122.md`.

## Logging

`pactd` writes its `tracing` output to **both** stdout and a rolling daily file
at `<data-dir>/logs/pactd.log.<date>` (for example
`logs/pactd.log.2026-06-20`). The file exists because a managed daemon spawned
by Satchel has no stdout capture, so the file is the only record of what the
engine did.

- The scheduler tags every swap event (`tracing::info!(swap, action, detail)`),
  so swap narration is captured per swap. The `dumpswap` RPC extracts the lines
  for one `swap_id` from these files.
- Verbosity follows `RUST_LOG` (an `EnvFilter`); the default level is `INFO`.
- **No secrets are ever logged.** Seeds, passphrases, the v1 preimage, and
  MuSig2 nonces are never passed to `tracing`, so the log file is safe to share
  for diagnostics.

## RPC authentication

Authentication is HTTP Basic, modelled on `bitcoind`:

- **Cookie (always on).** On startup `pactd` writes `<data-dir>/.cookie`
  containing `__cookie__:<32-byte hex>`, regenerated per run, and removes it on
  clean shutdown. Local clients (the CLI, Satchel) read this file — this is the
  zero-config default.
- **`pact.conf` (optional).** A `<data-dir>/pact.conf` file of `key = value`
  lines (`#` for comments) may set `rpcuser` and `rpcpassword`; those credentials
  are accepted *alongside* the cookie.
- Both forms are expanded to `Basic <base64(user:pass)>` and compared in
  constant time. The `GET /health` endpoint is unauthenticated.

> **Note** — To open an *encrypted* seed at boot without an interactive unlock,
> set the `PACT_PASSPHRASE` environment variable; the daemon uses it to decrypt
> the seed on startup. See the chapter *Seeds, Wallets & Merchants*.

The default RPC port is **9737**. (Coin peer-to-peer ports live in each coin's
chain params and are unrelated to the RPC port.)

## The scheduler tick model

`pactd` runs a background loop that fires every `--tick-secs` seconds (default
`30`). Each pass does two things: it syncs configured boards/relays (pulling new
offers and inbound sealed messages) and it runs the engine `tick`, which advances
every live swap's state machine — funding when due, redeeming when the secret is
available, fee-bumping a stuck transaction, and broadcasting a refund when a
timelock deadline passes. This is the mechanism that makes swaps *unattended*:
once a swap is underway, the scheduler will redeem your winnings and refund your
losses without any further RPC calls.

Setting `--tick-secs 0` disables the loop; the engine then advances only when
you call `tick` (or run with `--once`). That is useful for tests and for
externally orchestrated setups, but for a normal deployment leave the scheduler
running — a disabled scheduler means **no auto-refund**, which can lose funds if
a counterparty disappears.

> **Warning** — Do not run a mainnet maker with `--tick-secs 0` and walk away.
> The auto-refund safety net only fires on scheduler ticks; with the loop off, a
> stalled swap will sit unrefunded until you manually tick it.

## Example invocations

A regtest daemon that auto-creates its seed and runs over a local Corkboard:

```sh
pactd --data-dir ./data \
      --network regtest \
      --auto-init \
      --coin btcx=http://user:pass@127.0.0.1:19443/wallet/swap \
      --coin btc=http://user:pass@127.0.0.1:18443/wallet/swap \
      --board-url http://127.0.0.1:9780
```

A more realistic mainnet-shaped daemon: nested merchant layout, an encrypted
seed opened from the environment, the default Nostr transport, and an explicit
per-coin confirmation override:

```sh
PACT_PASSPHRASE='…' pactd \
      --data-dir /var/lib/pact \
      --network mainnet \
      --merchants \
      --coin btcx=http://user:pass@127.0.0.1:9332/wallet/swap \
      --coin btc=http://user:pass@127.0.0.1:8332/wallet/swap \
      --coin-confs btc=6 --coin-confs btcx=10 \
      --nostr-relay wss://relay.example.com,wss://relay2.example.com \
      --tick-secs 30
```

Here `--merchants` puts each seed under `merchants/<id>/`, `PACT_PASSPHRASE`
unlocks the encrypted seed at boot, the two `--coin` flags attach
wallet-qualified Core-RPC backends that fund the swap legs, and `--coin-confs`
overrides the per-coin finality depth. The defaults for coins, pairs, and
confirmation depth are explained in the next chapter.
