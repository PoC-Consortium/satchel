# Regtest harness

Launches a Bitcoin PoCX regtest node + a Bitcoin regtest node and drives `pact`
engines (via `pactd`'s JSON-RPC and the `pact-cli` client) through the
spec/protocol.md handshake. Python 3.10+, stdlib only.

## Binaries

| Node | Resolution order |
|---|---|
| PoCX | `$POCX_BITCOIND` → `bin/pocx-bitcoind(.exe)` → `../../../bitcoin-pocx/bitcoin/build/bin/bitcoind(.exe)` |
| Bitcoin | `$BTC_BITCOIND` → `bin/btc-bitcoind(.exe)` → `bitcoind` on PATH |

`bin/` is gitignored — copy installed daemons there (e.g. from
`C:\Program Files\Bitcoin-PoCX\daemon\bitcoind.exe` and
`C:\Program Files\Bitcoin\daemon\bitcoind.exe`). Startup asserts the
regtest genesis hash per chain, so a mixed-up copy fails loudly.

## Usage

```sh
# infrastructure only (no Pact): nodes start, mine, mocktime works
python regtest_harness.py --smoke

# the v1 (HTLC) end-to-end suite
python test_swap_e2e.py

# the v2 (Taproot/MuSig2 adaptor) end-to-end suite
python test_adaptor_swap.py
```

Both suites build the cargo workspace (`pactd`, `pact-cli`) and the
Corkboard first, then run every scenario against a single shared `Harness`.

## Entry points

- **`regtest_harness.py`** — the infrastructure layer. `Harness` brings up
  both nodes, funds Alice/Bob wallets (one wallet per party per chain),
  keeps both chains on a shared mock clock, and exposes RPC plumbing plus
  `advance_time()` for CLTV/MTP tests. Imported by every other script;
  `--smoke` runs it standalone.
- **`test_swap_e2e.py`** — the v1 (`pact-htlc-v1`) suite. See scenarios
  below.
- **`test_adaptor_swap.py`** — the v2 (`pact-htlc-v2`) suite: Taproot/MuSig2
  adaptor swaps. Three scenarios — cooperative key-path redeem (happy path),
  CLTV-tapleaf refund, and a board-driven v2 swap (PoCX↔BTC defaults to v2,
  so a plain `boardpostoffer` posts a v2 offer). Drives the adaptor
  lifecycle (`adaptorinit`/`adaptoraccept`/`adaptorfund`/`adaptornonces`/
  `adaptorsign`/`adaptorassemble`/`adaptorredeem`/`adaptorrefund`) through
  `pactd`'s JSON-RPC.
- **`playground.py`** — the full stack on regtest for hands-off clicking:
  both nodes (funded), a Corkboard, and two `pactd`s (Alice with POCX, Bob
  with BTC) running self-schedulers (`--tick-secs 5`) and `--auto-fund`. It
  mines a block on each chain every few seconds and advances the mock clocks
  with wall time, so confirmations, the scheduler, and timelock expiries
  behave like a tiny live network. Drive the daemons with `pact-cli` against
  the printed JSON-RPC ports (no web UI — that returns in Satchel, Phase 3).
- **`satchel_playground.py`** — managed-Satchel playground: nodes + a
  Corkboard + two headless counterparties (Bob on the BUY side, Carol on the
  SELL side) each posting a rate-sorted spread, so the board shows a real
  two-sided book. You run one Satchel as "Alice" (funded on both coins) and
  take either side. Mines and re-tops offers on a timer.
- **`repro_multiswap.py`** — a focused repro for the async take-storm bug
  (C13): real self-schedulers plus a separate background block-miner thread,
  then one taker takes several offers back-to-back with no wait, to surface
  interleaving the synchronous tick→mine→tick suite can't.

## Status

**GREEN** (2026-06-12): `test_swap_e2e.py` runs eight scenarios:

1. *Complete swap, manual*: the Phase 1 DoD — each party runs its own
   `pactd`, driven by `pact-cli` (bitcoin-cli style) through the file
   handshake; Bob extracts s from the chain (no courtesy message). Asserts
   HTLC outpoints spent on-chain and both parties received the other asset.
2. *Refund, manual*: both fund, Alice goes silent, clocks pass T1, both
   refund. Negative checks: premature refunds and a late redeem past T2
   are rejected (spec §7.4).
3. *Daemon autopilot swap*: Alice drives everything through `pactd`'s
   JSON-RPC API (cookie auth — a call with no/invalid cookie is rejected
   401) with duplicated backends (spec §10 multi-backend path), exercising
   the wallet RPCs (balance/receive/send + chain guard) along the way; both
   sides redeem via the scheduler, advanced one pass at a time with the
   `tick` RPC; the scheduler RBF-bumps Alice's unconfirmed redeem; the swap
   books `completed` once her redeem confirms.
4. *Daemon autopilot refund*: both parties walk away after funding; the
   `tick` scheduler alone reclaims both legs after the timelocks — and
   provably does nothing before them.
5. *Create/import then swap* (Phase B): seedless start; Alice creates a
   fresh seed and Bob imports a known mnemonic (encrypted) via the
   seed-lifecycle RPCs (`createseed`/`importseed`) the Satchel wizard uses,
   then they complete a normal manual swap.
6. *Coin setup* (Phase C): the `listcoins`/`listpairs`/`validatecoin` RPCs —
   configured + connected + genesis state, capability-derived pair
   availability, and the genesis-hash check that gates saving a backend
   (accepts the right node, rejects a cross-wired one).
7. *Corkboard swap* (Phase 2): maker posts a signed offer, taker takes it,
   the whole handshake travels through the blind relay, and both legs
   auto-fund and auto-redeem to completion. Covers offer withdraw, a
   competing second take (served-once + reject + auto-delist), and asserts
   every relay blob on the board is sealed ciphertext (`PACTSEALED1:`),
   never plaintext coordination JSON.
8. *Private-offer swap* (PRIVATE_OFFERS.md): the maker builds an off-market
   offer with `makeprivateoffer` (a `pactoffer1:` slip, posted to no board)
   and hands the slip to the taker out of band; `takeoffer <slip>` relays
   the take through the same blind relay and the swap auto-completes — with
   no board listing ever existing.

`test_adaptor_swap.py` runs its three v2 scenarios from `main()`.

## Notes baked into the harness (from bitcoin-pocx source)

- PoCX regtest shares Bitcoin regtest's network magic (`fa bf b5 da`) and
  default port 18444 — nodes run with `-listen=0` and explicit `-rpcport`s
  so they can never cross-talk.
- PoCX regtest forging refuses rapid blocks unless `setmocktime` is active
  (`pocx/regtest/forging.cpp`); the harness always sets mocktime on both
  nodes and advances it explicitly (`Harness.advance_time`) for CLTV/MTP
  refund tests.
- Genesis hashes are asserted at startup so a mis-pointed binary fails
  loudly instead of testing the wrong chain.
