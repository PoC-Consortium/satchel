# Satchel backend surface

The backend Satchel codes against: the `pactd` JSON-RPC surface plus the
corkboard HTTP API. Satchel is a thin client — it holds no swap logic and (with
merchants owned by pactd) persists no trading state of its own; everything below
is computed and owned by `pactd`/corkboard.

Companion docs: [SATCHEL_UI.md](SATCHEL_UI.md) (the UI that consumes this),
[ARCHITECTURE.md](ARCHITECTURE.md) (design), the protocol spec in
[../spec/](../spec/).

## Load-bearing constraints

These are not implementation details — they are the product:

- **Non-custodial.** Keys never leave the machine. pactd's seed holds only hot
  transit keys for in-flight swaps; proceeds sweep to the user's own wallet.
- **The corkboard never matches, executes, custodies, or charges.** It is a
  content-blind bulletin: signed offers + a store-and-forward relay. No order
  matching, no execution, no fees, no accounts. `platform_fee_sat` is wired to
  `0` everywhere it appears.
- **The board observing takes is a non-goal.** A `take` is relayed peer-to-peer;
  the board never learns who took an offer. Board-side take tracking would leak
  taker intent and make the board stateful about matches — matching-engine drift
  the MiCA position forbids. Offers disappear by maker-signed revocation or TTL
  expiry, never by the board observing a match (see *Offer lifecycle*).

---

## Daemon identity & wallet state

`getinfo` → `{ name, version, protocol, network, identity, seed_exists,
encrypted, locked, coins }`. Tolerates a missing/locked seed (a fresh or locked
merchant has no `identity` yet) so the UI can drive the first-run wizard and
unlock prompt. `identity` is the BIP340 x-only pubkey (hex) of the active
merchant's seed.

`walletstatus` → the seed lifecycle flags (`seed_exists`, `encrypted`,
`locked`). Seed provisioning: `createseed { passphrase? }` (returns the mnemonic
once), `importseed { mnemonic, passphrase? }`, `unlock { passphrase }`. Each
returns the derived `identity` when the seed is readable.

## Merchants (owned by pactd)

pactd owns the merchant registry, mirroring how `bitcoind` owns wallets. A
*merchant* is one seed = one trading identity = one data dir. pactd is launched
at a parent data dir and owns a `merchants/<id>/` subtree plus a `merchants.json`
manifest. Phase 1: one active merchant is loaded at a time and switched
in-process (no relaunch). The RPC surface is merchant-scoped-ready so concurrent
multi-merchant loading is a future internal change, not an API break.

- `createmerchant { label }` → `{ id, label }` — allocates the next free `m<N>`
  id (skipping any orphaned on-disk dir, so the registry/disk desync class can't
  recur), creates its data dir, makes it active. The seed is provisioned
  afterwards via `createseed`/`importseed` against the now-active merchant.
- `listmerchants` → `{ merchants: [{ id, label, identity, created, encrypted,
  active, locked }], active }`.
- `loadmerchant { id }` / `unloadmerchant` — switch / drop the active merchant.
  **Fund-safety gate:** refuses to switch away from a merchant that has a live
  (non-terminal) swap, so its timelocks keep being watched.
- `getmerchantinfo { id? }` → metadata for one merchant (defaults to active);
  same fields as a `listmerchants` row.

A flat/legacy layout (seed in the data-dir root, as the e2e harness and
`pact-cli` use it) is preserved: the data dir itself is a single synthetic
`default` merchant, and `createmerchant` is refused.

Identity and label are non-secret metadata held in the manifest; secrets stay in
pactd's per-merchant store.

## Coins & pairs

`listcoins` → `{ network, coins: [...] }`. Each shipped coin carries a
**live-probed** `status` (`"ok"` | `"unconfigured"` | `"error: <reason>"`) and a
live `tip_height` (a genesis check + tip query per configured coin), plus
`id`, `display_name`, `symbol`, `decimals`, `capabilities`, `configured`,
`genesis_hash`, `bech32_hrp`. The UI drives the per-coin health glyph from
`status` + `tip_height` directly; no extra backend call is needed.

`listpairs` → derived swappable pairs for the current setup (not a curated
list). `validatecoin { coin_id, chain_data }` → genesis-hash check of a proposed
backend before Satchel saves it; builds an ephemeral backend and leaves the
engine config untouched.

## Wallet

`getbalance { chain }` → `{ balance_sat }`. `getnewaddress { chain }` →
`{ address }`. `sendtoaddress { chain, address, amount }` → `{ txid }`.

