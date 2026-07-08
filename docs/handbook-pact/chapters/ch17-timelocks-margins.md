# Timelocks & Action Deadlines

Every Pact swap is held together by two absolute Unix-time locktimes, `T1` and
`T2`. They are the only thing standing between an atomic swap and a one-sided
loss: if a deadline slips, the chain may let a counterparty redeem a leg the
other party expected to refund. This chapter sets out the structural rule that
orders the locktimes, the per-network minimums that bound their duration, and
the §7.4 *action-deadline margins* that build in safety slack — and explains
why those margins exist.

## The structural rule: `T2 < T1`

The participant's refund locktime must always be **earlier** than the
initiator's:

```text
T2  <  T1
```

This is checked on every network for both protocol versions
(`swap.rs:97-104` for v1, `adaptor_swap.rs:76-83` for v2). The reasoning, from
the lifecycle (see the chapter "The Swap Lifecycle"):

- Alice reveals the secret when she redeems leg B; that publishes it so Bob can
  redeem leg A.
- Bob must have a window to redeem leg A *after* Alice reveals, but *before*
  Alice could refund leg A. So Alice's refund (`T1`) must come last.
- Bob's own funds on leg B must be refundable (`T2`) before that, so he is
  never left exposed if Alice simply walks away after funding.

If `T2 ≥ T1`, the safety ordering collapses and the swap is unsafe. The engine
refuses to construct or accept such a swap.

## Network-profile minimums

Beyond the ordering, mainnet and testnet enforce duration minimums so the
windows are wide enough for real confirmation times and operator slack
(`validate_profile`, `engine.rs:237-257`). **Regtest is exempt** so tests can
use tight timelocks.

| Rule | Mainnet / Testnet | Meaning |
|---|---|---|
| `T2 ≥ now + 3h` | yes | Bob's leg must give at least 3h before its refund |
| `T1 − T2 ≥ 4h` | yes | at least a 4h gap between the two refund deadlines |
| `T1 ≤ now + 48h` | yes | the whole swap must conclude within 48h |
| `2 ≤ N_A ≤ default` | yes | leg A's confirmation depth sits in the `[2, default]` band |
| `2 ≤ N_B ≤ default` | yes | leg B's confirmation depth sits in the `[2, default]` band |

The `4h` gap (`T1 − T2`) is the core safety window: it is the time Bob has to
redeem leg A after Alice reveals the secret, before Alice's refund could
race him.

The confirmation-depth rows are spec §7.3 **as amended for the rc12 recut**:
both legs floor at `N ≥ 2` (1-block reorgs and stale blocks are routine on
both chains, while depth-2 reorgs are rare even on a 2-minute-spacing chain —
so 0/1-conf trading is disallowed, but two consenting users may trade at 2),
and both cap at the chain's `default_confirmations` heuristic, which is now
the *maximum* as well as the default (regtest keeps its floor of 1, uncapped).
Crucially, the depths are **per-side-owned**: each side derives its own
`N_A`/`N_B` from its own per-coin config, and the values the counterparty
sends in the handshake are advisory — used only to display their progress —
never adopted into this side's safety gates. See the chapter "Network Support,
Reorgs & Safety".

## The §7.4 action-deadline margins

The profile minimums shape the *timelocks*. The §7.4 *action margins* shape the
*deadlines by which the engine must act* — they pull each action earlier than
its raw locktime so that a transaction broadcast late, or a daemon recovering
from downtime, still has slack to confirm before the chain turns against it
(`action_margins`, `engine.rs:272-278`).

| Network | (fund, reveal, redeem_A) |
|---|---|
| Mainnet | (3h, 2h, 1h) |
| Testnet | (3h, 2h, 1h) |
| Regtest | (0, 0, 0) |

Each margin protects a specific action:

