# The Swap Lifecycle

Pact settles a trade as an *atomic swap*: two on-chain transactions, one per
chain, bound so that either both parties get paid or both get their money
back. Nobody can run off with the funds. The chains — not a custodian, not a
relay, not Pact itself — enforce the deal. This chapter introduces the two
state machines that drive a swap to completion (v1 HTLC and v2 adaptor), the
roles each party plays, and the scheduler that walks every live swap forward
on a clock.

Two protocol versions ship today, both running on every network:

- **v1 (`pact-htlc-v1`)** — classic hash-timelock contracts (HTLCs) over
  P2WSH outputs. See the chapter "v1 HTLC Swaps".
- **v2 (`pact-htlc-v2`)** — Taproot outputs with a MuSig2 adaptor-signature
  binding. See the chapter "v2 Taproot/MuSig2 Adaptor Swaps".

The lifecycle, roles, and scheduler model described here are shared by both;
the difference is what the on-chain commitment *looks like* and how the secret
that unlocks it is revealed.

## The two roles

Every swap has exactly two parties, and the roles are fixed for the duration:

| Role | Code name | Who | Locks | Refund deadline | Holds the secret |
|---|---|---|---|---|---|
| *Initiator* | `Initiator` (Alice) | the maker — posts the offer | chain **A** | `T1` (later) | yes |
| *Participant* | `Participant` (Bob) | the taker — accepts the offer | chain **B** | `T2 < T1` (earlier) | no |

`Role` is defined in `swap.rs:50-57` (v1) and the same Alice-funds-A /
Bob-funds-B convention is documented in `adaptor_swap.rs:5-8` (v2).

The asymmetry is the whole point of the design:

- **The maker funds first.** Alice locks her coin on chain A before Bob locks
  his on chain B. The party who reveals the secret (Alice) is the one who must
  commit first, so she cannot strand Bob's funds without exposing her own.
- **Alice holds the secret.** In v1 it is a SHA256 preimage `s`; in v2 it is an
  adaptor scalar `t`. Either way, only the initiator can derive it from her
  seed, and revealing it on chain B is what lets Bob claim chain A.
- **`T2 < T1`.** Bob's refund unlocks *before* Alice's. Bob must always have a
  safe window to refund leg B after Alice's window to redeem it closes, before
  his own funds are at risk on the other side. This single inequality is the
  structural backbone of every Pact timelock; see the chapter "Timelocks &
  Action Deadlines".

> **Note** — "Maker funds first" is a protocol invariant, not a UI choice. The
> chain enforces the deal once both legs are funded; until then, the only party
> with capital at risk is the one who controls when the secret is revealed.

## The scheduler-driven model

A Pact swap is not a request/response RPC dance. Once a swap is `Accepted`,
the daemon owns it: `pactd` runs a background scheduler that re-examines every
live swap on a fixed interval and takes whatever action the current state and
the chain clock allow. The operator does not poll, click "redeem", or watch
the mempool — the engine does.

The loop is `Engine::tick`, which fans out to one handler per swap
(`engine.rs:2581`). For v1 each swap is advanced by `tick_one`
(`engine.rs:2654`); for v2 by `adaptor_tick_one` (`engine.rs:2614-2620`). The
interval is set by the `--tick-secs` flag.

On each tick, for each swap, the engine:

1. Reads the current state and the party's role.
2. Checks the chains it watches: have the funding outputs confirmed? has the
   counterparty redeemed and thereby revealed the secret? has a refund
   timelock matured (measured against the chain's median-time-past)?
3. Arms exactly the action the state machine permits — fund, redeem, refund,
   fee-bump, or nothing — and persists the new state.

This is what makes **auto-redeem and auto-refund clock-driven**. Alice's
daemon redeems leg B the moment it is safe; Bob's daemon extracts the revealed
secret and redeems leg A; and either party's daemon fires the timelock refund
the moment its deadline passes and the output is still unspent — all without
human intervention, and all while the operator is offline-tolerant within the
timelock margins (see the chapter "Timelocks & Action Deadlines"). The
per-role, per-state arming lives in `engine.rs:2666-2774` for v1.

> **Warning** — Because the engine acts on a clock, a daemon that is *down*
> across a critical deadline cannot fire its refund. The action-deadline
> margins (fund 3h / reveal 2h / redeem-A 1h on mainnet) exist precisely to
> give a recovering daemon slack. Keep `pactd` running for the life of any
> swap you have funded. See the chapter "Timelocks & Action Deadlines".

## v1 state machine — `swap::State`

The v1 lifecycle is `swap::State` (`swap.rs:61-72`). The happy path is a
straight line; the two refund/abort states branch off from any funded state,
driven by the clock rather than by a message.

```text
                        ┌─────────────┐
                        │   Created   │   offer made / init sent
                        └──────┬──────┘
                               │  accept
                        ┌──────▼──────┐
                        │  Accepted   │   both HTLCs reconstructed
                        └──────┬──────┘
                               │  Alice funds chain A (T1)
                        ┌──────▼──────┐
                        │   FundedA   │
                        └──────┬──────┘
                               │  Bob funds chain B (T2 < T1)
                        ┌──────▼──────┐
                        │   FundedB   │
                        └──────┬──────┘
                               │  Alice redeems B with s
                        ┌──────▼──────┐                  ┌────────────┐
                        │  RedeemedB  │   s now public   │  Refunded  │
                        └──────┬──────┘                  └─────▲──────┘
                               │  Bob extracts s,              │ clock:
                               │  redeems A                    │ MTP ≥ T &
                        ┌──────▼──────┐                        │ HTLC unspent
                        │  Completed  │      (from any FundedA/FundedB)
                        └─────────────┘
                                                          ┌────────────┐
            Aborted ◄──── handshake/validation failure    │  Aborted   │
                                                          └────────────┘
```

- `Created → Accepted`: the offer is taken; both parties now hold enough
  parameters (`SwapParams`, `swap.rs:74-91`) to reconstruct **both** HTLCs
  byte-for-byte and verify the counterparty's script locally.
- `Accepted → FundedA`: Alice broadcasts the chain-A funding transaction.
- `FundedA → FundedB`: Bob, having verified leg A on chain, funds leg B.
- `FundedB → RedeemedB`: Alice redeems leg B by revealing the preimage `s` in
  the spending witness. This is the irreversible step.
- `RedeemedB → Completed`: Bob's daemon scans the chain-B spend, extracts `s`,
  and redeems leg A.
- `Refunded` is reachable from `FundedA` or `FundedB` when a refund timelock
  matures and the corresponding HTLC is still unspent. `Aborted` covers
  pre-funding handshake or validation failures.

## v2 state machine — `AdaptorState`

The v2 lifecycle is `AdaptorState` (`adaptor_swap.rs:34-48`). It is the same
shape as v1 with **two extra pre-funding states** for the MuSig2 adaptor
ceremony that must complete *before* either party puts coins on chain.

```text
                        ┌─────────────────┐
                        │     Created     │
                        └────────┬────────┘
                                 │  accept
                        ┌────────▼────────┐
                        │    Accepted     │
                        └────────┬────────┘
                                 │  exchange MuSig2 nonces (both redeem sessions)
                        ┌────────▼────────┐
                        │ NoncesExchanged │
                        └────────┬────────┘
                                 │  aggregate + verify both adaptor sigs vs T
                        ┌────────▼────────┐
                        │     Signed      │   both legs pre-signed, fund-safe
                        └────────┬────────┘
                                 │  Alice funds leg A (T1)
                        ┌────────▼────────┐
                        │     FundedA     │
                        └────────┬────────┘
                                 │  Bob funds leg B (T2 < T1)
                        ┌────────▼────────┐
                        │     FundedB     │
                        └────────┬────────┘
                                 │  Alice adapts + broadcasts leg-B redeem
                        ┌────────▼────────┐                ┌────────────┐
                        │    RedeemedB    │  t now public  │  Refunded  │
                        └────────┬────────┘                └─────▲──────┘
                                 │  Bob extracts t,              │ clock:
                                 │  adapts + redeems A           │ MTP ≥ T &
                        ┌────────▼────────┐                      │ leg unspent
                        │    Completed    │    (from any FundedA/FundedB)
                        └─────────────────┘
                                                            ┌────────────┐
              Aborted ◄──── handshake/validation failure    │  Aborted   │
                                                            └────────────┘
```

The two added states are the heart of v2's safety:

- `Accepted → NoncesExchanged`: both parties exchange fresh MuSig2 secret
  nonces for the two redeem sessions (one per leg). Nonces are generated from a
  CSPRNG, never the seed, and persisted write-ahead — reuse is structurally
  impossible. See the chapter "v2 Taproot/MuSig2 Adaptor Swaps".
- `NoncesExchanged → Signed`: both adaptor signatures are aggregated and
  **verified against the adaptor point `T`** before anyone funds. Reaching
  `Signed` is the guarantee that the cooperative redeems will work and that
  Alice's broadcast will reveal `t` to Bob.

> **Note** — In v2 the legs are pre-signed (state `Signed`) before any funding
> transaction is broadcast. By the time a party commits coins, the redeem
> messages already exist and have been checked against `T`. From `FundedB`
> onward, v2 behaves like v1: Alice's redeem reveals the secret on chain, Bob's
> daemon extracts it and claims his leg.

## Where the secret lives

Both versions hinge on a single secret that only the initiator can produce,
and both derive it from the seed so that losing the state database never loses
the secret:

| | v1 | v2 |
|---|---|---|
| Secret | preimage `s` (32 bytes) | adaptor scalar `t` |
| Public commitment | `H = SHA256(s)`, on both legs | point `T = t·G`, binds both redeem sigs |
| Revealed by | the redeem witness on leg B | the adapted leg-B redeem signature |
| Extracted by Bob | SHA256-scan of the leg-B witness | subtracting his adaptor sig from the broadcast sig |

The mechanics of derivation, the exact witness layouts, and the extraction
procedure are covered in the v1 and v2 chapters that follow. The lifecycle to
hold in mind is the same in both: maker funds first, the chain enforces the
deal, and the scheduler redeems or refunds on a clock.
