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
| `revokeoffersforcoin` | `coin_id` | `{ revoked }` | yes |

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
- `revokeoffersforcoin` — withdraws **every** live offer whose pair involves
  `coin_id`, across all boards. Satchel calls this before removing or
  reconfiguring a coin, while `pactd` still has it configured, so the offers are
  cleanly de-listed rather than orphaned (#97); the surviving offers then ride
  the skip-de-list relaunch.

> **Note** — **Cumulative funds gate.** `boardpostoffer` rejects an offer the
> core wallet could not fund — and "fund" means the **sum** of the give-leg
> amounts across **all** your still-live offers in that coin, not just this one
> offer, plus per-offer funding-fee headroom (`Engine::ensure_can_fund_new_offer`
> → `committed_give_for_coin`). Because each offer is funded only when *taken*,
> advertising several offers whose give-legs together exceed your balance would
> let two of them be taken and only one funded; the gate stops that at post
> time. It is a best-effort pre-flight — in-flight takes are not subtracted and
> balances are not netted across coins. On failure the maker sees:
>
> ```text
> insufficient {coin_id} balance to advertise this offer: have {balance} sat,
> need ~{needed} sat ({amount} for this offer + {committed} already committed
> across {n_live} live offer(s) + ~funding-fee headroom). Withdraw or let some
> offers expire first.
> ```

> **Note** — On Nostr the listing's expiry is **rolling**:
> `min(now + 1800s, created + ttl_secs)`. It is refreshed as the offer is
> republished, not pinned to `ttl_secs` from creation.

> **Note** — **Wire epochs (rc10):** the signed offer body and the `take`
> envelope both carry `wire` — the protocol family's wire-compatibility epoch
> (v1 = 1, v2 = 2; **absent parses as 1**, the pre-rc10 era). The taker
> refuses an offer whose `wire` differs from what its build speaks
> (`boardtake` errors up-front), and the maker refuses a mismatched take with
> a `take-rejected` reason ("incompatible release") while the offer stays
> live. Handshake bodies (`init`/`accept`, both protocols) carry and validate
> the same field. Bump the epoch for any wire-breaking protocol amendment —
> mixed-epoch peers then gate cleanly instead of failing mid-handshake.

> **Warning** — **Breaking wire change:** the `take` envelope now carries a
> signed `taken_at` timestamp, and this field is **REQUIRED** — a `take` from a
> pre-rc8 build has none and is treated as stale (age saturates to "very old"),
> so it is silently dropped rather than served. Both parties must run rc8+ for
> `boardtake` to work. This closes a mainnet incident where a take that had sat
> queued for hours (e.g. the maker's node was unreachable) was served anyway
> after the taker's own pending-take entry had already self-pruned — burning the
> offer via revoke-on-commit and stranding an uncancelable record on a
> counterparty nobody was driving. The maker now compares `taken_at` against its
> own `PRE_FUNDING_TIMEOUT_SECS` (15 minutes) **before** serving or revoking the
> offer: a take older than that is dropped with no reply (the taker's own card
> pruned itself long ago, so nothing is listening), and the offer stays live for
> the next taker. A future `taken_at` (clock skew) saturates to age zero rather
> than being rejected, so ordinary clock drift within the window is tolerated.

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
