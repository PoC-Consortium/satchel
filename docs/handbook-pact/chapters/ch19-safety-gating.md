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
`--coin-confs`), floored at `≥ 1`. Deeper is safer but slower.

> **Note** — Spec §7.3's suggested-minimums table lists `N_B = 3` for BTC, but
> the engine's heuristic `default_confirmations` returns **6** for any chain
> with ≥ 5-minute block spacing, including BTC. The engine default is more
> conservative than the spec's floor; the spec value is a *minimum*, not the
> shipped default.

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
> stuck pre-funding has long self-aborted after `PRE_FUNDING_TIMEOUT_SECS` (15
> minutes). A v2 adaptor handshake stalled in `created`, `accepted`, or
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
  commits at the live market rate at init (live estimate × `committed_mult`,
  default **1**, clamped) plus a CPFP redeem-bump child that spends the redeem's
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
