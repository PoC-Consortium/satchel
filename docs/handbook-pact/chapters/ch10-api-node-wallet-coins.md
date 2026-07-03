# API: Node, Seed, Merchants, Coins

This chapter documents the non-swap RPC surface: node introspection, the seed
lifecycle, the merchant model, the coin/pair registry, and the wallet helper
methods. Conventions (transport, auth, request/response shape, the *no active
merchant* error) are covered in the chapter "JSON-RPC Conventions".

## Node / info

| Method | Params | Returns | Mutates |
|---|---|---|---|
| `getinfo` | — | `{ name, version, protocol, network, identity?, seed_exists, encrypted, locked, coins, watch_only }` | no |
| `walletstatus` | — | `{ seed_exists, encrypted, locked }` | no |
| `setwatchonly` | `on` | `{ watch_only }` | yes (live + persisted) |
| `stop` | — | `"pactd stopping"` | yes (lifecycle) |

- `getinfo` — `name` is always `"pactd"`; `version` is the crate version;
  `protocol` is the swap protocol version; `network` is the lowercased network
  name (`regtest`/`testnet`/`mainnet`); `coins` is the list of configured coin
  ids; `watch_only` is the active merchant's watch-only flag (see below).
  Tolerates a missing or locked seed — `identity` is `null` until a seed is
  present **and** unlocked.
- `walletstatus` — the seed state triple. `locked` is true only when the seed
  is encrypted **and** its passphrase is not held in memory.
- `setwatchonly` — enters (`on: true`) or leaves (`on: false`) **watch-only
  mode** for the active merchant. A watch-only session may browse the board and
  withdraw its own offers, but the engine **blocks** posting, taking, and
  funding (and no-ops offer-liveness management for another session). The flag
  is persisted per-merchant in pactd's store and applied live (no relaunch);
  `getinfo.watch_only` reports it, letting a UI skip the ≥ 2-coin first-run
  gate. Returns the new value.
- `stop` — requests a graceful shutdown and returns immediately.

### Fee policy

The active merchant's local fee-bump policy — the knobs that drive funding-nurse
bumps, the v2 committed-redeem feerate, and the market-tracking refund/redeem
bumps. Both methods are scoped to the active merchant.

| Method | Params | Returns | Mutates |
|---|---|---|---|
| `getfeepolicy` | — | the policy object (below) | no |
| `setfeepolicy` | positional, all optional (below) | the full updated policy | yes (live + persisted) |

The policy object is a flat shape:

```json
{ "max_feerate_sat_vb": 500, "reservation_mult": 3, "committed_mult": 1 }
```

- `getfeepolicy` — read-only; returns the active merchant's current policy.
- `setfeepolicy` — **positional** params, all optional, in order
  `[max_feerate_sat_vb?, reservation_mult?, committed_mult?]`.
  Only the fields you supply change; the rest keep their current values. The new
  values are validated server-side, applied live, and persisted per-merchant (they
  survive a restart). Returns the full updated policy (same shape).

| Field | Default | Range | What it does |
|---|---|---|---|
| `max_feerate_sat_vb` | 500 | `1..=500` | Local ceiling on any bump's feerate (sat/vB). |
| `reservation_mult` | 3 | `1..=1000` | Funding-nurse target multiplier over the old feerate. |
| `committed_mult` | 1 | `1..=1000` | v2 committed-redeem multiplier over live market (1 = commit at market; a CPFP child lifts it if the market climbs). |

> Every spend and bump is market-derived (`target_feerate` = `min(market,
> value-at-risk, max_feerate_sat_vb)`); there is no minimum-fee floor. The former
> `min_fee_sat` and per-step `step_pct` knobs were removed.

> **Note** — The old `step_pct` escalation knob is **retired**: the unified
> bump strategy is market-tracking, not a fixed per-tick percentage step, so
> `step_pct` is no longer part of the `getfeepolicy` / `setfeepolicy` surface.
> (It survives in the on-disk struct only for serde back-compat.) These knobs
> are the *local* policy; `max_feerate_sat_vb` is distinct from the
> protocol-negotiated redeem-feerate bound. See the chapter "Fees, Fee-Bumping
> & Auto-Refund" for what each value does.

## Seed lifecycle

| Method | Params | Returns | Mutates |
|---|---|---|---|
| `createseed` | `passphrase?` | `{ mnemonic, encrypted }` | yes (writes seed) |
| `generateseed` | — | `{ mnemonic }` | no |
| `importseed` | `mnemonic`, `passphrase?` | `{ mnemonic, encrypted, identity }` | yes (writes seed) |
| `unlock` | `passphrase` | `{ unlocked, identity }` | yes (in-memory) |

- `createseed` — generates and **persists** a new BIP39 seed. The `mnemonic`
  is returned exactly once, for the user to back up. `encrypted` is true iff a
  non-empty `passphrase` was supplied.
- `generateseed` — generates a mnemonic but does **not** persist it. Used by
  the onboarding flow to preview-then-confirm a phrase before committing it
  with `importseed`.
- `importseed` — installs a supplied `mnemonic` (optionally encrypted under
  `passphrase`). Echoes the normalized phrase plus the derived `identity`
  (npub-style pubkey). Refuses to overwrite an existing seed.
- `unlock` — verifies `passphrase` by trial-decrypt and holds it in memory for
  the process lifetime, returning the derived `identity`.

> **Warning** — A persisted mnemonic is shown only once by `createseed` /
> `importseed`. Back it up immediately; there is no recovery path if it is
> lost.

