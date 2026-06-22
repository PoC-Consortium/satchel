# Testing & the Regtest Harness

Pact is tested at two levels: fast in-process Rust tests (unit logic plus
spec-vector conformance), and a Python regtest harness that drives real `pactd`
daemons and real regtest nodes through complete swaps. This chapter inventories
both and shows how to run them.

## The cargo test suite

`cargo test` over the workspace runs the unit tests embedded in each crate
(canonical JSON, sign/verify, sealing, slip codec, the Noticeboard buffers, and
the swap state machines) plus the **protocol vector tests**. The vector tests pin
the wire/script construction against frozen fixtures so an accidental change to a
script, key path, or hash is caught immediately:

| Test file | Pins |
|---|---|
| `pact/libswap/tests/vectors.rs` | `spec/vectors/htlc_v1.json` (v1 HTLC). |
| `pact/libswap/tests/vectors_v2.rs` | `spec/vectors/htlc_v2.json` (v2 adaptor). |

Regenerate the vectors with
`cargo run -p libswap --example gen-vectors` and
`cargo run -p libswap --example gen-vectors-v2` (in `pact/`); the tests then
re-pin them.

```sh
cargo test --workspace      # unit + vector tests
cargo clippy --workspace    # lints
```

## The Python harness

The harness lives in `pact/harness/`, is Python 3.10+ stdlib-only, and is built
around `regtest_harness.py`:

- It brings up a **Bitcoin PoCX (btcx)** regtest node and a **Bitcoin (btc)**
  regtest node — always; an optional **Litecoin (ltc)** node starts only with
  `Harness(with_ltc=True)`.
- It funds one wallet per party per chain (e.g. `alice_pocx` funded /
  `bob_pocx` empty; `bob_btc` funded / `alice_btc` empty).
- It keeps both chains on a **shared mock clock** (`setmocktime`) and exposes
  `advance_time()` to push median-time-past (MTP) forward — this is how
  CLTV/timelock and refund behaviour is driven deterministically.
- Genesis hashes are asserted at startup, so a mis-pointed daemon binary fails
  loudly instead of silently testing the wrong chain.

The two end-to-end suites build the cargo workspace (`pactd`, `pact-cli`) and the
Corkboard first, then run every scenario against a single shared `Harness`.

### v1 HTLC scenarios — `test_swap_e2e.py`

| Scenario | What it proves |
|---|---|
| `test_complete_swap` | Happy-path manual swap: two `pactd`s driven by `pact-cli` through the file handshake; Bob extracts the preimage from the chain (no courtesy message). HTLC outpoints spent on-chain, both parties received the other asset. |
| `test_refund` | Both fund, Alice goes silent, clocks pass T1, both refund. Negative checks: premature refunds and a late redeem past T2 are rejected (§7.4). |
| `test_daemon_autopilot_swap` | Alice drives everything over `pactd` JSON-RPC (cookie auth; an unauthenticated call is rejected 401); duplicated backends (multi-backend path); the scheduler auto-redeems and RBF-bumps Alice's unconfirmed redeem; books `completed` on confirmation. |
| `test_daemon_autopilot_refund` | Both parties walk away after funding; the `tick` scheduler alone reclaims both legs after the timelocks — and does nothing before them. |
| `test_chain_watched_funding` | `funded` messages are withheld; the swap still completes via on-chain funding discovery. |
| `test_funding_fee_bump_v1` | The v1 RBF funding nurse: funding is pinned under the fee fallback, then ticks bump it — asserts a `funding-fee-bump` event and that the funding txid is replaced; completes via chain-watched detection (the RBF replacement is invisible to the counterparty). |
| `test_balance_validation` | Make-offer / take balance checks reject under-funded actions. |
| `test_create_import_then_swap` | Seedless start: Alice `createseed`, Bob `importseed` (encrypted) via the wizard's seed-lifecycle RPCs, then a normal swap. |
| `test_coin_setup` | `listcoins` / `listpairs` / `validatecoin`: configured + connected + genesis state, capability-derived pair availability, and the genesis-hash gate (accepts the right node, rejects a cross-wired one). |
| `test_corkboard_swap` | Maker posts a signed offer, taker takes it, the whole handshake travels through the blind relay to completion. Covers offer withdraw, a competing second take (served-once + reject + auto-delist), and asserts every relay blob is sealed `PACTSEALED1:` ciphertext, never plaintext. |
| `test_board_reset_recovery` | A board reset / stale cursor is recovered (serve-from-0 hygiene). |
| `test_nostr_relay_swap` | A swap over a **live local Nostr relay**, offers and relay mail flowing over Nostr alone. |
| `test_private_offer_swap` | Off-market: `makeprivateoffer` produces a `pactoffer1:` slip posted to no board; `takeoffer <slip>` relays the take and the swap auto-completes with no board listing. |

