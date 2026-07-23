# Network Support, Reorgs & Safety

This chapter states, precisely, where Pact's swap protocols stand on safety:
which versions run on which networks, how the engine picks a protocol, how
confirmation depth defends against reorgs, and the safety properties that hold
in every swap. Both swap protocols — v1 (HTLC) and v2 (Taproot/MuSig2 adaptor) —
are reviewed and live on mainnet. The point of this chapter is to make
the safety posture legible.

## Network support

Both protocols run on every network — regtest, testnet, and mainnet:

| Protocol | State |
|---|---|
| v1 (`pact-htlc-v1`) | Live on every network, including mainnet |
| v2 (`pact-htlc-v2`) | Live on every network, including mainnet |

In the code, `ADAPTOR_BUILT = true` (`registry.rs:53`) and
`ADAPTOR_MAINNET_ENABLED = true` (`registry.rs:59`) together make
`adaptor_allowed` return `true` on every network; the test suite asserts mainnet
is allowed. v1 has no network restriction.

> **Note** — Some older design docs and a few stale source comments still
> describe v2 as "refused on mainnet" or "not built". Those are out of date. The
> shipped code runs v2 on every network. Treat the registry constants
> (`ADAPTOR_BUILT`, `ADAPTOR_MAINNET_ENABLED`) as the source of truth.

## Network isolation: the foreign-pactd guard

Each network runs `pactd` on its own JSON-RPC port — **regtest 9739, testnet
9738, mainnet 9737** — so a mainnet and a regtest install never share a daemon.
Before the launcher adopts a `pactd` already listening on the configured
`listen` port, it confirms the daemon is genuinely *ours*: it reads the data-dir
`.cookie`, calls `getinfo`, and checks both that the cookie authenticates **and**
that `getinfo.network` matches the active network (`probe_adoptable`,
`satchel/src/main.rs:472`).

