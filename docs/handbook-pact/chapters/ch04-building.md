# Building from Source

Pact is a Rust workspace. Building the engine and running its tests needs only
the Rust toolchain; running the full end-to-end harness additionally needs
Python 3 and a regtest `bitcoin-pocx` build. This chapter covers building the
engine, running the test suites, building the Corkboard transport, and where the
binaries land.

## Prerequisites

- **Rust** (stable) with `cargo`. The whole engine — `libswap`, `pactd`, and
  `pact-cli` — builds with a standard stable toolchain; no nightly features are
  required.
- **Python 3**, for the end-to-end harness scripts under `pact/harness/`.
- A **`bitcoin-pocx` build** providing the regtest BTCX and BTC nodes the
  harness spins up. The harness drives real `bitcoind`-shaped nodes on regtest;
  it does not mock the chain.

> **Note** — The desktop app **Satchel** adds a Node/npm + Tauri toolchain on
> top of this. That build is covered in the *Satchel User Handbook*; this
> handbook concerns the engine and its transports only.

## Building and testing the engine

From the `pact/` directory:

```sh
cd pact
cargo build          # build libswap, pactd, pact-cli
cargo test           # unit tests + protocol-vector tests (v1 + v2)
```

`cargo test` runs the unit tests across the workspace and the deterministic
**protocol-vector** tests. Those vector tests
(`pact/libswap/tests/vectors.rs` for v1 and `vectors_v2.rs` for v2) pin the
on-chain construction against the JSON vectors in `spec/vectors/`
(`htlc_v1.json`, `htlc_v2.json`). If you change anything that touches script
construction, key derivation, or the adaptor mechanism, these are the tests that
must stay green.

## Running the end-to-end harness

The harness performs complete swaps on regtest against real nodes — the highest-fidelity
check that the engine works end to end:

```sh
python harness/test_swap_e2e.py      # full BTCX↔BTC v1 (HTLC) swap on regtest
python harness/test_adaptor_swap.py  # full v2 (Taproot/MuSig2 adaptor) swap
```

`test_swap_e2e.py` exercises the v1 flow: the manual two-CLI happy path, refund
(including premature- and late-reveal negatives), the daemon autopilot
(scheduler-driven auto-redeem and RBF fee-bump, and auto-refund on both sides),
chain-watched funding, balance validation, encrypted-seed create/import, coin
setup, and the Corkboard, Nostr-relay, and private-offer flows.

`test_adaptor_swap.py` exercises the v2 flow: the happy path, the single-key
CLTV-tapleaf refund and its fee-bump, the CPFP redeem-bump (on BTC and on
litecoind), the reveal depth-gate, and the v2 Corkboard flow.

> **Tip** — Both scripts share `regtest_harness.py`, which always brings up BTCX
> and BTC nodes and only adds an LTC node when constructed with
> `Harness(with_ltc=True)`. The harness uses `setmocktime` to advance median
> time on both chains, so timelock-dependent paths (refund, deadline gates) can
> be tested deterministically without waiting in real time.

## Building Corkboard

Corkboard is the self-hostable transport — a single `axum` + SQLite/Postgres
binary that stores signed offers and blind-relays sealed blobs. Build and run it
from its own directory:

```sh
cd corkboard
cargo build
cargo run -- --listen 127.0.0.1:9780 --db corkboard.sqlite
```

Its default listen address is `127.0.0.1:9780`. Corkboard is optional: the
default transport is **Nostr**, which needs no infrastructure — you simply point
`pactd` at relays. See the transports part of this handbook for how to wire each
one.

## Where the binaries land

`cargo build` produces unoptimised binaries under the workspace `target/debug/`
directory; `cargo build --release` produces optimised ones under
`target/release/`. From the `pact/` workspace you get `pactd` and `pact-cli`;
from `corkboard/` you get the `corkboard` binary. You can run any of them in
place with `cargo run -p <crate> -- <args>` without copying the binary out — for
example:

```sh
cargo run -p pactd -- --data-dir ./data --coin btcx=<rpc-url> --coin btc=<rpc-or-electrum-url>
cargo run -p pact-cli -- --data-dir ./data getinfo
```

The next chapter, *Running pactd*, covers the daemon's full command line, RPC
authentication, and the scheduler in detail.