### v2 adaptor scenarios — `test_adaptor_swap.py`

| Scenario | What it proves |
|---|---|
| `test_adaptor_swap` | Happy-path v2 over RPC: the full adaptor lifecycle (`adaptorinit` → `adaptoraccept` → `adaptorfund` → `adaptornonces` → `adaptorsign` → `adaptorassemble` → `adaptorredeem`), cooperative MuSig2 key-path redeem. |
| `test_adaptor_refund` | Single-key CLTV-tapleaf refund (script-path, no MuSig2, unattended-safe). |
| `test_adaptor_refund_feebump` | The single-key refund is RBF-bumpable (deterministic re-sign). |
| `test_adaptor_redeem_cpfp` | CPFP child bumps an unbumpable cooperative redeem on BTC (the key-path redeem fee is sealed; a self-funded child spends the redeem's own output). |
| `test_adaptor_redeem_cpfp_ltc` | The same CPFP redeem-bump on litecoind — the first v2 swap on LTC. |
| `test_adaptor_funding_cpfp` | The v2 CPFP-via-change funding nurse: drives to `Signed` with leg-A funding unconfirmed, then a tick emits `funding-cpfp-bump` — asserts the child spends the funding's change while the leg-A swap outpoint stays unchanged; the package is mined and the swap completes (adaptor sigs over the unchanged outpoint still redeem). |
| `test_adaptor_depth_gate` | The reveal/redeem gate fires at the configured `--coin-confs` confirmation depth. |
| `test_adaptor_corkboard_swap` | A board-driven v2 swap (a PoCX↔BTC pair off-mainnet defaults to `pact-htlc-v2`, so a plain `boardpostoffer` posts a v2 offer). |

## Running the suites

```sh
cd pact/harness

# infrastructure only (nodes start, mine, mocktime works)
python regtest_harness.py --smoke

# v1 (HTLC) end-to-end suite
python test_swap_e2e.py

# v2 (Taproot/MuSig2 adaptor) end-to-end suite
python test_adaptor_swap.py
```

The harness resolves node binaries from `$POCX_BITCOIND` / `$BTC_BITCOIND` (or
`bin/`), so copy your installed daemons into `pact/harness/bin/` first. PoCX
regtest refuses rapid blocks unless mocktime is active, which is why the harness
always sets and advances mocktime on both chains.

## The playground scripts

For a full local stack you can click through (rather than assert against), use
the PowerShell playground scripts in `tools/`:

| Script | Brings up |
|---|---|
| `tools/playground-cork.ps1` | Regtest PoCX + BTC nodes, a **Corkboard**, two headless counterparties posting a two-sided book, and Satchel launched as a funded "Alice". Offers on PoCX↔BTC default to v2. |
| `tools/playground-nostr.ps1` | The same, but **relays-only**: no Corkboard, a single local ephemeral Nostr relay, Satchel with a relays-only `satchel.json`. Proves offers flow over Nostr alone. |

Both block on the Satchel window and tear the whole stack down (PID/port-only)
when you close it; `tools/knockdown.ps1` force-tears a stale run. Logs land in
`<repo>\.playground\`.

> **Warning** — Teardown is **PID- and port-only**, never by process name. A live
> mainnet `pocx-bitcoind` must not be killed by a name-based sweep; the
> playground nodes run on dedicated ports the teardown targets explicitly.
