# Test & Playground Infrastructure — Consolidation Plan

Status: **respec'd + decided (2026-07-16)** — assessment findings folded in,
design decisions locked (§2.0). Supersedes the PowerShell-only scope of issue
#110 ("flexible playground"); #110 is re-scoped to point here and closes when
Phase 3 lands.

Blueprint: Bitcoin Core's functional test framework
(`test/functional/test_framework/` + `test_runner.py`).

---

## 1. Deep analysis — current state

Three orchestrators, ~7,700 lines, all standing up variations of the same
regtest stack:

| Surface | LOC | What it really is |
|---|---|---|
| **Python** `pact/harness/*.py` (12 files) | 5,163 | The real engine. Asserting e2e **and** interactive playgrounds. |
| **PowerShell** `tools/playground-*.ps1` (9 scripts) | 1,924 | **Thin GUI-launch + teardown wrappers** around the Python drivers. |
| **Rust** `tools/demo-runner` | 591 | A no-toolchain repackaging of `satchel_playground_nostr.py`. Referenced by nothing. |

Key finding: **this isn't three independent stacks — it's one Python stack
with two redundant shells around it.**

- **The PowerShell scripts contain almost no orchestration.** They pin binary
  env-vars, write `satchel.json`, stage sidecars into
  `satchel/binaries/<triple>`, launch a Python driver + the Satchel GUI, and
  tear down by port. Node bringup / funding / mocktime / relay / bot-pactd
  logic is *entirely* in Python. What's duplicated across the 7 GUI scripts is
  a copy-pasted skeleton — `Kill-Tree`, `Stop-Playground`, the `satchel.json`
  here-string, sidecar staging — repeated verbatim. This is issue **#110**.
- **`demo-runner` is redundant and orphaned** — it re-implements the
  relays-only managed-Satchel playground in Rust, is invoked by nothing in
  tracked source (only name-dropped in ps1 comments + a `.gitignore` stanza),
  and was a one-time packaging artifact.
