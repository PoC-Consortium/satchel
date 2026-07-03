# Nodeless wallet (epic #58) — branch status & resume notes

Branch: `nodeless-wallet`. Design: [`NODELESS_WALLET.md`](NODELESS_WALLET.md).
Paused 2026-07-03 waiting on the PoCX electrs patch (romanz latest fork —
being built/tested for Windows separately).

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

## Known TODOs / sharp edges (all noted in code comments too)
- A fresh wallet with no history full-scans (2×20 scripthash calls) on
  every sync until the first address is revealed — harmless, worth a
  "scanned once" marker later.
- `Engine::coin_wallet` (Wallets-screen scope label) shows "default
  wallet (not scoped)" for a nodeless coin — Satchel UI work will want a
  "pact seed" label instead.
- Electrum degradations are per design (D6): blind `is_in_mempool`,
  constant incremental relay fee, no CONSERVATIVE estimate mode.
