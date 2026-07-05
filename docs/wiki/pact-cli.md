# pact-cli

`pact-cli` is a thin JSON-RPC client for [pactd](Running-pactd) — the `bitcoin-cli` of the suite. It connects over loopback HTTP, autodiscovers the cookie for auth, and prints the daemon's response.

This page is a quick reference. For full argument details and worked examples, see the **Pact handbook** chapters on the CLI: <https://github.com/PoC-Consortium/satchel/tree/master/docs/handbook-pact>.

## Zero-config usage

Against a default local `pactd`, no flags are needed — **every RPC method is a direct subcommand**, and auth is found automatically:

```sh
pact-cli getinfo
pact-cli getbalance btc
pact-cli help              # the daemon's full method catalog, by category
pact-cli help sendtoaddress
```

Each argument is parsed as JSON when possible, else passed as a plain string (bitcoin-cli's convention). A typo'd method suggests the nearest real one (`unknown method 'getblance' — did you mean 'getbalance'?`).

## Auth discovery

Explicit `--rpcuser`/`--rpcpassword` win; an explicit `--data-dir` is read strictly. With neither, these dirs are searched for a `.cookie` (or `rpcuser`/`rpcpassword` in `pact.conf`), in order:

1. the `pactd` platform default — `%APPDATA%\Pact` (Windows), `~/Library/Application Support/Pact` (macOS), `~/.pact` (elsewhere); mainnet at the root, `testnet`/`regtest` nested per `--network`;
2. Satchel's managed pactd dir (`<app-local-data>/org.pocx.satchel/[net]/pactd`).

With `--rpc` omitted, the default URL follows **where the auth was found**: the platform-default dir means a hand-run pactd, which always listens on `9737` whatever the network; Satchel's managed dir means the per-network port — `9737` (mainnet) / `9738` (testnet) / `9739` (regtest). So `pact-cli --network regtest getbalance btcx` reaches Satchel's regtest daemon with no `--rpc` at all.

## Global flags

| Flag | Default | Meaning |
|---|---|---|
| `--rpc <url>` | derived from where the auth was found (`9737`, or Satchel's `9737`/`9738`/`9739` per `--network`) | pactd JSON-RPC endpoint. |
| `--data-dir <DIR>` | autodiscovered | Where to read the `.cookie` / `pact.conf` for auth. |
| `--network <net>` | `regtest` | Network subdir the auth discovery looks under (mirrors pactd's default). |
| `--rpcuser` / `--rpcpassword` | none | Explicit credentials (skip discovery entirely). |

## Structured subcommands

Beyond direct method dispatch, a handful of subcommands wrap an RPC **plus file I/O** for the manual (file-passing) swap handshake, or add flag-style arguments:

| Subcommand | Args | RPC method |
|---|---|---|
| `offer` | `--give --get --t1 --t2 --out` | `offer` |
| `accept` | `--in --out` | `acceptoffer` |
| `recv` | `--in` | `recv` |
| `fund` | `--swap --out` | `fund` |
| `redeem` | `--swap` | `redeem` |
| `refund` | `--swap` | `refund` |
| `abort` | `--swap --reason` | `abort` |
| `status` | `--swap` (optional) | `getswap` / `listswaps` |
| `restore` | — | `restorefromrelay` |
| `rescue-status` | — | `rescuestatus` |
| `walletstatus` | — | `walletstatus` |
| `coins` | — | `listcoins` |
| `pairs` | — | `listpairs` |
| `transactions` | `<coin>` | `listtransactions` |
| `validatecoin` | `--coin --backend` | `validatecoin` |
| `createseed` | `--passphrase` (opt) `--words` (12 default \| 24) | `createseed` |
| `importseed` | `--mnemonic --passphrase` (opt) | `importseed` |
| `unlock` | `--passphrase` | `unlock` |
| `board post` | `--give --get --t1_secs` (d=12h) `--t2_secs` (d=6h) | `boardpostoffer` |
| `board offers` | — | `boardlistoffers` |
| `board take` | `--offer` | `boardtake` |
| `board revoke` | `--offer` | `boardrevoke` |
| `board sync` | — | `tick` |

## Direct dispatch (and `call`)

Any other RPC method is a subcommand of its own name; `call <method> [params...]` remains as the explicit passthrough spelling of the same thing:

```sh
pact-cli estimateswapfees btcx btc
pact-cli adaptorinit btcx:100000 btc:100000 600 300
pact-cli listmethods                # machine-readable name array
```

See `pact-cli help` (the daemon's catalog) or the [JSON-RPC API](JSON-RPC-API) index for the full method list.

## See also

- [Running pactd](Running-pactd) · [JSON-RPC API](JSON-RPC-API)
