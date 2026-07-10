# API: Node, Seed, Merchants, Coins

This chapter documents the non-swap RPC surface: node introspection, the seed
lifecycle, the merchant model, the coin/pair registry, and the wallet helper
methods. Conventions (transport, auth, request/response shape, the *no active
merchant* error) are covered in the chapter "JSON-RPC Conventions".

## Node / info

| Method | Params | Returns | Mutates |
|---|---|---|---|
| `getinfo` | — | `{ name, version, protocol, wire_epochs, network, identity?, seed_exists, encrypted, locked, needs_reimport, machine_label, coins }` | no |
| `walletstatus` | — | `{ seed_exists, encrypted, locked, needs_reimport }` | no |
| `help` | `method?` | plain-text catalog (string) | no |
| `listmethods` | — | `[name, …]` | no |
| `stop` | — | `"pactd stopping"` | yes (lifecycle) |

- `getinfo` — `name` is always `"pactd"`; `version` is the crate version;
  `protocol` is the swap protocol version; `wire_epochs` maps each protocol
  family to the wire-compatibility epoch this build speaks (current, after the
  rc12-recut per-side-confirmations bump:
  `{ "pact-htlc-v1": 2, "pact-htlc-v2": 3 }`; rc10 spoke `1`/`2`) — a UI
  badges offers whose
  signed `wire` differs as un-takeable; `network` is the lowercased network
  name (`regtest`/`testnet`/`mainnet`); `coins` is the list of configured coin
  ids. Tolerates a missing or locked seed — `identity` is `null` until a seed is
  present **and** unlocked. `machine_label` is the short one-way label of this
  install's derive scope (e.g. `"M-7f3a"` — never the raw scope);
  `needs_reimport` is true when the seed file exists but its OS-keystore key is
  unavailable, so the recovery phrase must be re-imported (see the chapter
  "Seeds, Wallets & Merchants").
- `walletstatus` — the seed state triple. `locked` is true only when the seed
  is encrypted **and** its passphrase is not held in memory. `needs_reimport`
  as in `getinfo`.
- `help` — with no param, the daemon's full method catalog grouped by
  category, rendered as plain text (the CLI prints string results raw, so it
  reads like a man page); with a `method` name, that one method's arguments
  and summary. This catalog is the authoritative live method list — the same
  one that drives the CLI's *did-you-mean* suggestion.
- `listmethods` — the same catalog as a machine-readable JSON array of method
  names.
- `stop` — requests a graceful shutdown and returns immediately.

### Fee policy

The active merchant's local fee-bump policy — the knobs that drive funding-nurse
bumps and the market-tracking refund/redeem bumps. Both methods are scoped to
the active merchant.

| Method | Params | Returns | Mutates |
|---|---|---|---|
| `getfeepolicy` | — | the policy object (below) | no |
| `setfeepolicy` | positional, all optional (below) | the full updated policy | yes (live + persisted) |

The policy object is a flat shape:

```json
{ "max_feerate_sat_vb": 500, "reservation_mult": 3 }
```

- `getfeepolicy` — read-only; returns the active merchant's current policy.
- `setfeepolicy` — **positional** params, all optional, in order
  `[max_feerate_sat_vb?, reservation_mult?]`.
  Only the fields you supply change; the rest keep their current values. The new
  values are validated server-side, applied live, and persisted per-merchant (they
  survive a restart). Returns the full updated policy (same shape).