### Seed-only rescue (#54)

A machine restored from the seed alone — a wiped data directory, a fresh
install — can rediscover and resume in-flight swaps it has no local record of,
from encrypted-to-self snapshots on the configured Nostr relays. See the
chapter "Seeds, Wallets & Merchants" for the full mechanics.

| Method | Params | Returns | Mutates |
|---|---|---|---|
| `restorefromrelay` | — | `{ restored, seen }` | yes (adopts records) |
| `rescuestatus` | — | `{ pending, seen, warning? }` | no |

- `restorefromrelay` — fetches this identity's rescue snapshots from the
  configured relays and adopts every one that (a) isn't already a local record
  and (b) isn't terminal. `restored` is how many were adopted; `seen` is how
  many snapshot events were fetched in total. Errors if the seed is
  locked/unreadable or no relay transport is configured.
- `rescuestatus` — the read-only preview: `pending` is how many snapshots
  `restorefromrelay` *would* adopt right now, without adopting anything.
  `warning` is present (a fixed advisory string) whenever `pending > 0`,
  cautioning that driving the same swap from two live machines on one seed can
  double-fund it.

> **Note** — `pactd` never adopts a snapshot on its own. Boot, unlock, and
> merchant-load each trigger a **read-only** rescue preview and log a warning
> if snapshots are pending, but adoption is always the explicit
> `restorefromrelay` call (or `pact-cli restore`) — call it only once the
> machine that ran those swaps is genuinely retired.

## Merchants

A *merchant* is one seed bound to one data directory — the engine's wallet
analog. The RPC surface is merchant-scoped: all swap/board/seed calls target
the **active** merchant. Nested mode (`--merchants`) lays out
`merchants/<id>/`; the flat layout has a single seed in the data-dir root.

| Method | Params | Returns | Mutates | Mode |
|---|---|---|---|---|
| `createmerchant` | `label?` | `{ id, label }` | yes | nested only |
| `listmerchants` | — | `{ merchants:[…], active }` | no | any |
| `loadmerchant` | `id` | `{ id, label }` | yes | any |
| `renamemerchant` | `id`, `label` | `{ id, label }` | yes | nested only |
| `unloadmerchant` | — | `{ unloaded }` | yes | nested only |
| `getmerchantinfo` | `id?` | merchant metadata | no | any |

- `createmerchant` — allocates the next free id (`m<N>`) and makes it active.
- `listmerchants` — each entry is
  `{ id, label, identity?, created, encrypted, active, locked }`; `active`
  names the currently selected id.
- `loadmerchant` — switches the active merchant in-process.
- `renamemerchant` — changes a merchant's user-facing `label`; the label is
  trimmed and an empty one is rejected. The label is the only mutable field
  (`id`, `identity`, and the seed are immutable), so it touches only the manifest
  with no engine reload — renaming is safe even for the active merchant mid-swap.
- `unloadmerchant` — clears the active merchant.
- `getmerchantinfo` — metadata for one merchant, defaulting to the active one.

> **Warning** — `loadmerchant` and `unloadmerchant` **refuse** to switch away
> from a merchant that has a live (non-terminal) swap, so an in-flight swap is
> never orphaned. Drive the swap to a terminal state first.

## Coins / pairs

| Method | Params | Returns | Mutates |
|---|---|---|---|
| `listcoins` | — | `{ network, coins:[…] }` | no |
| `listpairs` | — | `{ network, pairs:[…] }` | no |
| `validatecoin` | `coin_id`, `chain_data` | `{ ok, tip_height, genesis_hash? }` | no |

`listcoins` returns every coin in the shipped registry that is defined on the
active network. Each entry:

| Field | Meaning |
|---|---|
| `id` | Canonical coin id (e.g. `btcx`, `btc`, `ltc`). |
| `display_name` | Human name. |
| `symbol` | Ticker. |
| `decimals` | Smallest-unit precision. |
| `capabilities` | `{ cltv, segwit_v0, taproot }` booleans. |
| `configured` | True if a chain backend is wired for this coin. |
| `status` | Live probe: `"ok"`, `"unconfigured"`, or `"error: …"`. |
| `tip_height` | Chain tip from the probe (`null` if unconfigured/errored). |
| `genesis_hash` | Expected genesis hash for this network. |
| `bech32_hrp` | Address HRP. |
| `confirmations` | Effective confirmation depth in force. |
| `default_confirmations` | The network/spacing default depth. |

- `listpairs` — derived (never curated). Each `PairInfo` is
  `{ coin_a, coin_b, protocols, selectable?, both_configured, available }`,
  where `protocols` lists `htlc` and/or `adaptor`.
- `validatecoin` — genesis-hash checks a *proposed* backend (`chain_data`)
  before Satchel saves it. Builds an ephemeral backend; the running engine
  config is untouched.

## Wallet helpers

| Method | Params | Returns | Mutates |
|---|---|---|---|
| `getbalance` | `chain` | `{ balance_sat }` | no |
| `getnewaddress` | `chain` | `{ address }` | yes (advances HD index) |
| `sendtoaddress` | `chain`, `address`, `amount` | `{ txid }` | yes (broadcasts) |

`chain` is a coin id (e.g. `btc`). `amount` for `sendtoaddress` is a decimal
string in whole coin units. `getnewaddress` advances the HD derivation index;
`sendtoaddress` constructs and broadcasts a payment.
