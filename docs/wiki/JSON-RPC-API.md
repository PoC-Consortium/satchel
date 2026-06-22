# JSON-RPC API

[pactd](Running-pactd) exposes the swap engine over **JSON-RPC 2.0** — 54 methods, grouped below by area with a one-line purpose each. This is an index; for full params, returns, and field shapes see the **Pact handbook API part**: <https://github.com/PoC-Consortium/satchel/tree/master/docs/handbook-pact>.

## Conventions

- **Transport:** HTTP `POST /` on `127.0.0.1:9737` (loopback only). `GET /health` is the unauthenticated liveness probe.
- **Auth:** HTTP Basic, `bitcoind`-style — the per-run cookie at `<data-dir>/.cookie`, or `rpcuser`/`rpcpassword` from an optional `pact.conf`.
- **Request:** `{ jsonrpc, id, method, params }`. `params` accept either a **positional array or a named object**.
- **Response:** `{ jsonrpc:"2.0", id, result }` on success, or `{ …, error:{ code:-1, message } }` on failure. Unknown methods return an error.
- **No platform fees:** `platform_fee_sat` is hard-wired `0` everywhere fees are reported.
- All swap/board/seed methods act on the **active merchant's engine**; with none loaded they error `"no active merchant — create or load one first"`.

## Node / info

| Method | Purpose |
|---|---|
| `getinfo` | Daemon name/version/protocol/network, identity, seed status, coin ids. |
| `walletstatus` | `{ seed_exists, encrypted, locked }`. |
| `getfeepolicy` | Active merchant's fee-bump policy `{ max_feerate_sat_vb, min_fee_sat, reservation_mult, committed_mult, step_pct }`. |
| `setfeepolicy` | Update the fee-bump policy — positional, all optional `[max_feerate_sat_vb?, min_fee_sat?, reservation_mult?, committed_mult?, step_pct?]`; returns the updated policy; persisted per-merchant. |
| `stop` | Trigger graceful shutdown. |

## Seed / wallet lifecycle

| Method | Purpose |
|---|---|
| `createseed` | Create + persist a seed; returns the mnemonic once (encrypted iff a passphrase is given). |
| `generateseed` | Generate a mnemonic preview **without** persisting it (onboarding). |
| `importseed` | Import a mnemonic (optional passphrase); returns identity. |
| `unlock` | Unlock an encrypted seed by trial-decrypt; holds the passphrase in memory. |

## Merchants

| Method | Purpose |
|---|---|
| `createmerchant` | Allocate next `m<N>` and make it active (nested mode). |
| `listmerchants` | All merchants + which is active. |
| `loadmerchant` | Switch active merchant (refused if current has a live swap). |
| `unloadmerchant` | Unload active merchant (same fund-safety gate). |
| `getmerchantinfo` | Metadata for one merchant (defaults to active). |

## Coins / pairs

| Method | Purpose |
|---|---|
| `listcoins` | All registry coins with capabilities, live status/tip, and confirmation depths. |
| `listpairs` | Derived (never curated) tradeable pairs with supported protocols. |
| `validatecoin` | Genesis-hash check of a proposed backend; engine config untouched. |

## Swaps — v1 HTLC

| Method | Purpose |
|---|---|
| `listswaps` | All v1 swap records. |
| `getswap` | One swap record by id. |
| `listpendingtakes` | Takes awaiting maker initiation. |
| `listmyoffers` | My posted offers with expiry/state. |
| `offer` | Start a swap as initiator (`give`/`get` = `coin:amount`, `t1`/`t2`). |
| `acceptoffer` | Accept an offer envelope. |
| `recv` | Receive/ingest a counterparty envelope. |
| `fund` | Fund our HTLC leg (broadcasts). |
| `redeem` | Redeem the counterparty leg (broadcasts). |
| `refund` | Refund our funded leg after timeout (broadcasts). |
| `abort` | Abort before our leg is funded. |
| `tick` | Run one scheduler pass; returns events. |

## Swaps — v2 adaptor (Taproot/MuSig2)

v2 adaptor swaps are enabled on **all networks including mainnet** (reviewed).

| Method | Purpose |
|---|---|
| `listadaptorswaps` | All v2 swap records. |
| `adaptorinit` | Start a v2 swap as initiator. |
| `adaptoraccept` | Accept a v2 offer. |
| `adaptorrecv` | Ingest a v2 envelope. |
| `adaptorfundingready` | Declare a funding output (`txid`,`vout`). |
| `adaptornonces` | Exchange MuSig2 nonces. |
| `adaptorsign` | Produce partial adaptor signatures. |
| `adaptorassemble` | Assemble the signed transactions. |
| `adaptorfund` | Broadcast our funding tx. |
| `adaptorredeem` | Redeem (broadcasts). |
| `adaptorrefund` | Refund after timeout (broadcasts). |

## Board (Corkboard / Nostr)

| Method | Purpose |
|---|---|
| `boardlistoffers` | Browse one board's offers (`board` = URL or `"nostr"`). |
| `boardstatus` | Per-relay connectivity. |
| `boardpostoffer` | Post an offer; fans out to all configured boards. |
| `boardtake` | Take a posted offer. |
| `boardrevoke` | Revoke one of my offers. |

## Private (off-market) offers

| Method | Purpose |
|---|---|
| `makeprivateoffer` | Produce a signed `pactoffer1:` slip; never posted to a board. |
| `takeoffer` | Take a slip received over chat. |
| `listprivateoffers` | My outstanding private offers. |
| `cancelprivateoffer` | Cancel a private offer. |

## Fees / wallet

| Method | Purpose |
|---|---|
| `estimateswapfees` | Per-leg fee estimate (`platform_fee_sat:0`). Params: `give_coin`, `get_coin` only. |
| `getbalance` | Balance for one chain. |
| `getnewaddress` | Fresh HD address for one chain. |
| `sendtoaddress` | Send from one chain (broadcasts). |

## Diagnostics

| Method | Purpose |
|---|---|
| `dumpswap` | Secret-free per-swap bundle (`swap_id`): scrubbed record + the `pactd` log lines mentioning that swap. Works for v1 and v2. Backs Satchel's **Dump logs** button. |

## See also

- [pact-cli](pact-cli) · [Running pactd](Running-pactd) · [Transports](Transports)
