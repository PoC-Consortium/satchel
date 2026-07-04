# Nodeless wallet (epic #58) — branch status & resume notes

Branch: `nodeless-wallet`, draft PR #67. Design:
[`NODELESS_WALLET.md`](NODELESS_WALLET.md); overnight-run decisions:
[`NODELESS_DECISIONS.md`](NODELESS_DECISIONS.md).

## State after the 2026-07-03/04 overnight run — REVIEW-READY

Everything through the playground is DONE and verified:

- **libswap foundation + live spike GREEN** (bdk 2.4, electrs, v31 node).
- **Phantom-funding release + listtransactions** (see "Done since the
  pause" below).
- **E2E parity suite GREEN** (`test_nodeless_e2e.py`, 4 scenarios: v1
  nodeless maker, v2 nodeless taker, v2 cancel releases inputs LIVE,
  v1 nodeless↔nodeless).
- **Satchel UI**: Wallets screen send/receive/activity for nodeless coins,
  "pact seed wallet" label, CoinSetup nodeless (Electrum) mode; wizard
  inherits it. i18n: en + 25 locales (new keys optional in Bundle).
- **Playground**: `tools/playground-nodeless.ps1` — Alice's BTCX is
  NODELESS over electrs, faucet auto-funds her pact-seed wallet after the
  wizard; BTC + LTC stay node-backed. Dress-rehearsed headless: a v2 board
  take gave 47 BTCX from the bdk wallet and completed. The classic
  all-Core `playground-cork.ps1` is untouched (both share
  `satchel_playground.py`; the nodeless variant passes `--nodeless`).

Still open after review: pactd deep-rescan affordance (O2), sweep-target
question (O3), handbook/wiki flip (+1 RPC method count), sub-issues under
#58 (not filed — user decides), Electrum-over-Tor / multi-server policy
tuning (hardening, later). Upstream electrs-pocx wishes: a `--rest-url`
override (bindex hardcodes :18443 on regtest) and a fix for the
empty-index `headers.subscribe` panic (harness works around both).

## Landed on this branch

| Commit | What |
|---|---|
| `5759fc9` | design doc (decisions D1–D6, sub-issue plan §6) |
| `22f1cbb` | libswap foundation (~840 lines, all tests green, clippy clean) |

The foundation, concretely:

- **`keys.rs`** — `PactSeed::wallet_descriptors(coin_type)`: BIP-86 branch
  `m/86'/<bip32_coin_type>'/0'` off the same mnemonic (BIP39 passphrase
  always `""`, matching `store.seed()`), as `tr(…/0/*)` + `tr(…/1/*)`
  private descriptors. Unit-pinned to the **official BIP-86 test vectors**
  (account xprv + first two receiving addresses).
- **`wallet_bdk.rs`** (new) —
  - `sync_entry` / `chain_update`: the unforked-bdk chain source. Feeds
    `bdk_wallet` 1.2 from the raw Electrum calls in `chain.rs`
    (scripthash histories, `header_at` for anchors/checkpoints) so PoCX's
    286-byte headers never reach bdk. Fresh store ⇒ STOP_GAP(20) full
    scan; steady state ⇒ revealed spks only.
  - `WalletManager`: per-coin cached bdk wallet, sqlite persister at
    `<data_dir>/wallet/<coin_id>.sqlite`, genesis-hash + descriptor
    checked at load (wrong seed refuses).
  - `BdkWalletBackend`: full `ChainBackend`. Chain reads delegate to the
    wrapped `ElectrumBackend`; all nine `wallet_*` ops served by bdk —
    including the v2 `wallet_build_funding` (build-sign-persist, NO
    broadcast, inputs locked by inserting the unbroadcast tx) and the
    CPFP `wallet_sign_send` (floating-txout fallback for a not-yet-synced
    parent).
- **Engine wiring** — `Engine::backend` dispatches to `nodeless_backend`
  when a coin's URL list has no `http://` primary: `BdkWalletBackend` at
  `backends[0]`, remaining URLs as independent Electrum views. Mainnet
  requires ≥2 URLs. Locked/absent seed ⇒ chain-reads-only +
  `wallet_locked() == true` (the existing funds gate handles the rest).
  `MultiBackend::from_backends` added; swap-engine logic untouched.
- `bdk_wallet` workspace dep gained the `rusqlite` feature (0.31 — same
  pin as our own rusqlite, single bundled libsqlite3-sys).

## Resume here (in order)

1. **electrs answer (O1).** Does the PoCX electrs build/run on Windows?
   → native playground leg, else Docker/WSL wrapper.
2. **The live spike** — first real test of the chain source
   (`sync_entry` has never talked to a server; everything below it is
   unit-tested):
   - electrs against a regtest PoCX node;
   - `pactd --coin btcx=tcp://127.0.0.1:<port>` (regtest allows a single
     URL);
   - `getnewaddress` → mine to it → `getbalance` → `sendtoaddress` →
     re-sync → balance again. Then the same through a v1 regtest swap.
3. **File the sub-issues** under #58 (§6 of the design doc) once the
   spike confirms the shape.
4. Next code, roughly in order: Satchel send/receive/activity UI +
   wizard nodeless path (i18n ×26), regtest e2e parity suite.
   ~~`cancel_tx` wiring~~ and ~~pactd `listtransactions`~~ are DONE (see
   below).

## Done since the pause

- **bdk_wallet 1.2 → 2.4** (still bitcoin 0.32 / rusqlite 0.31, one-line
  sync churn: `seen_ats` is a set now). Motive: `apply_evicted_txs` — the
  only way to drop a never-broadcast tx from the canonical set.
- **Phantom-funding release** (`wallet_cancel_funding` on `ChainBackend`):
  bdk evicts the phantom + unmarks its change index; Core `lockunspent`s
  the built tx's inputs back; default no-op. The engine wires it into
  every path where a BUILT-but-unbroadcast leg B ends: user abort + peer
  abort (both now use *commitment* semantics — a built tx with its
  outpoint provably off-network and unspent no longer blocks cancel), the
  §7.4 fund-deadline dead-end in the `Signed` tick (used to idle forever),
  and `adaptor_refund_if_due` (used to error-loop refunding a nonexistent
  outpoint). Tick paths go terminal only once the release succeeds;
  `adaptor_leg_b_uncommitted` answers "committed" on any read error and
  refuses when the outpoint was ever spent (rescue corner: a pre-wipe
  broadcast + maker redeem must drive the claim, never an abort).
- **`listtransactions <coin>` RPC** + `pact-cli transactions <coin>`:
  activity off the bdk tx graph (txid, direction, net amount, fee, confs,
  timestamp), newest first; Core-backed coins refuse (read-only by
  design). Mapping lives in `wallet_activity` (pure, unit-tested).
- Test: `built_funding_reserves_inputs_and_cancel_releases_them` pins the
  whole reserve→activity→evict→release cycle against real bdk 2.4.

## Live spike (harness/spike_electrs.py) — **GREEN** (2026-07-03)

With a **PoCX v31.0.0rc1** node in `harness/bin/pocx-bitcoind.exe` (the
Phoenix PoCX node build), the full nodeless flow passes against live
electrs on native Windows: fresh-seed scan → `rpocx1p…` receive → mature
coinbase balance (PoCX 286-byte headers → anchors) → bdk-built send
broadcast over Electrum into the node mempool (143 sat fee) → steady-state
re-sync → correct `listtransactions` rows → pactd-restart persistence.
Design decision **D3 (no bdk fork) is validated on the wire.** Original
run notes below.

## Spike run notes (first attempt, superseded)

`python pact/harness/spike_electrs.py` drives the full nodeless flow against
a real electrs (fresh-seed scan → receive → balance → send → re-sync →
listtransactions → restart persistence). Needs `harness/bin/electrs.exe`
(gitignored, like the node binaries; `PACT_ELECTRS_BIN` overrides). Two
findings from the first run (2026-07-03):

1. **bindex-pocx hardcodes its REST URL** to
   `http://localhost:{default_rpc_port}` (18443 on regtest) —
   `--daemon-rpc-addr` does not move the indexer. The spike therefore runs
   its node on 18443 + `-rest=1` instead of the harness's isolated 19443.
   Worth a flag upstream (or deriving from `daemon_rpc_addr`).
2. ~~Blocked~~ **RESOLVED**: bindex needs `/rest/blockpart` (Bitcoin PR
   #33657, v31+); the harness's old node was v30.2.2 → 404. Fixed by
   upgrading `harness/bin/pocx-bitcoind.exe` to **v31.0.0rc1** (the e2e
   suite runs on the same binary — regression-ran after the swap).

## Known TODOs / sharp edges (all noted in code comments too)
- A fresh wallet with no history full-scans (2×20 scripthash calls) on
  every sync until the first address is revealed — harmless, worth a
  "scanned once" marker later.
- `Engine::coin_wallet` (Wallets-screen scope label) shows "default
  wallet (not scoped)" for a nodeless coin — Satchel UI work will want a
  "pact seed" label instead.
- Electrum degradations are per design (D6): blind `is_in_mempool`,
  constant incremental relay fee, no CONSERVATIVE estimate mode.
