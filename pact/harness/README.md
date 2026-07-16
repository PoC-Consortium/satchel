# Pact test harness

The regtest e2e framework + interactive playgrounds for the Pact swap engine.
Python 3.10+, **stdlib only** — no venv/pip needed on a fresh box. Structure
follows Bitcoin Core's functional-test framework; the full design and roadmap
live in `docs/TEST_FRAMEWORK_PLAN.md`.

```
framework/          the importable library
  binaries.py         ONE binary resolver (shared bin/ dir + env overrides)
  node.py             Node (pocx/btc/ltc) + ElectrsServer + Harness
  daemon.py           Pactd (a.k.a. Party) — one pact daemon under test
  services.py         Corkboard + NostrRelay
  stack.py            build_workspace + the funded-datadir cache
  testbase.py         PactTestFramework: one scenario = one fresh stack
  util.py             cookie-RPC client, wait_until, assert helpers,
                      the MAINNET-SAFE teardown port registry
tests/              the asserting suites (one scenario class per cell)
test_runner.py      runs everything (the CI entry once e2e lands in CI)
bin/                gitignored: node/electrs/relay binaries (see below)
cache/              gitignored: pre-mined funded datadirs (auto-built)
```

## Running the tests

```sh
# everything (builds the workspace once, then each suite as a subprocess)
python test_runner.py

# one suite / one scenario
python tests/swap_v1.py
python tests/swap_v1_rescue.py --filter RescueTakerCommittedV1

# fast sanity only (no nodes): port registry, allocator, binary resolver
python tests/framework_selftest.py

# infrastructure smoke (no Pact): nodes start, mine, mocktime works
python regtest_harness.py --smoke
```

Every scenario runs on its own fresh stack in its own tmpdir (kept + path
printed on failure; `--keep` keeps all). The funded datadirs come from an
on-demand cache under `cache/` — invalidated automatically when the node
binaries change, forced with `--rebuild-cache`. The runner is **sequential by
design** (fixed ports — see `framework/util.py`'s registry).

Suites: `swap_v1` (14 v1 HTLC scenarios: manual/autopilot/refund, chain-watched
funding, fee-bump nurses, board + nostr + private offers, board-reset,
concurrent-drain), `swap_v1_rescue` (the 7-cell seed-only rescue matrix, #54),
`swap_v2_adaptor` (8 Taproot/MuSig2 scenarios incl. CPFP nurses, depth gate,
LTC), `nodeless` (4 bdk-over-electrs parity scenarios, #58), `follow` (3
dormant-observer reconstruction cells, #166), `multimachine` (#122 partition
checks), `framework_selftest`.

Adding a scenario: subclass `PactTestFramework` in the matching `tests/*.py`
and append it to that file's `SCENARIOS`. A new file must also be added to
`TEST_LIST` in `test_runner.py` — the runner hard-errors on unlisted files, so
nothing can be silently skipped.

## Binaries

`framework/binaries.py` is the single resolver: the shared `bin/` dir
(override the dir with `PACT_HARNESS_BIN`), a per-binary env var, then legacy
fallbacks. Copy the daemons in once:

| Binary | env override | bin/ name |
|---|---|---|
| PoCX node | `POCX_BITCOIND` | `pocx-bitcoind(.exe)` |
| Bitcoin node | `BTC_BITCOIND` | `btc-bitcoind(.exe)` |
| Litecoin node (LTC cells only) | `LITECOIND` | `litecoind(.exe)` |
| PoCX electrs | `PACT_ELECTRS_BIN` | `electrs(.exe)` |
| vanilla electrs (BTC leg) | `PACT_BTC_ELECTRS_BIN` | `btc-electrs(.exe)` |
| Nostr relay | `PACT_NOSTR_RELAY_BIN` | `nostr-rs-relay(.exe)` |

Node startup asserts the regtest genesis hash per chain, so a mixed-up copy
fails loudly.

## Playgrounds (interactive, launched via tools/*.ps1 until Phase 3)

- **`playground.py`** — headless full stack (nodes + Corkboard + two
  auto-fund pactds) for driving with `pact-cli`.
- **`satchel_playground.py`** — managed-Satchel two-sided market (Bob buys /
  Carol sells, LTC sub-book); `--nodeless` puts Alice's btcx on a bdk wallet
  over electrs (`--electrs-count N` for the failover fleet).
- **`satchel_playground_nostr.py`** — the same book relays-only (no
  corkboard); `--nodeless` for the full end-user vision stack.
- **`observer_playground.py`** — main + observer Satchel on one seed
  (follow/failover eyeballing); `observer_compare.py` is its read-only
  divergence oracle.
- **`repro_multiswap.py`** — async take-storm repro (C13).

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
- **Teardown is PID/port-only, never by process name** — the registry in
  `framework/util.py` is the single source of kill targets and structurally
  cannot contain the live mainnet/testnet pactd ports (9737/9738).
