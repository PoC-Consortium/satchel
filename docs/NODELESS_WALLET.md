# Nodeless wallet — bdk + Electrum send/receive wallet (epic #58)

Design doc for the nodeless build: Satchel carries its own on-chain wallet
(bdk + Electrum) so a user needs **no full node** to receive, send, and swap.
Supersedes the read-only wallet scoping decision once shipped.

Status: design accepted 2026-07-03 (seed derivation, backend shape, electrs
strategy). Sub-issues tracked under #58.

---

## 1. Where the code already is

The abstraction work landed long before this epic; the gap is narrow and
well-defined.

- Every chain touch in the engine goes through `trait ChainBackend`
  (`libswap/src/chain.rs:59`) — 23 operations, no raw RPC anywhere else.
- `ElectrumBackend` (`chain.rs:753`) already implements the **entire
  chain-data half** over raw Electrum JSON (raw `raw_call` only, because PoCX
  headers are 286 bytes and break typed clients — `chain.rs:745-752`):
  funding discovery, spend-witness/preimage extraction, MTP clocks,
  confirmations, broadcast, fee estimates.
- The gap is exactly the **nine `wallet_*` methods** (`chain.rs:139-213`),
  today implemented only by `CoreRpcBackend` and routed to `backends[0]` by
  `MultiBackend` (`chain.rs:1091`): `wallet_new_address`, `wallet_balance`,
  `wallet_send`, `wallet_build_funding`, `wallet_locked`, `wallet_sign_send`,
  `wallet_tx_fee_vsize`, `wallet_change_output`, `wallet_bumpfee`.
- `bdk_wallet = "1"` is a declared, unused workspace dependency, reserved for
  this work (`engine.rs:6198-6201` calls it out explicitly).
- pactd already exposes `getbalance` / `getnewaddress` / `sendtoaddress`
  (pass-through to the Core primary). Seed create/import/unlock/encrypt
  exists in `store.rs` and is UI-wired (`SeedForm` / `Unlock`).
- Net-new surface: a transaction-history RPC, the send/receive/activity UI
  (WalletScreen is read-only by design today), and a nodeless first-run path.

The swap engine itself — v1, v2, both fee nurses, rescue rediscovery —
**changes zero lines**: it sees the same trait.

## 2. Decisions

### D1 — Wallet keys: same mnemonic, standard BIP-86 branch

The on-chain wallet derives from the **same BIP39 mnemonic** as the Pact
seed, under **standard BIP-86 paths**, not under the Pact purpose:

```
m/86'/<bip32_coin_type>'/0'   (external + internal keychains)
```

- `bip32_coin_type` comes from the registry (`registry.rs:76`; BTC = 0,
  PoCX = 0x504F4358 — fits a 31-bit hardened index; file coins declare their
  own, e.g. LTC = 2).
- The Pact tree `m/7228'/{0..3}'/…` (`keys.rs`) is untouched; the branches
  cannot collide (different purpose).
- One mnemonic = one backup covering identity **and** funds; the funds side
  is recoverable in any standard descriptor wallet.
- Taproot (BIP-86) everywhere: every supported coin requires the modern
  script set already; single-keychain-pair, no legacy descriptors
  (no-backward-compat principle — this wallet never shipped).

**Consequence to document loudly:** in nodeless mode the seed is no longer
"hot transit only" (`keys.rs:58`) — it holds funds. Passphrase encryption
moves from recommended to strongly recommended; handbook ch11/ch19, wiki
Satchel guide, and the README claim all need the update when this ships.

### D2 — Backend shape: `BdkWalletBackend` = bdk wallet ⊕ existing raw Electrum

A new backend implements the **full** `ChainBackend` trait by composition:

- **Chain reads** delegate to the existing raw `ElectrumBackend` (unchanged).
- **Wallet ops** are served by a `bdk_wallet::Wallet`: keychain tracking,
  UTXO set, coin selection, tx building, signing.