| Field | Default | Range | What it does |
|---|---|---|---|
| `max_feerate_sat_vb` | 500 | `1..=500` | Local ceiling on **funding** bumps' feerate (sat/vB); redeem/refund bumps ignore it (value-capped instead). |
| `reservation_mult` | 3 | `1..=1000` | Funding-nurse target multiplier over the old feerate. |

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
| `serverstatus` | `coin_id` | `{ servers:[…] }` | no |

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
| `nodeless` | True when the coin runs **nodeless** (Electrum-only backend list; the wallet is the Pact seed's bdk wallet). This is the field a UI keys its send/receive/activity surface off. |
| `wallet` | The Core wallet name the coin's RPC is scoped to, parsed from the configured URL (`/wallet/<name>`); `null` when none is set (node default wallet, or a nodeless coin). |
| `status` | Live probe: `"ok"`, `"unconfigured"`, or `"error: …"`. |
| `tip_height` | Chain tip from the probe (`null` if unconfigured/errored). |
| `genesis_hash` | Expected genesis hash for this network. |
| `bech32_hrp` | Address HRP. |
| `confirmations` | Effective confirmation depth in force. |
| `default_confirmations` | The network/spacing default depth. |
| `servers_total` | *Nodeless only.* Count of configured Electrum servers. |
| `servers_healthy` | *Nodeless only.* Count currently reachable and serving the right chain. |
| `servers_down` | *Nodeless only.* Count in failure-backoff right now. |
| `wallet_server_state` | *Nodeless only.* State of the **elected wallet-home** server: `"healthy"`, `"down"`, or `"untested"`. |
| `wallet_synced_secs_ago` | *Nodeless only.* Seconds since the wallet cache was last confirmed against its server — the "balance as of" staleness signal. |

The five server fields come from the passive health registry (never a probe) —
see "Multiple Electrum servers & failover" in the chapter "Configuring Coins"
for the active-set model behind them, and `serverstatus` below for the
per-server detail.

- `listpairs` — derived (never curated). Each `PairInfo` is
  `{ coin_a, coin_b, protocols, selectable?, both_configured, available }`,
  where `protocols` lists `htlc` and/or `adaptor`.
- `validatecoin` — genesis-hash checks a *proposed* backend (`chain_data`)
  before Satchel saves it. Builds an ephemeral backend; the running engine
  config is untouched.
- `serverstatus` — per-server Electrum health for one **nodeless** `coin_id`,
  a **pure in-memory read** of the health cells that real traffic feeds. It
  **never dials or probes** (the Network monitor polls it every few seconds),
  so a server is only ever reported `down` because a genuine request to it
  failed — not because opening this page touched it. Each `servers[]` row:

| Field | Meaning |
|---|---|
| `url` | The server URL (`tcp://…` / `ssl://…`). |
| `state` | `"healthy"`, `"down"` (in failure-backoff), or `"untested"` (a configured server never yet needed — a cold standby). |
| `role` | `"wallet"` (the elected home), `"view"` (an active cross-check), or `"standby"` (configured but idle); absent until the coin routes this run. |
| `retry_in_secs` | When `down`: seconds until the backoff window reopens the server for use (`0` = eligible now). |
| `latency_ms` | Smoothed request latency. |
| `requests` / `failures` | Lifetime counters for this run. |
| `last_error` / `last_error_secs_ago` | Most recent failure text and its age, when any. |

## Wallet helpers

| Method | Params | Returns | Mutates |
|---|---|---|---|
| `getbalance` | `chain` | `{ balance_sat }` | no |
| `getnewaddress` | `chain` | `{ address }` | yes (advances HD index) |
| `estimatesendfee` | `chain` | `{ min_sat_per_vb, fast, normal, slow }` | no |
| `sendtoaddress` | `chain`, `address`, `amount`, `conf_target?`, `fee_rate?` | `{ txid }` | yes (broadcasts) |
| `bumpfee` | `chain`, `txid`, `fee_rate` | `{ txid }` | yes (broadcasts replacement) |
| `listtransactions` | `chain` | `{ transactions: [...] }` | no |

`chain` is a coin id (e.g. `btc`). `amount` for `sendtoaddress` is a decimal
string in whole coin units — or the literal string `"all"`, which **sweeps
the wallet**: the fee comes out of the swept amount (Core-RPC via
`subtractfeefromamount`; bdk via `drain_wallet`), so the recipient receives
balance − fee and the wallet ends empty. UTXOs reserved by a
built-but-unbroadcast v2 funding are not spendable, so a sweep can never claw
back a reservation. `getnewaddress` advances the HD derivation index (on a
nodeless coin the handout is capped — see the chapter "Coins, Pairs &
Capabilities");
`sendtoaddress` constructs and broadcasts a payment, always BIP125-replaceable
(sweeps included).
The fee is priced by `fee_rate` (explicit **decimal** sat/vB, e.g. `1.08` —
the send form's Custom field) when given, else by a market estimate at
`conf_target` blocks (default 6, the Normal preset; Slow/Fast are 144/1),
floored to the coin's `min_feerate_sat_vb` with the usual 1 sat/vB fallback;
both fee params apply to a sweep as usual. Rates travel internally at the
estimator's native **sat/kvB** resolution, so the fraction is actually paid —
at the bottom of the market it is real queue priority (1.08 confirms ahead
of 1.00), not display sugar.

`estimatesendfee` backs the send form's fee presets: raw estimator answers as
decimal sat/vB at full sat/kvB resolution (e.g. `1.08`) at the 1/6/144-block
targets — `null` where the estimator has no data
(fresh chain, quiet mempool, regtest), which the form maps to disabled presets
and a custom-rate fallback at the coin floor (`min_sat_per_vb`), mirroring
phoenix's send dialog.

`bumpfee` RBF-replaces an unconfirmed wallet send at the given `fee_rate`
(sat/vB — must beat what the tx pays now plus the incremental-relay margin the
wallet enforces). Satchel offers it on pending sent rows of the Activity
dialog for nodeless coins; a node-backed wallet is bumped with the node's own
tooling instead. A txid that **funds a live swap** is refused — v1 HTLC and
v2 funding txids alike — with
`<txid> funds live swap <id> — the swap engine manages its fee (see
get/setfeepolicy), bumpfee must not replace it`. Those fees belong to the
funding nurse: v1 fundings are re-RBF'd under the swap's own fee policy, and
a v2 funding is deliberately CPFP'd, because replacing it would change its
txid and invalidate the pre-signed MuSig2 redeems (see the chapter "Fees,
Fee-Bumping & Auto-Refund").

`listtransactions` serves the activity feed of an **Electrum-connected
(nodeless) coin** — each entry carries `txid`, `direction` (`"sent"` /
`"received"`), the net `amount_sat` (fee excluded on sends), `fee_sat` (absent
when the wallet doesn't own every input), `vsize` (with `fee_sat` this yields
the feerate a bump has to beat), `confirmations`, and `timestamp` (block time;
first-seen for mempool entries; absent for a built-but-unreleased v2 funding).
A self-transfer nets to `"sent"` with `amount_sat` 0. Newest first.
Node-backed coins refuse (`wallet activity requires a nodeless
(Electrum-backed) coin`) — the node wallet keeps its own history.