If something is healthy on the port but is **not** this install's engine — a
different network's daemon, or one whose cookie is unreadable — the launcher
**refuses to start** rather than silently latching onto the wrong daemon (which
would point every RPC at another network's chains) or starting with empty auth.
It fails loud with an actionable error (`satchel/src/main.rs:1251`):

```text
configured listen <listen> is already serving a different engine (not this
<network> install) — another network's Satchel/pactd is using that port. Set a
distinct `listen` in satchel.json (regtest uses 9739, testnet 9738, mainnet
9737), or stop the other instance, then relaunch.
```

The usual trigger is a stale or copied `satchel.json` whose `listen` still
points at another network's port. The remedy is to give each network a distinct
`listen` (the defaults above already are) or stop the colliding instance.

## Multiple machines on one seed

Running one BIP39 seed on more than one machine (failover, standby, recovery)
is safe. There is no lease, heartbeat, or passive mode — every session is
active; safety comes from **partitioning**, not from electing an owner:

- **Derive scope.** Each install holds a random per-install derive scope
  (`machine.json` at the data-dir root), injected into every initiator
  (counter-based) derivation — two machines on one seed derive different
  secrets and swap ids at the same counter (see the chapter "Seeds, Wallets &
  Merchants").
- **Take gate.** A maker serves a take only if this machine owns the offer;
  another machine on the same seed ignores it silently. Symmetrically, a
  maker's `init` arriving at a standby that holds no matching pending take is
  expected fan-out noise — the pending take lives only in the driving
  machine's DB — and is consumed quietly as an `"init-ignored"` tick event,
  not logged as a handshake failure.
- **Drive rule.** A record is driven only if its scope matches this machine
  (or it was explicitly adopted). Another machine's swap shows as **followed**
  — read-only chain monitoring, never broadcast; the engine funnels every
  broadcast through one belt that refuses followed records.
- **Takeover.** If a machine dies, another adopts its swaps behind one
  explicit confirm ("confirm that machine is stopped") — the `takeover` RPC or
  the dock's "Take over" button. Takeover never skips a swap. A v2 record
  whose pinned cooperative-redeem sweep address is not owned by this machine's
  wallet is adopted anyway, with a warning, as **refund-only**: a refund pays a
  fresh owned address and needs no sweep custody, so the payout check
  (`v2_owns_redeem_payout`, gating both `adaptor_redeem` call sites) refuses
  only the cooperative *completion* to the foreign wallet and rides the swap
  to its timelock refund. v1 pins no destination, and a sweep-unset v2 falls
  back to a seed-derived address, so those adopt with full redeem capability.
- **Reconcile before drive.** On every restart, a driven swap first derives
  its true status from chain before its drive arms act — the same
  classification and depth gate the follower uses (`reconcile_driven_v1`/`_v2`,
  `engine.rs:7113/7237`) — so a resumed driver never drives forward from
  persisted state alone. A swap already settled elsewhere (say, by a same-seed
  standby while the owner was down) is written to its true terminal state
  locally with **zero broadcasts**: chain pointers are adopted and the
  settling spend becomes the record's `final_tx` so the confirmation nurse
  converges on it. Cadence is once per driven swap per process start,
  re-armed by the drive arms whenever a tracked leg unexpectedly vanishes
  (`request_reconcile`). A terminal is written only when our settlement leg is
  spent deep at its own confirmation target — a shallow spend keeps the record
  driving, so a reorg cannot fake a completion. Each reconciliation surfaces
  as a `"reconciled"` tick event ("chain truth → {State}: …").
- **Upgrade path.** Pre-upgrade records carry the legacy scope. Terminal
  legacy records are auto-claimed as this machine's history on the first tick
  (they stay in the Swaps ledger); active legacy swaps appear under "Another
  machine" and need one Take-over confirm. Legacy records are never
  auto-purged.

> **Warning** — Concurrency is **safe, not coherent**: N machines on one seed
> share ONE wallet balance — N interchangeable drivers, not N× liquidity.
> Shared-wallet races (input-race errors, address reuse, a stale standby
> balance until a rescan) are documented in the design doc. Withdraw/receive
> is never gated. A takeover still asserts dead-is-dead at the moment it runs
> — but a machine that was taken over and later restarted now reconciles each
> driven swap against chain before acting ("Reconcile before drive" above), so
> its settled swaps close out to their true terminal state instead of
> lingering active or arming a doomed refund against a spent leg.

The full design — the partitioning model, the followed-swap auto-purge at deep
terminal, the scope-rotation self-heal on re-import — is
`docs/MULTI_MACHINE_122.md`.

## Protocol selection prefers HTLC

When a pair could run either version, the engine **prefers v1 HTLC**
(`select_protocol`). It chooses v2 adaptor only for Taproot pairs that lack an
HTLC option:

- If both legs support CLTV + segwit (the v1 requirement), the engine selects
  **v1 HTLC**.
- v2 adaptor is selected only for taproot-capable pairs where HTLC is not
  available.

The shipped **BTCX ↔ BTC** pair therefore defaults to **v1 HTLC**. v2 is the
more advanced construction (better privacy, single-sig refund); the engine uses
the older, more-exercised path whenever it is available and v2 where it is the
better or only fit.

## Confirmation depth and reorg protection

A blockchain reorganization can un-confirm a transaction the engine already
acted on. Pact's defense is to withhold finality until a leg is buried deep
enough that a reorg is implausible (`default_confirmations`,
`engine.rs:388-394`):

| Chain class | Default `N` |
|---|---|
| Regtest | 1 |
| Fast chain (< 5-min spacing; BTCX ≈ 120s) | 10 |
| Slow chain (≥ 5-min spacing; BTC ≈ 600s) | 6 |

Two behaviors flow from this:

- **Auto-redeem / completion is withheld until N-deep.** The engine does not
  treat a swap as done — and keeps the refund armed — until the relevant spend
  is confirmed `N` blocks deep. A shallow reorg that un-confirms a redeem
  re-arms the refund rather than leaving a party exposed.
- **Reorg alert.** If a watched HTLC or leg output *vanishes* from the chain
  (a reorg dropped its funding or spend), the engine raises a reorg alert so
  the operator and the state machine can react rather than silently proceeding
  on a stale view.

Override `N` per coin via `Engine.coin_confirmations` (`satchel.json` →
`--coin-confs`). On mainnet and testnet the allowed band is **`[2, default]`**
— the heuristic default above is now also the *maximum* — and the engine
clamps an out-of-band operator value into the band (`confirmations_for`):
below 2 raises to 2, above the default lowers to the default. Regtest keeps a
floor of 1 and no cap. Within the band, deeper is safer but slower.

> **Note** — This is spec §7.3 **as amended for the rc12 recut**. The floor of
> **2** for both legs replaces the old asymmetric minimums (`N_A ≥ 6`,
> `N_B ≥ 1`, with earlier spec drafts suggesting e.g. `N_B = 3` for BTC):
> 1-block reorgs and stale blocks are routine on both chains, depth-2 reorgs
> are rare even on a 2-minute-spacing chain, so 0/1-conf trading is disallowed
> while two consenting users may trade at 2. The cap at
> `default_confirmations` keeps a fat-fingered depth from stalling a live swap
> for hours. Confirmation depth is **per-side-owned** in both protocols: each
> side derives its own `N_A`/`N_B` from its own per-coin config; the depths a
> counterparty advertises in the handshake are advisory display values, never
> adopted into safety gates (see the chapter "v2 Taproot/MuSig2 Adaptor
> Swaps" for the advisory exchange).

## The wallet-lock funding gate

Before committing a trade, the engine verifies that the funding coin's node
wallet is actually **unlocked** — a pre-flight check on the side that is about to
spend. If the wallet is **encrypted and locked**, the engine **refuses up front**
rather than accepting a trade it cannot fund: the taker's get-leg is refused at
**take**, and the maker's give-leg is refused at **post**, each with a message
telling the user to unlock the wallet (`walletpassphrase`) and keep it unlocked
until the swap completes.

The reason the balance check alone is not enough: a locked wallet still **reads**
its balance, so the funds gate passes, but it **cannot sign** the funding
transaction — `signrawtransactionwithwallet` returns RPC `-13`
("wallet must be unlocked"). Without this gate that failure surfaced only at
funding time, after the handshake, stranding the swap. The gate **fails open** on
a transient `getwalletinfo` error (node reachability is already covered by the
balance read), so a momentary RPC hiccup never blocks an otherwise-fundable
trade.

> **Note** — **Companion funding self-retry.** A v1 fund that fails to broadcast
> *after the state has already advanced* — e.g. a wallet locked mid-swap — is
> re-attempted on each scheduler tick. The retry is idempotent (it locates the
> funding on-chain first, so it never double-funds) and self-heals the moment the
> wallet is unlocked again; it composes with the pre-funding timeout-abort. v2
> adaptor funding does **not** yet auto-retry on a tick: it fails closed into a
> recoverable `Accepted` state, resumable via relay re-drive or a manual
> `adaptor_fund`.

> **Note** — **Pre-funding timeout-abort now covers v2 too.** A v1 handshake
> stuck pre-funding self-aborts after `PRE_FUNDING_TIMEOUT_SECS` (15 minutes) —
> since rc15 this includes a `Created` swap whose accept never arrives, which
> previously errored every tick forever instead of aborting. A v2 adaptor
> handshake stalled in `created`, `accepted`, or
> `nonces_exchanged` — before either leg is funded — now does the same,
> silently and independently on each side's own clock (`signed` is excluded, as
> funding may already be in flight by then). Previously a stalled pre-`signed`
> v2 handshake was inert to the ticker: neither the `abort` RPC (which only
> read the v1 table) nor a timeout could clear it, so a maker gone unreachable
> during the handshake left the taker with a record that could neither be
> cancelled nor time out. See the chapters "API: v1 HTLC Swaps" and "API: v2
> Adaptor Swaps".

## Safety properties

Pact's crypto core — the v1 witness construction, the v2 Taproot output and
tapleaf, the key derivation paths, and the adaptor reveal — matches the specs
and the pinned test vectors. Two operational details are worth understanding:

- **v2's cooperative redeem is not RBF-bumpable.** Its fee is sealed into the
  pre-signed adaptor sighash, so it cannot be fee-bumped after the fact the way
  an ordinary RBF transaction can. The engine handles this two ways: the redeem
  commits at the live market rate at init (live estimate, clamped) plus a CPFP
  redeem-bump child that spends the redeem's
  own sweep output to lift the package feerate if the network gets busy. The
  single-key refund path is always bumpable, so the funder is never the stuck
  party. See the chapter "Fees, Fee-Bumping & Auto-Refund".
- **Relay trust is liveness-only.** Pact never trusts a relay or a chain backend
  for *safety* — timelocks and on-chain enforcement protect funds regardless of
  what any relay says. A misbehaving or offline relay can only delay a swap (a
  liveness failure), never steal from it. The remedy for a dead relay is the
  timelock refund.

> **Tip** — The atomicity guarantees are real and chain-enforced. As with any
> self-custody software, keep `pactd` running for the life of any funded swap,
> and prefer the **Medium** or **Long** timelock presets for larger value (see
> the chapter "Timelocks & Action Deadlines").

## Summary

The safety model is: the chain enforces the deal, timelocks bound the worst
case, confirmation depth defends against reorgs, and relays are trusted only for
liveness. v2's cooperative redeem is non-bumpable by construction; it commits at
the market rate and is dragged through by a CPFP child. Both protocols are
reviewed and live on every network.
