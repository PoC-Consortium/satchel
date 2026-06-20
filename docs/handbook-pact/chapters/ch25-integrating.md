# Building Your Own Front-End

Satchel is the reference desktop client, but it has no privileged access: it is
just one consumer of `pactd`'s JSON-RPC API. Anything Satchel does, your own
front-end — a CLI, a TUI, a web dashboard, a bot — can do, by speaking the same
JSON-RPC over loopback and authenticating with the cookie. This chapter is the
orientation for that; per-method detail lives in the API chapters ("API: Node,
Seed, Merchants, Coins", "API: v1 HTLC Swaps", "API: v2 Adaptor Swaps", "API:
Board, Private Offers & Fees").

## The integration contract

`pactd` is to the swap engine what `bitcoind` is to Bitcoin: a long-running
daemon you drive entirely over RPC. To integrate:

1. **Speak JSON-RPC 2.0 over loopback.** `POST /` with
   `{ "jsonrpc": "2.0", "id", "method", "params" }`. The listen address is
   `127.0.0.1:9737` by default and is enforced to be loopback. Params accept a
   positional array or a named object.
2. **Authenticate with the cookie.** `pactd` writes `<datadir>/.cookie` holding
   `__cookie__:<hex>` each run; pass it as HTTP Basic credentials. A `pact.conf`
   with `rpcuser`/`rpcpassword` is an alternative. `GET /health` is the only
   unauthenticated route. See "JSON-RPC Conventions" for the full handshake.
3. **Drive the same methods** Satchel does. There is no second, private surface.

> **Warning** — Never expose the `pactd` port directly to a network. It binds
> loopback only; if you need remote access, front it with your own authenticated,
> encrypted tunnel.

## A recommended app flow

A front-end typically walks this sequence. Each step maps to one or more RPC
methods; consult the API chapters for params and return shapes.

1. **Probe the daemon.** `getinfo` and `walletstatus` — is there a seed, is it
   encrypted, is it locked, what coins are configured, what network.
2. **Bring up the seed.** If no seed: `createseed` (or `generateseed` →
   `importseed` for a preview-then-confirm flow). If encrypted and locked:
   `unlock` with the passphrase (or set `PACT_PASSPHRASE` at boot).
3. **Select a merchant** if running nested mode (`--merchants`): `listmerchants`,
   then `createmerchant` / `loadmerchant`. All swap/board calls target the active
   merchant.
4. **Configure and check coins.** `listcoins` (live status, tip height, genesis,
   capabilities), `validatecoin` before saving a backend, `listpairs` to learn
   which pairs and protocols are available.
5. **Browse a board.** `boardlistoffers` (a single board per call — an HTTP URL
   or `"nostr"`), or build your own ladder from the returned offer envelopes.
6. **Post or take.** `boardpostoffer` to advertise; `boardtake` to take an offer;
   or `makeprivateoffer` / `takeoffer` for off-market slips.
7. **Poll and render.** Call `tick` on an interval to advance the scheduler and
   drain coordination mail, then `listswaps` / `listadaptorswaps` /
   `listpendingtakes` / `listmyoffers` to render state.

## What the front-end owns vs what `pactd` owns

The division of responsibility is strict and worth internalizing:

| The front-end owns | `pactd` owns |
|---|---|
| Presentation: order-book ladder, swap cards, balances, forms. | The seed and all keys; they never leave the daemon. |
| Input validation for UX (amounts, pair selection). | Funding, redeeming, and **refunds** — the on-chain transactions. |
| When to call `tick` / how often to poll. | The scheduler that auto-funds, auto-redeems, and auto-refunds on the clock. |
| Choosing which board to browse. | Sealing, signing, and the noticeboard fan-out. |
| Storing UI preferences. | Swap state, timelock enforcement, fee bumping. |

In short: your front-end is a *view and a controller*. It never holds key
material and never has to implement swap safety — the engine enforces timelocks
and refunds whether or not any UI is attached.

## Threading and polling

- **`pactd` self-ticks.** With `--tick-secs` (default 30) the daemon runs its own
  scheduler loop; safety does not depend on your UI polling. A swap will refund
  on time even if your front-end is closed, as long as the daemon is running.
- **Poll `tick` for responsiveness.** Calling `tick` from your UI on a short
  interval makes state advance promptly (e.g. picking up a take, redeeming a
  newly confirmed leg) rather than waiting for the next self-tick. `tick` returns
  `{ events: [ … ] }` you can surface as activity.
- **One active merchant at a time.** RPC calls operate on the active merchant; do
  not assume concurrency across merchants within one process.

## Error handling

Errors come back as a JSON-RPC `error` object, never an HTTP error status for
business failures:

```json
{ "jsonrpc": "2.0", "id": 1,
  "error": { "code": -1, "message": "no active merchant — create or load one first" } }
```

`code` is always `-1`; the human-readable cause is in `message`. Common ones to
handle: `unknown method '<m>'`, `missing param '<name>'`, and
`no active merchant — create or load one first` (prompt the user to create or
load a merchant). Treat the presence of an `error` key — not the HTTP status — as
the failure signal.

> **Tip** — For the exhaustive list of methods, their parameters, and return
> shapes, work from the API chapters. This chapter only sketches the flow; those
> are the contract.