The node-backed Satchel UI consumes **only `getbalance`** — the Wallets screen
is read-only (the balance is the node's own core wallet, so send/receive would
duplicate the node's wallet). `getnewaddress` / `sendtoaddress` remain on the
pactd surface for the CLI and for the future nodeless (bdk + Electrum) wallet,
which is where Satchel grows a real send/receive UI.

## Swaps

`listswaps` → an array of swap records; `getswap { swap_id }` → one. The record
(`SwapRecord`) carries everything the Swaps history and live-swaps indicator
need:

- `swap_id`, `role` (initiator/participant), `state`
- `created_at` (unix seconds, stamped on insert; surfaced for time-ordering)
- `counterparty_identity` (pinned from the counterparty's first message)
- `chain_a` / `chain_b`, `amount_a` / `amount_b`
- `t1` / `t2` (timelocks), `hash_h`
- HTLC outpoints (`htlc_a_txid`/`vout`, `htlc_b_txid`/`vout`) and the spend
  `final_txid`

The `state` enum is `Created → Accepted → FundedA → FundedB → RedeemedB →
Completed`, plus the terminal `Refunded` / `Aborted`. **Active** = any
non-terminal state; **terminal** = `Completed` / `Refunded` / `Aborted`.

`fund` / `redeem` / `refund` / `abort` drive a swap; `tick` advances the
scheduler (timelock watching, rebroadcast). v2 (Taproot/MuSig2 adaptor) swaps
live in their own table, surfaced via `listadaptorswaps` with an analogous
record.

### Fee preview

`estimateswapfees { give_coin, get_coin, protocol?, role? }` exposes the
internal fee estimation (live `estimatefee(6)` per chain, fallback 10 sat/vB,
clamp 1–500). `protocol`/`role` are accepted for forward-compat but do not
change today's HTLC legs. As-built shape:

```
{
  platform_fee_sat: 0,                 // ALWAYS 0 — Corkboard takes nothing
  give: { coin_id, fee_rate_sat_per_vb, fee_rate_is_fallback, legs: [
            { name: "fund",   vbytes, fee_sat },
            { name: "refund", vbytes, fee_sat }   // unhappy-path alternative
          ] },
  get:  { coin_id, fee_rate_sat_per_vb, fee_rate_is_fallback, legs: [
            { name: "redeem", vbytes, fee_sat }
          ] }
}
```

On the happy path the user pays the **fund** leg on the give-chain plus the
**redeem** leg on the get-chain; **refund** replaces redeem on timeout.
`fee_rate_is_fallback` is per-side so the UI can flag a guessed rate when a
coin's node is unreachable.

## Offers

### Board offers

- `boardpostoffer { give, get, t1_secs, t2_secs, protocol? }` → `{ offer_id }`.
- `boardlistoffers { board? }` → `{ offers }`. Each envelope carries `created`
  + `ttl_secs` and a `revoked` flag. The UI derives state client-side:
  `open` = `!revoked && created + ttl_secs > now`, else `revoked` / `expired`.
- `boardtake { offer_id }` → relays a `take` to the maker.
- `boardrevoke { offer_id }` → posts a signed revocation.

Corkboard HTTP API (what pactd speaks to the board):

```
POST /v1/offers           signed offer envelope
GET  /v1/offers           list active offers (filters: give/get/network);
                          server-side filter is revoked = 0 AND not expired
POST /v1/offers/revoke    signed revocation (same identity)
POST /v1/relay            {to, blob} — store-and-forward, content-blind
POST /v1/relay/poll       signed poll → messages since cursor
```

Offer TTL defaults to 24h, capped at one week.

### Offer lifecycle

Surfaced offer states are **open / revoked / expired**, plus the locally
knowable **taken-by-us** (correlate a local pending-take to a `listswaps` row by
counterparty identity). Two mechanisms remove an offer:

- **Maker auto-revoke-on-commit.** When pactd (as maker) commits to a swap for
  an offer (sends `init`), it auto-posts the signed `boardrevoke` for that
  offer. The listing disappears for everyone, shown as "withdrawn", never "taken
  by X" — the board never learns who took it. Best-effort: even if the revoke
  fails, the maker's local `offer_served`/`offer_revoked` guards reject any late
  take.
- **Take / handshake timeout.** The taker stamps each pending take with a
  timestamp; the scheduler prunes a take the maker never answered, and a swap
  stalled pre-funding in `Created`/`Accepted` auto-aborts, after
  `PRE_FUNDING_TIMEOUT_SECS` (15 min). Nothing is locked pre-funding, so this is
  cleanup, not fund recovery. (Rows migrated from before this field read as
  ancient and are pruned on the first tick after upgrade.)

Two concurrent takes with the **same maker** are kept distinct: the maker echoes
the originating `offer_id` in the `init` body, and the taker matches the init to
the exact pending take by that id (falling back to identity match for boardless
or older inits).

### Private (off-market) offers

`makeprivateoffer` / `takeoffer` / `listprivateoffers` / `cancelprivateoffer`
mirror the board calls but never touch a board: the maker's signed offer travels
to a friend as a slip string over their own chat. See
[PRIVATE_OFFERS.md](PRIVATE_OFFERS.md).

---

## Open backend items

> **TODO:** C7 GitHub release polling — the version indicator ships as a static
> placeholder (`UPDATE_AVAILABLE = false`); a live update check against the
> GitHub releases API is unbuilt.

> **TODO:** C9 corkboard liveness / stale-offer cleanup — offers expire on TTL
> and honor revocation, but there is no maker-heartbeat or signed
> remaining-count ("tear-off tabs") so a dead maker's offer decays before its
> TTL. Any design must keep the board a dumb bulletin: availability state must be
> maker-signed (or freshness-derived), never inferred from board-observed
> takes.

> **TODO:** C12 backup / restore — the mnemonic restores identity (deterministic
> key derivation), not swap history or in-flight state, which live only in
> pactd's local SQLite store. So after a seed import on a new machine the Swaps
> page is empty and an in-flight swap cannot be continued. Options under
> consideration: backing up pactd's store alongside the mnemonic; a chain-rescan
> that rebuilds completed-swap history; or an in-flight failover via a
> self-sealed recovery blob on the existing relay (failover only — two pactd on
> one seed must not act concurrently, and a standby must act within the T1/T2
> window). Until built: the mnemonic is trust continuity, not operational
> continuity.