- **The Python layer itself holds the real structural debt**, hidden by the
  PS1/Rust noise:
  - **No base class.** No `BitcoinTestFramework` analog (no
    `set_test_params`/`setup_network`/`run_test`/`main` lifecycle). Every
    suite re-implements setup, a run loop, and teardown.
  - **The 1,665-line `test_swap_e2e.py` monolith does double duty** as both
    the biggest test *and* the de-facto framework library: it defines `Party`
    (the pactd wrapper), `Corkboard`, `NostrRelay`, and `build_workspace()` —
    and **every other file imports infra from it**. A test file is the library
    root.
  - **All 21 monolith scenarios share ONE `Harness`** — shared chains, a
    mocktime that only ratchets forward (each refund scenario jumps +5h),
    shared wallets (asserted via before/after deltas), and party names that
    must be globally unique across scenarios ("alice6/bob6 are taken" is a
    live comment). Bitcoin Core gives every test a fresh stack; we don't.
  - **No runner/discovery.** Scenarios are hand-listed in per-file
    `tests=(…)` tuples; an unlisted `test_*` silently never runs. Adding
    scenario #22 means editing a central list — which is why #183's
    concurrent-drain repro got *folded into the monolith* instead of a new
    file.
  - **Duplication inside Python too:** `NostrRelay` is defined twice — and
    the copies have **diverged**: the e2e one (port 19791) fail-louds on a
    port already in use (a leaked relay's stale DB poisons scenarios), the
    playground one (port 19788) doesn't. `spike_electrs.py` re-implements
    `Party` as `NodelessPactd`; the cookie-auth JSON-RPC client is hand-rolled
    5× in Python **plus a 6th time in PowerShell**
    (`playground-multimachine.ps1` `Invoke-Pactd`); `wait_until` is open-coded
    in every file; the playground mine/mocktime loop exists in **five**
    variants (uniform 4s, per-coin cadence ×2, per-coin + observer auto-take,
    async thread miner) all re-implementing the subtle monotonic-mocktime rule
    ("never move the clock backwards; PoCX forging auto-advances it").
  - **Binary resolution is scattered** — `regtest_harness.py` `find_*` helpers
    point at `pact/harness/bin`, the ps1 set env-vars at the same,
    `demo-runner` resolves relative to its own exe into a *different*
    `bin/`+`satchel/` layout, and Satchel sidecars live in
    `satchel/binaries/`. No single source.
  - **The teardown port lists have already drifted:** `knockdown.ps1`
    (delegating to `playground-cork.ps1 -Down`) misses the playground Nostr
    relay (19788), the observer's second pactd (9740), the nodeless-BTC ports
    (18332/19760/19761) and the electrs fleet (19752–19757) — a stale
    nostr/observer/multi-electrs playground **survives knockdown today**. The
    single unit-tested registry (§2) fixes a live gap, not a hypothetical one.

**Hard invariant that must survive any refactor:** teardown is
**PID/port-only, never by process name**, and the port list must **never**
include mainnet `9737` / testnet `9738` — a by-name sweep killed the live
mainnet node on 15 Jun 2026. (See memory `no-kill-nodes-by-name` and the
corrected port registry in the appendix.)

---

## 2. Proposed structure — Python framework on the Bitcoin Core blueprint

### 2.0 Decisions (locked 2026-07-16)

| Decision | Choice |
|---|---|
| Isolation model | **Per-scenario stack** (Core-strict): every scenario runs against fresh nodes/services in its own tmpdir. Made affordable by a **Core-style datadir cache** (§2.3) — this cache is a *required* Phase 2 deliverable, not an optimization. |
| Runner | **Stdlib, hand-rolled** (no pytest): Core-style `test_runner.py`, explicit ordered list **cross-checked against the directory** (unlisted file = hard error), `--filter`, per-scenario tmpdir kept on failure, **sequential by design** (fixed ports; do not add `--jobs` without port-namespacing work). |
| Delivery | **One PR per phase** (§3) — each leaves the tree green and e2e-verifiable. |
| Issue #110 | **Re-scoped** to point at this doc; closed when Phase 3 lands. |

### 2.1 Layout

Split cleanly into **framework (library) / tests (asserting) / play
(interactive)**, with one binary dir and one runner:

```
pact/harness/
  framework/                  # the importable package (Core's test_framework/ analog)
    binaries.py               # ONE resolver -> the single shared bin dir (env-overridable)
    node.py                   # Node (pocx/btc/ltc) + ElectrsServer   [from regtest_harness.py]
    daemon.py                 # Pactd  (rename of Party = Core's TestNode); the ONE cookie-RPC client
    services.py               # Corkboard + NostrRelay  (single home — kills the diverged duplicate;
                              #   the fail-loud port probe becomes universal)
    stack.py                  # INFRA composition only: nodes+electrs(+fleet)+board+relay+pactds
                              #   from a spec; owns the datadir cache (§2.3)
    market.py                 # market simulation: Bob/Carol offer books, non-destructive topup/
                              #   repost, nodeless faucet polling, observer auto-take
    clock.py                  # the mining/mocktime models: uniform cadence, per-coin cadence,
                              #   async thread miner; ONE home for the monotonic-mocktime rule
    satchel.py                # Satchel GUI launcher (§2.4): satchel.json writer (UTF-8 no-BOM),
                              #   sidecar staging, SATCHEL_NETWORK/SATCHEL_DATA_DIR, pidfile,
                              #   block-on-window, tree-kill teardown
    testbase.py               # PactTestFramework base: set_test_params/setup_stack/run_test/main
                              #   + argparse + self.log; one instance = one fresh stack
    util.py                   # wait_until, assert_*, cookie helpers, MAINNET-SAFE teardown port
                              #   registry (ranges, unit-tested) + kill-by-port (tree-kill, per-OS)
  tests/                      # asserting suites; each file holds 1..n PactTestFramework
                              #   subclasses, each class = one scenario = one fresh stack
    swap_v1.py  swap_v1_rescue.py  swap_v2_adaptor.py  follow.py  nodeless.py  multimachine.py
  play/
    __main__.py               # ONE flag-composed playground (§2.5)
    repro_multiswap.py        # diagnostic driver (kept, framework-based)
    observer_compare.py       # live observer/main oracle (kept, framework-based)
  test_runner.py              # runs each tests/*.py as a subprocess, list-vs-dir cross-check,
                              #   per-test timing, --filter, keep-tmpdir-on-failure (CI entry)
  bin/                        # the ONE shared binaries dir (gitignored): nodes, electrs, relay
  cache/                      # datadir cache (gitignored), built on demand (§2.3)
```

Rust `tests/vectors.rs` + `vectors_v2.rs` (spec-vector regression pins) stay as
cargo integration tests — unrelated to this consolidation.

### 2.2 Disposition of every current file

| Today | Becomes |
|---|---|
| `regtest_harness.py` | `framework/node.py` + `framework/binaries.py` (+ cache logic → `stack.py`) |
| `test_swap_e2e.py` | infra classes → `framework/`; 21 scenarios → `tests/swap_v1.py` + `tests/swap_v1_rescue.py` (+ board/nostr cells staying with their protocol group) |
| `test_adaptor_swap.py` | `tests/swap_v2_adaptor.py` (its two-Harness main — core + with_ltc — maps to per-scenario stacks trivially) |
| `test_nodeless_e2e.py` | `tests/nodeless.py` |
| `test_follow_e2e.py` | `tests/follow.py` |
| `playground.py` | `play --board cork --satchel none` (headless variant) |
| `satchel_playground.py` | `play --board cork` |
| `satchel_playground_nostr.py` | `play --board nostr` |
| `observer_playground.py` | `play --board nostr --satchel two-observer` |
| `observer_compare.py` | `play/observer_compare.py` (companion oracle tool, framework imports) |
| `repro_multiswap.py` | `play/repro_multiswap.py` (diagnostic, framework imports) |
| `spike_electrs.py` | **deleted** — superseded by `tests/nodeless.py` (its `NodelessPactd` and its port-19752 collision with the electrs fleet range die with it) |
| `tools/playground-*.ps1` (7 GUI wrappers) | **deleted** (Phase 3); flag matrix in §2.5 |
| `tools/playground-multimachine.ps1` | `tests/multimachine.py` (it's an asserting test: data-dir lock, distinct machine labels, derive-scope partition; the printed MANUAL failover walkthrough survives as its docstring) |
| `tools/knockdown.ps1` | `play --down` |
| `tools/demo-runner` | **deleted** (Phase 4, incl. the `.gitignore` stanza) |
| `pact/harness/README.md` | rewritten in Phase 3 |
| `tools/relay-prober` | kept as-is (orthogonal dev tool) |

### 2.3 Per-scenario isolation + the datadir cache

Every `PactTestFramework` instance gets a fresh stack in its own tmpdir. Raw
per-scenario bringup would re-mine 110 funding blocks × ~37 scenarios, so we
adopt Bitcoin Core's cache mechanism:

- `framework/stack.py` builds `pact/harness/cache/` **once, on demand**: start
  each node type, create the standard wallet layout (`alice_pocx` funded /
  `bob_pocx` empty; `bob_btc` funded / `alice_btc` empty; ltc variant with
  `alice_ltc`/`bob_ltc`/`carol_ltc` for stacks that ask), mine 110 blocks,
  stop cleanly, keep the datadirs.
- Each scenario **copies** the cached datadirs into its tmpdir and starts the
  nodes there; mocktime is re-based per scenario exactly as `Harness.__enter__`
  does today (`max(tip time, now)`), so cached block timestamps are harmless.
- Cache invalidation: a fingerprint file (node binary path + size + mtime,
  per coin) written at build; mismatch → rebuild. A `--rebuild-cache` runner
  flag forces it.
- electrs is **not** cached — it indexes the copied chain at start (fast at
  height 110). Corkboard/relay are per-scenario ephemeral as today.

What per-scenario isolation buys (and the migration should *not* immediately
exploit — keep assertions behavior-preserving during conversion, simplify
later): unique-party-name bookkeeping across scenarios becomes unnecessary,
the distinct-mnemonic-per-rescue-cell requirement dissolves, and
before/after balance deltas could become absolute assertions.

Expected cost: node start (~1–2 s × 2–3 nodes) + datadir copy per scenario —
a few minutes over today's shared-stack total for the full run. Accepted.

### 2.4 The Satchel launcher (`framework/satchel.py`) — parity requirements

Porting the ps1 skeleton means preserving all of its load-bearing details:

1. **satchel.json writer** — all variants (cork / nostr relays-only /
   nodeless / first-run empty-coins / viewer mainnet-no-coins), written
   **UTF-8 without BOM** (a BOM breaks pactd's serde parse), single source
   for the coins/confirmations blocks.
2. **Sidecar staging** — copy `pactd.exe`/`pact-cli.exe` to
   `satchel/binaries/<host-triple>` (triple read from `rustc -vV`).
3. **Data-dir semantics** — regtest playgrounds wipe `<config>/regtest/pactd`
   for a factory-new run; the prod viewer **persists** its config dir and only
   refreshes `pactd_path` on relaunch; `SATCHEL_NETWORK` /
   `SATCHEL_DATA_DIR` env selection.
4. **Teardown parity** (in `util.py`, shared with `play --down`):
   - **tree-kill** (`taskkill /T /F` semantics) — `cargo tauri dev` is a
     cargo→app→Vite tree and the app itself listens on no port;
   - a **pidfile** recording the driver + Satchel PIDs — the Python driver
     listens on nothing either, and a half-dead driver's Harness cleanup
     would `stop` the *next* run's fresh nodes (today's ps1 hunts orphan
     drivers by script path for exactly this reason);
   - the **wait-until-ports-free loop** before the next stack comes up;
   - **per-OS kill-by-port**: `Get-NetTCPConnection`/`netstat -ano`+`taskkill`
     on Windows, `lsof -i`/`kill` on POSIX — the Python port makes Linux
     playground runs (Debian wizard testing, #161) possible for the first
     time;
   - the **port registry** (appendix) is the only source of kill targets,
     models the pactd allocation *range* (not fixed slots), and structurally
     cannot contain 9737/9738 — unit-tested to prove it.
5. **Block-on-window** — Satchel exit tears the whole stack down (kept).

### 2.5 The flexible playground (fulfils #110, in Python)

One entrypoint whose flags compose the stack:

```
python -m harness.play --board cork|nostr|none --btcx node|nodeless --electrs N
                       --satchel one|two-observer|viewer|none
                       [--first-run] [--relay-cmd CMD] [--persist] [--keep] [--down]
```

| Today | Becomes |
|---|---|
| `playground-cork.ps1 [-FirstRun]` | `play --board cork [--first-run]` |
| `playground-nostr.ps1 [-FirstRun] [-RelayCmd …]` | `play --board nostr [--first-run] [--relay-cmd …]` |
| `playground-nodeless.ps1 [-MultiElectrs]` | `play --board cork --btcx nodeless --electrs N` |
| `playground-nostr-nodeless.ps1` | `play --board nostr --btcx nodeless` |
| `playground-observer.ps1` | `play --board nostr --satchel two-observer` |
| `playground.py` (headless) | `play --board cork --satchel none` |
| `playground-viewer.ps1` | `play --satchel viewer` (mainnet, no backdrop, ephemeral) |
| `prod-watch-viewer.ps1` | `play --satchel viewer --persist` |
| `knockdown.ps1` | `play --down` (kills the FULL registry — closes today's knockdown gaps) |
| `--keep` | leaves the stack up on driver exit (the open `-Keep` request from #125) |

**Why PowerShell disappears entirely:** the only two things ps1 does that
Python doesn't are (a) launch the Satchel GUI and (b) kill-by-port on Windows.
Both are portable (§2.4). A one-line `play.cmd` shim stays only if a
double-click entry is wanted.

**Satchel attach seam:** Satchel supports three pactd modes
(spawn / adopt / external via `SATCHEL_PACTD_URL`). In external mode the pactd
child is "not ours to kill" (`satchel/src/main.rs:375`), so the Python-owned
backdrop can own every process and Satchel just attaches.

**Layering rule:** `play/__main__.py` is *composition only* — stack spec from
flags, then `stack.py` (infra) + `market.py` (books/faucet/auto-take) +
`clock.py` (miner model) + `satchel.py` (GUI). If `play` grows scenario logic,
that logic belongs in one of those four modules; this is the guard against
`play` becoming the next monolith.

### 2.6 Runner + discovery

- `test_runner.py` holds an **explicit ordered list** (longest first, like
  Core) and **hard-errors** if a `tests/*.py` exists that isn't listed —
  hand-curated order, impossible-to-forget registration.
- Each test file runs as a **subprocess** (crash isolation, clean env); inside
  a file, each scenario class runs with its own fresh stack + tmpdir.
- `--filter <substr>`, per-test wall-time report, tmpdir kept + path printed
  on failure, `--rebuild-cache`.
- `build_workspace()` (cargo build) is hoisted into the runner — built once
  per run, not once per suite.
- **Sequential by design** (fixed ports). Parallelism would need Core-style
  port-seeding; explicitly out of scope.

### 2.7 Migration tolerance

Phases 1–2 are "behavior-preserving" at the level of **observable suite
outcomes**, not byte-identical logs. Known deliberate unifications: the
merged `NostrRelay` keeps the fail-loud port probe everywhere (playgrounds
gain it); `Pactd` log-open mode unifies on `"w"`; per-scenario stacks replace
the shared Harness (assertion deltas still hold on fresh stacks — they're
just no longer load-bearing).

---

## 3. Migration plan — one PR per phase, each leaves the tree green

- **Phase 0 — one bin dir + `binaries.py`.** Pure consolidation, no behavior
  change (bin/ already is the de-facto home; this formalizes it + adds the
  resolver). Enabler.
- **Phase 1 — extract `framework/` from the monolith, no behavior change.**
  Move `Party`→`daemon.py` (keep a `Party` alias through Phase 2),
  `Corkboard`/`NostrRelay`→`services.py` (delete the diverged 2nd copy, probe
  becomes universal), `build_workspace`→`stack.py`; add `util.py`
  (`wait_until`/`assert_*`/cookie/teardown registry). Suites keep their own
  `main()`s and shared Harnesses this phase. Prove green by running all
  suites.
- **Phase 2 — `PactTestFramework` + datadir cache + `test_runner.py`; split
  the monolith.** The big one: per-scenario stacks land here (§2.3), suites
  convert to scenario classes, `multimachine.ps1` logic becomes
  `tests/multimachine.py`, runner + list-vs-dir cross-check + subprocess
  execution. Now `python test_runner.py` runs the whole e2e set hermetically.
- **Phase 3 — `framework/satchel.py` + `market.py`/`clock.py` +
  `play/__main__.py`; delete the 8 ps1.** Port the GUI-launch + teardown per
  §2.4, wire the full flag matrix (§2.5), move repro/observer_compare under
  `play/`, delete `spike_electrs.py`, rewrite `pact/harness/README.md`.
  **Closes #110.**
- **Phase 4 — delete `demo-runner`** (+ its `.gitignore` stanza).
- **Phase 5 (optional, unblocks CI e2e).** Once binaries have a home
  (cache/artifact/self-hosted runner), `test_runner.py` is the single CI
  entry — `ci.yml` already documents this as the sole blocker. (The
  multi-workspace `cargo test` sprawl is orthogonal Rust tidy-up — kept out of
  this effort unless bundled deliberately.)

**Risk controls:** Phases 0–1 validated by the existing suites; Phase 2
validated by a full runner pass + spot-diff of scenario outcomes against a
pre-split run; the mainnet-safe port registry is centralized and unit-tested;
teardown stays PID/port-only throughout; `demo-runner` deletion is safe
(nothing references it); each phase is an independently revertible PR.

---

## Appendix A — what maps where (source inventory)

Shared abstractions that already exist (imported, not copied):
- `Party` (pactd wrapper) — `test_swap_e2e.py:184` → `framework/daemon.py`
- `Corkboard` — `test_swap_e2e.py:55` → `framework/services.py`
- `NostrRelay` — `test_swap_e2e.py:100` (+ **diverged** dup at
  `satchel_playground_nostr.py:122`) → `framework/services.py` (single, probed)
- `build_workspace()` + path constants — `test_swap_e2e.py` → `framework/stack.py`
- `Harness` / `Node` / `ElectrsServer` / `find_*` — `regtest_harness.py`
  → `framework/node.py` + `framework/binaries.py`

Copy-paste to collapse:
- `NostrRelay` ×2 (behaviorally diverged — see §2.7)
- `NodelessPactd` (`spike_electrs.py:53`) re-implements `Party` (file deleted)
- cookie-auth JSON-RPC client ×5 in Python (`Party.rpc`,
  `satchel_playground.alice_rpc`, `observer_playground.rpc`,
  `observer_compare.rpc`, `spike_electrs.NodelessPactd.rpc`) **+ ×1 in
  PowerShell** (`playground-multimachine.ps1` `Invoke-Pactd`)
- ad-hoc `wait_until` loops in every file
- mine/mocktime loop ×5 variants → `framework/clock.py`;
  offer books + topup/faucet/auto-take → `framework/market.py`

## Appendix B — regtest port registry (corrected; the teardown source of truth)

| Port(s) | What |
|---|---|
| `19443` / `19543` / `19643` | pocx / btc / ltc node RPC |
| `18443` / `18332` | REST/bindex-hardcoded node RPC (nodeless: pocx regtest-default, btc testnet-default) |
| `19750/19751` (+fleet `19752`–`19757`) | PoCX electrs electrum/monitoring (fleet steps by 2) |
| `19760/19761` | vanilla (BTC) electrs |
| `19737`–`19749` | **pactd allocation range** (bots + e2e parties). `_alloc_port` starts at 19737 and today rolls unbounded into the electrs range — the shared-Harness monolith burns ~48 ports per run, so the **cap at 19749 can only land with Phase 2's per-scenario allocator reset** (≤4 parties per scenario). |
| `19788` | **playground** Nostr relay |
| `19791` | **e2e-suite** Nostr relay (the old doc wrongly listed only 19788) |
| `19790` | corkboard |
| `9739` / `9740` | managed Satchel pactd (regtest) / observer second instance |
| `9747` | viewer pactd (mainnet-isolated) |
| `5173` | Vite (cargo tauri dev) |
| `19801`–`19803` | multimachine test |
| **NEVER** `9737` / `9738` | user's live mainnet / testnet pactd — structurally excluded from the registry, unit-tested |

(Deleted with spike_electrs.py: its pactd on 19752, which collided with the
electrs fleet range.)
