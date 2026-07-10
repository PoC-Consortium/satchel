# API: v1 HTLC Swaps

This chapter documents the **v1 (HTLC)** swap RPCs. These drive a classic
hashlock/timelock atomic swap between two coins. The cross-chain mechanics —
why two timelocks, what each transaction does — are covered in the protocol
chapters; here we document the RPC surface only. For shared conventions see the
chapter "JSON-RPC Conventions". For v2 (Taproot/MuSig2 adaptor) swaps see the
chapter "API: v2 Adaptor Swaps".

> **Note** — *v1 (HTLC) and v2 (Taproot/MuSig2 adaptor) swaps are reviewed
> and live on mainnet.*

## Swap state machine

A v1 swap is a `SwapRecord` whose `state` advances through this enum
(snake_case in JSON):

```text
created → accepted → funded_a → funded_b → redeemed_b → completed
```

| State | Meaning |
|---|---|
| `created` | Offer created by the initiator; no on-chain action yet. |
| `accepted` | Counterparty accepted; both sides committed to terms. |
| `funded_a` | The first HTLC leg is funded on-chain. |
| `funded_b` | The second HTLC leg is funded on-chain. |
| `redeemed_b` | The B-side HTLC has been redeemed (secret revealed). |
| `completed` | Both legs settled; swap finished successfully. |

Terminal failure states:

| State | Meaning |
|---|---|
| `refunded` | A funded leg timed out and was refunded. |
| `aborted` | Swap cancelled before our HTLC funded. |

## The `coin:amount` convention

`give` and `get` are strings of the form `coin:amount`, where `coin` is a
registry coin id and `amount` is a decimal in whole coin units — for example
`"btcx:1.0"` and `"btc:0.5"`. `t1`/`t2` are the two relative timelocks in
blocks (initiator and counterparty legs respectively).

## Read methods

| Method | Params | Returns | Mutates |
|---|---|---|---|
| `listswaps` | — | `[SwapRecord]` | no |
| `getswap` | `swap_id` | one `SwapRecord` | no |
| `listpendingtakes` | — | outstanding takes awaiting maker init | no |
| `listmyoffers` | — | `[{ offer_id, offer, state, created, valid_for, current_expiry, final_expiry, now }]` | no |

- `listswaps` — every v1 swap record for the active merchant. Each entry
  carries `source` — `"local"` (driven by this machine: own scope, or taken
  over) or `"foreign"` (another machine's swap on the same seed, followed
  read-only) — and `machine_label`, the short one-way label of the record's
  originating machine (for foreign swaps, which machine owns it). Both are
  computed server-side; the raw derive scope is never exposed.
- `getswap` — a single record by `swap_id`.
- `listpendingtakes` — takes that have arrived but for which the maker has not
  yet initiated a swap (no `SwapRecord` exists yet — the UI's "initiating"
  pre-swaps).
- `listmyoffers` — the maker's own offers (the My-offers view). `current_expiry`
  is the rolling expiry (last refresh + relay TTL, capped at `final_expiry`);
  `final_expiry` is the maker-set hard expiry (`created + valid_for`); `now` is
  the server timestamp for client-side countdown rendering.

## Lifecycle methods

| Method | Params | Returns | Mutates | Purpose |
|---|---|---|---|---|
| `offer` | `give`, `get`, `t1`, `t2` | `{ record, envelope }` | yes | Start a swap as initiator. |
| `acceptoffer` | `envelope` | `{ record, envelope }` | yes | Accept a received offer envelope. |
| `recv` | `envelope` | `{ record }` | yes | Ingest a counterparty reply envelope. |
| `fund` | `swap_id` | `{ record, envelope }` | yes (broadcasts) | Broadcast our HTLC funding tx. |
| `redeem` | `swap_id` | `{ record }` | yes (broadcasts) | Redeem the counterparty HTLC (reveals secret). |
| `refund` | `swap_id` | `{ record }` | yes (broadcasts) | Reclaim our funded HTLC after timeout. |
| `abort` | `swap_id`, `reason?` | `{ record }` or `{ cancelled_pending_take }` | yes | Cancel an unfunded swap, a v2 adaptor swap, or an unanswered pending take. |
| `takeover` | `swap_id` | `{ taken_over, swap_id }` | yes | Adopt another machine's swap (same seed). |
| `tick` | — | `{ events:[…] }` | yes | Advance the scheduler one pass. |

