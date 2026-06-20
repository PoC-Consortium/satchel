# API: Board, Private Offers & Fees

This chapter documents the board (Corkboard / Nostr) RPCs, the private
("trade-with-a-friend") offer RPCs, and the fee-estimation method. For shared
conventions see the chapter "JSON-RPC Conventions"; for the swap lifecycle
those offers feed into, see "API: v1 HTLC Swaps" and "API: v2 Adaptor Swaps".

## Board (Corkboard / Nostr)

An offer posted to the board fans out to every configured transport (HTTP
Corkboard URLs and/or Nostr relays). Reads are per-board: a `board` selector
chooses one transport rather than merging.

| Method | Params | Returns | Mutates |
|---|---|---|---|
| `boardlistoffers` | `board?` | `{ offers }` | no |
| `boardstatus` | — | `{ relays:[{ url, connected }] }` | no |
| `boardpostoffer` | `give`, `get`, `t1_secs`, `t2_secs`, `protocol?`, `ttl_secs?` | `{ offer_id }` | yes |
| `boardtake` | `offer_id` | `{ taken }` | yes |
| `boardrevoke` | `offer_id` | `{ revoked }` | yes |

- `boardlistoffers` — lists offers from one board. The optional `board` is an
  HTTP Corkboard URL **or** the literal `"nostr"`; omitted, it defaults to the
  first configured board.
- `boardstatus` — relay connectivity for the header indicator: one
  `{ url, connected }` entry per configured Nostr relay. Empty when the Nostr
  transport is not configured.
- `boardpostoffer` — posts a listing. `give`/`get` are `coin:amount` strings;
  `t1_secs`/`t2_secs` are the two timelocks **in seconds**. `protocol` (param
  4) optionally forces `"pact-htlc-v1"` or `"pact-htlc-v2"`; omitted, the
  engine picks the default for the pair. `ttl_secs` (param 5) sets the listing
  validity; omitted, the engine default applies.
- `boardtake` — takes a posted offer by `offer_id`.
- `boardrevoke` — revokes one of your own posted offers.

> **Note** — On Nostr the listing's expiry is **rolling**:
> `min(now + 1800s, created + ttl_secs)`. It is refreshed as the offer is
> republished, not pinned to `ttl_secs` from creation.

## Private (off-market) offers

A private offer is built and signed locally but **never posted to a board**. It
travels to a counterparty as a *slip* string (paste it into your own chat).
The methods mirror the board ones, but no board is touched.

| Method | Params | Returns | Mutates |
|---|---|---|---|
| `makeprivateoffer` | `give`, `get`, `t1_secs`, `t2_secs`, `protocol?`, `ttl_secs?` | `{ slip }` | yes |
| `takeoffer` | `slip` | `{ taken }` | yes |
| `listprivateoffers` | — | `{ offers }` | no |
| `cancelprivateoffer` | `offer_id` | `{ cancelled }` | yes |

- `makeprivateoffer` — builds and signs an offer, returning the `slip` string
  to hand to a friend. As with `boardpostoffer`, `protocol` is **param 4** and
  `ttl_secs` is **param 5** (both optional).
- `takeoffer` — takes a private offer from a pasted `slip`: decodes and
  verifies the signed offer, then relays a take to the maker.
- `listprivateoffers` — your outstanding private offers.
- `cancelprivateoffer` — cancels one by `offer_id`.

## Fees

| Method | Params | Returns | Mutates |
|---|---|---|---|
| `estimateswapfees` | `give_coin`, `get_coin` | fee preview (see below) | no |

`estimateswapfees` previews the on-chain fees for a prospective swap. It takes
**only** `give_coin` and `get_coin` (coin ids).

> **Warning** — Older docs list `protocol?` and `role?` params for
> `estimateswapfees`. Those are **not parsed** — passing them has no effect.
> Legs are keyed solely off `give`/`get` (you fund what you give, redeem what
> you get).

The result shape:

```json
{
  "platform_fee_sat": 0,
  "give": {
    "coin_id": "btcx",
    "fee_rate_sat_per_vb": 5,
    "fee_rate_is_fallback": false,
    "legs": [
      { "name": "fund",   "vbytes": 0, "fee_sat": 0 },
      { "name": "refund", "vbytes": 0, "fee_sat": 0 }
    ]
  },
  "get": {
    "coin_id": "btc",
    "fee_rate_sat_per_vb": 5,
    "fee_rate_is_fallback": false,
    "legs": [
      { "name": "redeem", "vbytes": 0, "fee_sat": 0 }
    ]
  }
}
```

- `platform_fee_sat` is always `0` — the board takes nothing.
- The `give` side covers the `fund` and `refund` legs; the `get` side covers
  the `redeem` leg. Each leg reports `{ name, vbytes, fee_sat }`.
- `fee_rate_is_fallback` is true when a live fee estimate was unavailable and a
  built-in fallback rate was used.
