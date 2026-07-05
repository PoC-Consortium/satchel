# JSON-RPC Conventions

`pactd` exposes the `libswap` engine over **JSON-RPC 2.0**. This chapter
covers the transport, authentication, request/response envelope, and the
common error shapes that every method shares. The per-method reference lives
in the chapters that follow ("API: Node, Seed, Merchants, Coins", "API: v1
HTLC Swaps", "API: v2 Adaptor Swaps", and "API: Board, Private Offers &
Fees").

> **Note** — `pactd` is to the swap engine what `bitcoind` is to Bitcoin: a
> long-running daemon driven entirely over RPC. `pact-cli` is the thin
> command-line client (the `bitcoin-cli` analog), and Satchel is the desktop
> GUI (the `bitcoin-qt` analog).

## Transport

| Property | Value |
|---|---|
| Protocol | JSON-RPC 2.0 over HTTP |
| Method endpoint | `POST /` |
| Health endpoint | `GET /health` (returns `ok`, **unauthenticated**) |
| Default listen address | `127.0.0.1:9737` |
| Binding | **Loopback only** — a non-loopback `--listen` aborts boot |

All RPC calls are HTTP `POST` to the root path `/`. The daemon enforces that
its listen address is a loopback address (`127.0.0.1`/`::1`); binding to a
routable interface is rejected at startup. To reach `pactd` from another host,
front it with your own authenticated tunnel — never expose the port directly.

`GET /health` is the only unauthenticated route. It returns the literal string
`ok` and is intended for liveness probes.

## Authentication

`pactd` uses **HTTP Basic** auth, exactly like `bitcoind`. Two credential
sources are accepted (either works):

1. **Cookie (always written).** On startup `pactd` writes
   `<datadir>/.cookie` containing `__cookie__:<hex>`, where `<hex>` is a fresh
   32-byte random value rendered as hex. The file is rewritten per run and
   removed on a clean shutdown. Use `__cookie__` as the username and the hex
   as the password.
2. **`pact.conf` (optional).** A `<datadir>/pact.conf` file with
   `key = value` lines (`#` introduces a comment). If it sets `rpcuser` and
   `rpcpassword`, those credentials are accepted **alongside** the cookie.

Both sources are normalized to a `Basic <base64(user:password)>` header and
compared in constant time. `pact-cli` reads `.cookie` automatically when you
pass `--data-dir`; pass `--rpcuser`/`--rpcpassword` to override.

> **Tip** — The passphrase that unlocks an encrypted seed is **not** an RPC
> credential. Supply it via the `unlock` method or the `PACT_PASSPHRASE`
> environment variable at boot. See the chapter "Wallet & Seed Lifecycle".

## Request shape

Every request is a JSON object:

```json
{ "jsonrpc": "2.0", "id": 1, "method": "getinfo", "params": [] }
```

| Field | Required | Notes |
|---|---|---|
| `jsonrpc` | no | Ignored on input; the response always sets `"2.0"`. |
| `id` | no | Echoed verbatim in the response. |
| `method` | yes | One of the methods in the following chapters. |
| `params` | no | Defaults to `null`. See below. |

**Params accept a positional array OR a named object.** These two calls are
equivalent:

```json
{ "method": "offer", "params": ["btcx:1.0", "btc:0.5", 144, 72] }
```

```json
{ "method": "offer",
  "params": { "give": "btcx:1.0", "get": "btc:0.5", "t1": 144, "t2": 72 } }
```

Numeric params also accept numeric strings (e.g. `"144"`), matching
`bitcoin-cli` behaviour. A missing required param fails with
`missing param '<name>'` (positional form adds `(position <i>)`).

## Response shape

A successful call returns:

```json
{ "jsonrpc": "2.0", "id": 1, "result": { "...": "..." } }
```

An error returns an `error` object instead of `result`:

```json
{ "jsonrpc": "2.0", "id": 1,
  "error": { "code": -32601,
             "message": "unknown method 'getblance' — did you mean 'getbalance'? (see 'help')" } }
```

| Error | Code | Message text |
|---|---|---|
| Unknown method | `-32601` | `unknown method '<method>' — did you mean '<nearest>'? (see 'help')` (the *did-you-mean* hint appears only when the name is plausibly a typo) |
| Missing param | `-1` | `missing param '<name>'` |
| No merchant loaded | `-1` | `no active merchant — create or load one first` |

An unknown method returns JSON-RPC's standard *method not found* code
`-32601`; every other error is code `-1`, with the human-readable detail in
`message`.

> **Note** — All swap, board, offer, and seed RPCs operate on the **active
> merchant's** engine. If no merchant is loaded (fresh nested-mode datadir
> with nothing selected), they fail with
> `no active merchant — create or load one first`. Use `createmerchant` /
> `loadmerchant` first. See the chapter "API: Node, Seed, Merchants, Coins".

## Examples

A raw `curl` call against the cookie file:

```sh
# Cookie file holds "__cookie__:<hex>"; pass it straight to --user.
curl -s --user "$(cat ~/.pact/.cookie)" \
  -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getinfo","params":[]}' \
  http://127.0.0.1:9737/
```

The same call through `pact-cli` (which autodiscovers the `.cookie` — an
explicit `--data-dir` also works):

```sh
pact-cli getinfo
```

Any method is callable this way — `pact-cli <method> [params...]` — with each
argument parsed as JSON if possible, otherwise treated as a string. `pact-cli
help` lists the daemon's full catalog — all **64 public methods**, the same
list `listmethods` returns as a name array — and an unknown method is refused
with JSON-RPC code `-32601` and a *did-you-mean* suggestion (all other errors
are code `-1` today).

> **Note** — `platform_fee_sat` is always `0`. There are no platform fees
> anywhere in the engine; the field exists only so fee previews have a
> consistent shape. See the chapter "API: Board, Private Offers & Fees".