bdk is used at the **script level only**. Address encode/decode stays in
`ChainParams` (`params.rs` — already handles PoCX/LTC HRPs); block hashes
pass through bdk as opaque 32 bytes, so PoCX's non-standard header hash
never reaches bdk.

**Lifecycle:** `Engine::backend()` builds backends fresh per call
(`engine.rs:568`), but a bdk wallet is stateful (sync position, revealed
indexes, persistence). The engine gains a per-coin **wallet manager**
(`Arc<Mutex<…>>`-cached `bdk_wallet::Wallet` + persister in the merchant
data dir, keyed by coin id); the per-call backend borrows a handle. The
persister is bdk's SQLite store at `<data_dir>/wallet/<coin_id>.sqlite`.

### D3 — No bdk fork: a PoCX chain-source instead

Stock `bdk_wallet` core is already PoCX-compatible (rust-bitcoin
`Transaction`/`Script` are shared). What breaks is only the glue:
`bdk_electrum` → `electrum-client`'s typed API parses 80-byte headers.

So: **do not fork bdk.** Write our own chain-source — a `full_scan`/`sync`
implementation feeding `bdk_wallet` through its public update API, reusing
the raw-Electrum machinery from `chain.rs`. It lives in the workspace first;
when other PoCX wallets need it, it extracts into a published `bdk-pocx`
companion crate (the ecosystem-standard shape for alt-chain bdk support).
If deeper divergence ever appears, that crate is the natural seed of a real
fork — with the interface already proven.

### D4 — electrs strategy: mod latest as the community server; explorer fork bootstraps

(User decision 2026-07-03.) Two PoCX electrs candidates exist; both stay:

- **Canonical wallet-facing server:** a fresh minimal PoCX patch series on
  **latest upstream electrs** (header deserialization/hashing + network
  magic; the explorer's custom transaction recognition is cosmetic and NOT
  ported). Lightweight, community-runnable — which is what the ≥2-backend
  safety model actually needs (two *independent operators*, not two URLs).
- **Bootstrap/test backend:** the existing explorer electrs fork already
  speaks Electrum RPC against PoCX; it is backend #1 for the spike, the
  first e2e, and launch.

pact/satchel speak only the Electrum wire protocol, so the two servers are
interchangeable; nothing client-side depends on this choice. BTC/LTC legs
use existing public Electrum servers.

### D5 — Config shape: Electrum-only URL list ⇒ nodeless mode

A coin whose `--coin id=url[,url]` list contains **no `http://` Core-RPC
primary** (i.e. `tcp://`/`ssl://` only) runs in nodeless-wallet mode:
`MultiBackend::new` (`chain.rs:1070`) puts a `BdkWalletBackend` at
`backends[0]` wrapping the Electrum URLs. No new flag, no parallel route.
The existing plumbing (`--coin` repeatable flag, Satchel coin config,
`coins.toml` with `header_format`/`magic` already specified for the light
path) carries this unchanged. **Nodeless coins require ≥2 Electrum URLs**
(enforced at config validation) — the `MultiBackend` agreement machinery
(script+value agreement, min-confirmations, min/max MTP clocks) already
exists for exactly this.

`wallet_locked` maps to the **seed** lock state (encrypted seed, passphrase
not held) — the funds gate (`ensure_can_fund`) works unchanged.

### D6 — Accepted degradations (Electrum vs Core)

Funds safety is untouched: the design already treats every backend as an
untrusted hint, with local re-verification and clock-driven refunds
(`chain.rs:1-9`). What degrades is fee-bumping finesse:

| Operation | Core | Electrum-only | Impact / mitigation |
|---|---|---|---|
| `is_in_mempool` | `getmempoolentry` | blind `true` | evicted-only re-anchor never fires; the deadline-driven bump path still escalates. Consider `scripthash.get_mempool` as a partial signal. |
| `incremental_relay_feerate` | `getmempoolinfo` | constant 1 | BIP125 rule-4 floor is conservative-enough; keep. |
| `fee_rate_for(…, conservative)` | `estimatesmartfee CONSERVATIVE` | `estimatefee` | apply an explicit escalation multiplier near deadlines instead. |
| `find_spend_witness` mempool scan | `getrawmempool` enumeration | `scripthash.get_history` (includes mempool) | equivalent for our watched scripts; already implemented. |
| block-scan fallback | `getblock v2` | none (history is per-script) | not needed: all watched outputs are script-known. |

### D7 — Wallet exclusivity: one wallet per coin, user picks the world

**PRINCIPLE (user, 2026-07-04).** Per coin there is exactly ONE wallet, chosen
by the connection mode and never mixed: **RPC ⇒ the node's wallet** (funds
swaps, receives sweeps), **Electrum ⇒ the pact-seed bdk wallet**. Funding
comes from and proceeds return to the same wallet, so working capital cycles
hands-off in either world. Enforced in `Engine::nodeless_backend`
(Electrum-first lists must be Electrum-only) and `compose_chain_data`
(pact-seed mode refuses non-Electrum URLs). Scope notes: Electrum URLs behind
a Core primary stay read-only CHAIN VIEWS (custody exclusivity, not transport
exclusivity), and in-flight swap keys always live on the seed in both worlds —
the cut governs where UTXOs rest. This closes **O3** ("sweep node-coin
proceeds to the seed wallet?") as NO by principle: who wants proceeds on the
seed switches the coin to Electrum mode wholesale. Known follow-up: warn on a
mode switch while the now-hidden wallet still holds a balance.

## 3. The nine `wallet_*` methods on bdk

| Trait method | Used by | bdk implementation |
|---|---|---|
| `wallet_new_address` | sweeps, CPFP outputs | `reveal_next_address` (external) + persist; encode via `ChainParams` |
| `wallet_balance` | funds gate | `balance().trusted_spendable()` after sync |
| `wallet_send` | v1 fund, v2 leg-A fund | `TxBuilder` (RBF on, explicit feerate) → sign → broadcast via backend |
| `wallet_build_funding` | v2 leg-B (build, DON'T broadcast) | `TxBuilder` → sign → **persist tx, mark UTXOs reserved, return without broadcast** (broadcast later by the scheduler tick, engine.rs:2426) |
| `wallet_locked` | funds gate | seed lock state (D5) |
| `wallet_sign_send` | v2 CPFP child | child spends our own sweep/change output: insert parent into the wallet graph, sign, broadcast |
| `wallet_tx_fee_vsize` | funding nurses | `calculate_fee` + `tx.vsize()` from the local graph |
| `wallet_change_output` | v2 funding CPFP | our own tx: identify the non-HTLC output owned by the keychain (`is_mine`) |
| `wallet_bumpfee` | v1 funding RBF nurse | `build_fee_bump(txid)` → sign → broadcast |

The v2 leg-B build-without-broadcast is the correctness-critical one: the
pre-signed adaptor signatures bind to the funding outpoint, so the built tx
must be persisted (and its inputs locked, bdk's equivalent of
`lockUnspents`) before the handshake continues — mirroring the Core path's
persist-before-broadcast discipline and the rc6 commit rule.

### 3.1 Sync model: background worker, cached reads (step 2, issue #87)

The wallet ops above perform **no chain I/O of their own**. Step 1 (#86)
batched the sync to a few round-trips; step 2 moved it off the RPC path
entirely:

- One `SyncWorker` thread per nodeless coin (the extracted `electrum-btcx`
  crate's `worker` module; debug trace via `BTCX_WALLET_SYNC_TRACE`), owning
  the coin's **one long-lived Electrum connection** (an `ElectrumPool`
  entry shared with every engine call; lazy, reconnects on transport
  errors, `verify_chain` cached per connection generation).
- Loop: `wait(poke OR ~15s tick)` → observe (scripthash subscriptions on
  every revealed spk + one tip poll; unchanged statuses skip the sync) →
  snapshot revealed spks under a brief entry lock → fetch batched with NO
  locks → `apply_update` + persist under a brief entry lock.
- Reads (`wallet_balance`, `wallet_transactions`, …) serve the bdk cache
  as-is: ~0ms, never network, stale-not-slow when the server is down.
  Freshness = worker cadence + pokes (own broadcasts, Receive dialog,
  swap events) + `scripthash.subscribe` push notifications.
- Writes (`wallet_send`, `wallet_build_funding`, `wallet_bumpfee`, …) gate
  on the worker's `first_sync_done` latch (bounded wait, honest error) so
  a spend can never coin-select from a never-synced cache at boot.
- Race-safety: all wallet mutation stays under the per-coin entry mutex;
  the snapshot→fetch→apply gap is what bdk's monotonic `Update` merge is
  designed for (a non-connecting chain update is rejected and retried).

## 4. New surface beyond the trait

- **pactd RPC:** `listtransactions <coin>` (activity: txid, direction,
  amount, fee, confirmations, timestamp) — net-new, straight off the bdk tx
  graph. `getnewaddress`/`sendtoaddress`/`getbalance` gain nodeless
  implementations for free via the trait.
- **Satchel:** WalletScreen grows send / receive / activity when the coin is
  nodeless (read-only stays for Core-backed coins). First-run gate changes
  from "≥2 live nodes" to "≥2 live coins" where a coin is live via a node
  *or* via ≥2 Electrum servers. All new copy through `en.ts` + 25 locales
  (i18next/no-literal-string is enforced).
- **Docs:** handbook ch11/ch19, wiki (Configuring-Coins, FAQ, Satchel
  guide), README — the "read-only by design / nodeless is future" claims all
  flip.

## 5. Testing

- **Unit/component:** `BdkWalletBackend` against electrs on regtest;
  deterministic derivation vectors for the BIP-86 tree next to the existing
  spec vectors.
- **e2e parity:** the full regtest suite runs v1+v2 swaps
  nodeless↔nodeless and nodeless↔Core, including funding-nurse RBF/CPFP,
  locked-seed gating, and rescue rediscovery (`find_funding` re-adoption).
- **Infra:** electrs (explorer fork first) against the regtest nodes; the
  playground gains an electrs leg. Windows build story: under
  investigation — native if the fork builds on Windows, otherwise
  Docker/WSL wrapper (open question O1).

## 6. Sub-issue decomposition (execution order)

1. **Spike:** stock `bdk_wallet` + custom raw-Electrum chain source against
   PoCX regtest via the explorer electrs fork. De-risks D3; validates the
   fork's Electrum RPC on regtest.
2. **Infra:** electrs regtest integration + playground leg (+ Windows
   answer). Parallel track: minimal PoCX patch series on latest electrs.
3. **libswap:** `BdkWalletBackend` (all nine `wallet_*`), wallet manager +
   persistence, derivation vectors.
4. **pactd:** nodeless-mode detection (D5), seed-lock ⇒ `wallet_locked`,
   `listtransactions`, config validation (≥2 Electrum URLs).
5. **Satchel:** send/receive/activity UI, wizard nodeless path, i18n ×26.
6. **e2e parity suite** (§5) — gates mainnet enablement of nodeless mode.
7. **Hardening (later):** multi-server policy tuning, Electrum-over-Tor,
   macOS packaging.

## 7. Open questions

- **O1** — electrs on Windows: native build vs Docker/WSL for the
  playground (being checked now).
- **O2** — gap-limit / full-scan policy for restores (bdk default stop-gap
  vs a Satchel "deep rescan" affordance).
- **O3 — CLOSED by D7** (wallet exclusivity: no seed-wallet sweeps for
  node-backed coins). Original question: whether the nodeless wallet should also serve as the *sweep
  target* for Core-backed coins (today sweeps go to the Core wallet;
  keeping that unchanged is the default).