| Margin | Action it bounds | Deadline |
|---|---|---|
| `fund` = 3h | Bob funds leg B | no later than `T2 − 3h` |
| `reveal` = 2h | Alice redeems leg B (revealing the secret) | no later than `T2 − 2h` |
| `redeem_A` = 1h | Bob redeems leg A | before `T1 − 1h` |

The engine decides whether an action is still safe to start with one formula
(`engine.rs:272-278`):

```text
action_safe(clock, margin, deadline)  =  clock + margin < deadline
```

If the current clock plus the margin already reaches the deadline, the action
is **not** safe — the engine will not start a funding or reveal it cannot
expect to finish in time, and instead steers toward refund.

### The deadline clock

What "now" means depends on the network (`engine.rs`, deadline clock):

```text
deadline_clock = max(local_time, MTP)      off regtest
deadline_clock = MTP                        on regtest
```

Off regtest, the engine takes the **later** of local wall-clock time and the
chain's median-time-past (MTP) — being conservative, it never assumes the chain
clock is ahead of where it actually is. On regtest it uses pure MTP so
`setmocktime`-driven tests are deterministic.

## Why the margins exist: the reveal-too-late race

The margins are not padding for slow blocks; they close a specific attack
window — the *reveal-too-late race*. Walk it through without margins:

1. Alice waits until the very last moment before `T2` to redeem leg B and
   reveal the secret.
2. Her redeem confirms just barely before `T2`.
3. Bob now has the secret — but only a sliver of time before `T1` to get his
   leg-A redeem confirmed.
4. If Bob's redeem does not confirm in that sliver (a fee spike, a slow block,
   a brief outage), `T1` passes and **Alice can refund leg A** — taking back
   the coin Bob just paid her for. Bob has lost his leg-B coin to Alice's
   redeem *and* his claim on leg A. One-sided total loss.

The margins defuse this. By requiring Alice to reveal no later than `T2 − 2h`
and bounding Bob's redeem to `T1 − 1h`, with `T1 − T2 ≥ 4h`, Bob is guaranteed
a multi-hour window — between Alice's enforced reveal deadline and his own
enforced redeem deadline — to get leg A confirmed even under adverse
conditions. The engine will not let Alice's daemon reveal so late that Bob is
squeezed.

> **Warning** — The margins assume both daemons are *running* through the swap.
> A daemon down across its reveal or redeem deadline cannot act; the margins
> give a recovering daemon slack, but they are not a substitute for keeping
> `pactd` up for the life of a funded swap.

## Offer-offset validation at post time

The same constraints are checked when an offer is **posted**, on the offsets
(`t2_secs`, `t1_secs`) the maker advertises, so a malformed offer never reaches
the board (`validate_offer_offsets`, `engine.rs:312-330`):

```text
T2 < T1
t2_secs   ≥ 3h
t1_secs − t2_secs ≥ 4h
```

Regtest is exempt here too. Validating at post time means a taker who accepts a
listed offer can trust its timelocks are already sane.

## Recommended values: the UI presets

Satchel's offer form ships three timelock presets, all with `T1 = 2 × T2`
(`OfferForm.tsx:46-50`). These are the recommended values for real swaps:

| Preset | `T2` | `T1` | Notes |
|---|---|---|---|
| Short | 6h | 12h | fastest turnaround; least slack |
| **Medium (default)** | 12h | 24h | the recommended balance |
| Long | 18h | 36h | most slack; for slow or congested conditions |

All three satisfy the profile minimums (`T2 ≥ 3h`, `T1 − T2 ≥ 4h`,
`T1 ≤ 48h`) with comfortable headroom. **Medium** is the default and the right
choice unless you have a specific reason to widen or tighten the windows.

> **Tip** — Wider timelocks cost nothing but time-to-refund-if-things-go-wrong.
> If you expect fee volatility or intermittent connectivity, prefer **Long**.
> For a quick swap between two responsive, well-funded daemons, **Short** is
> fine. **Medium** is the safe default.
