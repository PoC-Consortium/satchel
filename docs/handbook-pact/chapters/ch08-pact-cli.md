# The pact-cli

`pact-cli` is the thin command-line client for `pactd` — the `bitcoin-cli` of
this stack. It is a hand-rolled HTTP caller (a raw `TcpStream` with a 120-second
read timeout and chunked-response decoding) that maps subcommands onto JSON-RPC
methods. This chapter covers its global flags, the structured subcommand table,
the generic `call` escape hatch (and which methods need it), and a worked
end-to-end v1 swap.

## Global flags

These apply to every invocation:

| Flag | Default | Meaning |
|---|---|---|
| `--rpc` | `http://127.0.0.1:9737` | The `pactd` JSON-RPC endpoint. |
| `--data-dir` | — | Data directory to read the `.cookie` from (cookie auth). |
| `--rpcuser` | — | Explicit RPC username (overrides cookie auth). |
| `--rpcpassword` | — | Explicit RPC password. |

Authentication uses the explicit `--rpcuser`/`--rpcpassword` credentials if
given, otherwise the cookie read from `--data-dir`. This mirrors `bitcoin-cli`.

## Subcommands

The CLI exposes structured subcommands for the common v1 and board operations:

| Subcommand | Flags / args | RPC method |
|---|---|---|
| `call <method> [params...]` | each param JSON-parsed, else treated as a string | **any** (generic passthrough) |
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
| `validatecoin` | `--coin --backend` | `validatecoin` |
| `createseed` | `--passphrase` (optional) | `createseed` |
| `importseed` | `--mnemonic --passphrase` (optional) | `importseed` |
| `unlock` | `--passphrase` | `unlock` |
| `board post` | `--give --get --t1_secs` (default 12h) `--t2_secs` (default 6h) | `boardpostoffer` |
| `board offers` | — | `boardlistoffers` |
| `board take` | `--offer` | `boardtake` |
| `board revoke` | `--offer` | `boardrevoke` |
| `board sync` | — | `tick` |

## The generic `call` escape hatch

The `pact-cli call <method> [params...]` form invokes **any** RPC method
directly. Each positional argument is JSON-parsed if it parses, otherwise passed
as a string — so `call getbalance btc` and `call offer btcx:1.0 btc:0.9 ...`
both work. This is how you reach everything that has no structured subcommand.

> **Note** — There is a real *CLI gap*: large parts of the RPC surface have no
> dedicated subcommand and are reachable **only** through `call`. That includes
> all of the **v2 adaptor** methods (`adaptorinit`, `adaptoraccept`,
> `adaptorfund`, `adaptorredeem`, …), all **merchant** methods
> (`createmerchant`, `loadmerchant`, …), the **private-offer** methods
> (`makeprivateoffer`, `takeoffer`, …), the **wallet** methods (`getbalance`,
> `getnewaddress`, `sendtoaddress`), and assorted others (`getinfo`,
> `estimateswapfees`, `generateseed`, `boardstatus`, `listmyoffers`,
> `listpendingtakes`). For those, use `call`.

For example, to drive a v2 adaptor swap or check a balance:

```sh
pact-cli call getinfo
pact-cli call getbalance btc
pact-cli call adaptorinit btcx:1.0 btc:0.95 86400 43200
pact-cli call estimateswapfees btcx btc
```

## Worked example: a v1 swap with two CLIs

This is the manual happy path the harness drives in `test_swap_e2e.py`, with two
daemons — **Alice** (the initiator, giving BTCX, getting BTC) and **Bob** (the
participant). Each `pact-cli` here is assumed pointed at its own daemon's
`--rpc` / `--data-dir`; messages are passed between the two as JSON files.

1. **Alice makes the offer.** She gives `btcx`, gets `btc`, with timelocks `t1`
   (her chain-A refund) and `t2 < t1` (Bob's chain-B refund). This writes an
   init envelope:

   ```sh
   alice> pact-cli offer --give btcx:1.0 --get btc:0.95 \
                         --t1 <T1> --t2 <T2> --out init.json
   ```

2. **Bob accepts.** He reads the init envelope, reconstructs the HTLC scripts
   locally, and emits an accept envelope:

   ```sh
   bob> pact-cli accept --in init.json --out accept.json
   ```

3. **Alice receives the acceptance** and funds her chain-A leg first (the maker
   funds first), producing a `funded` message that names the funding outpoint:

   ```sh
   alice> pact-cli recv --in accept.json
   alice> pact-cli fund --swap <swap_id> --out funded_a.json
   ```

4. **Bob verifies Alice's funding on-chain, then funds his chain-B leg:**

   ```sh
   bob> pact-cli recv --in funded_a.json
   bob> pact-cli fund --swap <swap_id> --out funded_b.json
   ```

5. **Alice receives Bob's funding, then redeems chain B** — this is the step
   that reveals her preimage `s` on the BTC chain:

   ```sh
   alice> pact-cli recv --in funded_b.json
   alice> pact-cli redeem --swap <swap_id>     # reveals s on chain B
   ```

6. **Bob redeems chain A.** His engine extracts `s` from Alice's chain-B
   spend and uses it to claim the BTCX leg:

   ```sh
   bob> pact-cli redeem --swap <swap_id>       # engine extracted s from chain B
   ```

Both HTLCs are now spent and the swap is `completed`. The `swap_id` is the
16-hex identifier returned by `offer` (and visible via `status`).

> **Tip** — In a real deployment you do not perform steps 5 and 6 by hand. With
> the scheduler running (the default `--tick-secs 30`), each side's daemon
> redeems automatically once the required confirmations are reached, and
> auto-refunds if a deadline passes — *see the chapters "Running pactd" and the
> swap-protocol chapters*. The manual flow above is the way to understand and
> test the protocol, not the way to operate it.

> **Warning** — The timelock ordering is load-bearing: `t2` (Bob's refund) must
> be earlier than `t1` (Alice's refund), and both must respect the network's
> action-deadline margins. The engine validates these offsets at offer time and
> will reject an unsafe pair; do not work around the rejection. See the
> timelock material in the protocol part of this handbook.