- `offer` — initiates a swap with the given terms and returns the signed
  `envelope` to hand to the counterparty.
- `acceptoffer` — accepts a counterparty's offer `envelope`, returning a reply
  `envelope`.
- `recv` — ingests a counterparty envelope (e.g. an acceptance reply).
- `fund` — builds and broadcasts our HTLC funding transaction, then **relays the
  `funded` envelope to the counterparty** (via the engine's notify path), so a
  manually or hand-recovered swap notifies the maker exactly like the automatic
  auto-fund path does. (Previously the RPC just returned the envelope without
  relaying it.)
- `redeem` — spends the counterparty's funded HTLC, revealing the preimage.
- `refund` — reclaims our own funded HTLC once its timelock has expired.
- `abort` — cancels the given `swap_id`; `reason` defaults to `"user aborted"`.
  **One method covers every in-flight card the UI shows**: it looks up `swap_id`
  as a v1 record first, then a v2 (Taproot) adaptor record, and — if neither
  exists — treats it as an *offer id* for a still-unanswered pending take
  (`listpendingtakes`) and cancels that instead, returning
  `{ cancelled_pending_take: <offer_id> }` rather than a record. A cancelled
  pending take best-effort notifies the maker with an `abort` envelope keyed by
  the offer id, so the maker's served-offer marker resolves it even though no
  `SwapRecord` ever existed on that side.
- `takeover` — starts driving a *followed* swap on this machine: another
  machine's swap on the same seed, or this machine's own after a
  scope-rotating re-import. Only call it once that machine is confirmed
  stopped — two live drivers can double-fund a swap. Dispatch covers v1 and v2
  ids alike. See "One seed on more than one machine" in the chapter "Seeds,
  Wallets & Merchants".
- `tick` — runs one scheduler pass (board sync + engine tick) and returns the
  resulting `events`, each `{ swap_id, action, detail }`.

> **Warning** — `abort` is **refused once our HTLC has funded** (v1) or once
> our leg is funded (v2). After funding, the only safe exits are `redeem` (if
> you can claim the counterparty's leg) or `refund` (after your timelock
> expires). Aborting a funded swap is not an option because the coins are
> already committed on-chain.

> **Note** — A v2 handshake that stalls before either leg is funded — stuck at
> `created`, `accepted`, or `nonces_exchanged` — also **self-aborts after 15
> minutes** (`PRE_FUNDING_TIMEOUT_SECS`) without any `abort` call or relay
> message: both sides' schedulers independently time out their own copy of the
> stalled handshake. `signed` is excluded from this auto-timeout, since funding
> may already be in flight by then. See the chapter "API: v2 Adaptor Swaps".

> **Note** — The `funded` envelope `fund` relays is an *accelerator*, not a
> requirement. Even if the relay message never reaches the maker, the swap still
> completes via chain-watching: both sides detect each other's legs on-chain and
> drive to redemption regardless. The relayed `funded` / `redeemed` messages only
> shave latency off that chain-watched path.

## Diagnostics

| Method | Params | Returns | Mutates |
|---|---|---|---|
| `dumpswap` | `swap_id` | `{ swap_id, pactd_version, record, log }` | no |

- `dumpswap` — returns a developer-shareable diagnostics bundle for one swap:
  its current `record` (the v1 `SwapRecord`, or the v2 `AdaptorSwapRecord` if the
  id is a v2 swap) plus `log`, the array of `pactd` log lines that mention that
  `swap_id` (the scheduler tags every event with `swap=<id>`). `pactd_version` is
  the engine's crate version. The dispatch tries the v1 store first, then the v2
  adaptor store, and — if `swap_id` matches neither — falls back to a pending
  take by offer id, so an "initiating" pre-swap that never became a `SwapRecord`
  can still be dumped for support.

> **Note** — `dumpswap` is **secret-safe by construction**. The record is passed
> through `scrub_secrets`, which redacts the v1 preimage and any secret-named
> field; the v2 adaptor record stores no secret (`t` is never persisted); and
> seeds, passphrases, and MuSig2 nonces never appear in a record or in the log.
> The bundle is safe to paste into a bug report. It backs Satchel's per-swap
> **Dump logs** button.
