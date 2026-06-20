# pact-cli

`pact-cli` is a thin JSON-RPC client for [pactd](Running-pactd) — the `bitcoin-cli` of the suite. It connects over loopback HTTP, reads the cookie for auth, and prints the daemon's JSON response.

This page is a quick reference. For full argument details and worked examples, see the **Pact handbook** chapters on the CLI: <https://github.com/PoC-Consortium/satchel/tree/master/docs/handbook-pact>.

## Global flags

| Flag | Default | Meaning |
|---|---|---|
| `--rpc <url>` | `http://127.0.0.1:9737` | pactd JSON-RPC endpoint. |
| `--data-dir <DIR>` | none | Where to read the `.cookie` for auth. |
| `--rpcuser` / `--rpcpassword` | none | Explicit credentials (else the cookie is used). |

```sh
pact-cli --data-dir ./alice getinfo
```

## Subcommands

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
| `walletstatus` | — | `walletstatus` |
| `coins` | — | `listcoins` |
| `pairs` | — | `listpairs` |
| `validatecoin` | `--coin --backend` | `validatecoin` |
| `createseed` | `--passphrase` (opt) | `createseed` |
| `importseed` | `--mnemonic --passphrase` (opt) | `importseed` |
| `unlock` | `--passphrase` | `unlock` |
| `board post` | `--give --get --t1_secs` (d=12h) `--t2_secs` (d=6h) | `boardpostoffer` |
| `board offers` | — | `boardlistoffers` |
| `board take` | `--offer` | `boardtake` |
| `board revoke` | `--offer` | `boardrevoke` |
| `board sync` | — | `tick` |

## The `call` escape hatch

Any RPC method is reachable via the generic passthrough, with each argument JSON-parsed (falling back to a plain string):

```sh
pact-cli call estimateswapfees btcx btc
pact-cli call adaptorinit btcx:100000 btc:100000 600 300
```

> **Note — CLI gap.** There are no structured subcommands for v2 adaptor swaps, merchants, private offers, `getbalance`/`getnewaddress`/`sendtoaddress`, `getinfo`, `estimateswapfees`, `generateseed`, `boardstatus`, `listmyoffers`, or `listpendingtakes`. Reach those through `pact-cli call <method>` — see the [JSON-RPC API](JSON-RPC-API) index for the full list.

## See also

- [Running pactd](Running-pactd) · [JSON-RPC API](JSON-RPC-API)
