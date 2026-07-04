# The pact-cli

`pact-cli` is the thin command-line client for `pactd` — the `bitcoin-cli` of
this stack. It is a hand-rolled HTTP caller (a raw `TcpStream` with a 120-second
read timeout and chunked-response decoding) that maps subcommands onto JSON-RPC
methods. This chapter covers its global flags and auth discovery, direct method
dispatch, the structured subcommand table, and a worked end-to-end v1 swap.

## Direct dispatch and `help`

**Every RPC method is a subcommand of its own name** — the first token that is
not a structured subcommand is sent to the daemon as the method, with the
remaining tokens as params (each JSON-parsed if it parses, else passed as a
string):

```sh
pact-cli getinfo
pact-cli getbalance btc
pact-cli adaptorinit btcx:1.0 btc:0.95 86400 43200
```

`pact-cli help` asks the daemon for its full method catalog by category;
`pact-cli help <method>` explains one method; `pact-cli listmethods` returns a
machine-readable name array. A typo'd method name is answered with the nearest
real one (`unknown method 'getblance' — did you mean 'getbalance'?`). Clap's
own usage text stays on `pact-cli --help`.

## Global flags

These apply to every invocation:

| Flag | Default | Meaning |
|---|---|---|
| `--rpc` | `http://127.0.0.1:9737` | The `pactd` JSON-RPC endpoint. |
| `--data-dir` | autodiscovered | Data directory to read the `.cookie` (or `pact.conf`) from. |
| `--network` | `regtest` | Network subdir the auth discovery looks under (mirrors `pactd`). |
| `--rpcuser` | — | Explicit RPC username (overrides cookie auth). |
| `--rpcpassword` | — | Explicit RPC password. |

Authentication mirrors `bitcoin-cli`: explicit `--rpcuser`/`--rpcpassword` win;
an explicit `--data-dir` is read strictly (its `.cookie`, else the
`rpcuser`/`rpcpassword` in its `pact.conf`). With neither flag, the CLI
searches, in order:

1. the `pactd` platform default — `%APPDATA%\Pact` (Windows),
   `~/Library/Application Support/Pact` (macOS), `~/.pact` (elsewhere) —
   mainnet at the root, `testnet`/`regtest` nested per `--network`;
2. Satchel's managed pactd dir
   (`<app-local-data>/org.pocx.satchel/[net]/pactd`). Satchel offsets its
   listen port per network (`9737`/`9738`/`9739`), so pass `--rpc` off-mainnet.

So against a default local `pactd`, `pact-cli getbalance btc` works with no
flags at all.

## Structured subcommands

A handful of operations additionally get structured subcommands — they wrap the
RPC plus the file I/O of the manual v1 handshake, or add flag-style arguments:

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

## The explicit `call` spelling

`pact-cli call <method> [params...]` is the explicit passthrough spelling of
direct dispatch — byte-for-byte the same request. It exists for scripts that
want to make "this token is an RPC method, not a CLI subcommand" unambiguous
(a structured subcommand name always wins over a same-named method in the bare
form).

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
