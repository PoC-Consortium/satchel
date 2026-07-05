//! Nodeless on-chain wallet — bdk over the raw Electrum machinery
//! (docs/NODELESS_WALLET.md, epic #58).
//!
//! The wallet side of a nodeless coin: a `bdk_wallet::Wallet` derived from
//! the SAME mnemonic as the Pact seed (the BIP-86 branch of
//! [`crate::keys::PactSeed::wallet_descriptors`]), synced over the
//! PoCX-header-safe raw Electrum calls of [`ElectrumBackend`], and
//! persisted per coin in the merchant data dir. [`BdkWalletBackend`]
//! implements the full [`ChainBackend`] trait: chain reads delegate to the
//! wrapped Electrum backend, and the nine `wallet_*` operations — the ones
//! only a Core-RPC wallet URL could serve until now — are served by bdk.
//!
//! Syncing is a BACKGROUND job (issue #87): the per-coin
//! [`crate::wallet_worker::SyncWorker`] keeps the bdk cache fresh over the
//! coin's one long-lived Electrum connection, so the `wallet_*` operations
//! here never perform chain I/O for the wallet's own state — reads serve
//! the cache as-is, writes only gate on the worker's first-sync latch.
//!
//! bdk is used at the *script* level only (design doc D2/D3):
//! - Address encode/decode goes through [`ChainParams`] (PoCX/LTC HRPs).
//!   The `bitcoin::Network` handed to bdk is the constant
//!   [`bitcoin::Network::Bitcoin`] — it only affects xprv serialization
//!   (always `NetworkKind::Main`, see [`crate::keys::PactSeed::from_seed`])
//!   and bdk's own address strings, which are never used. The real chain
//!   binding is the per-coin genesis-hash checkpoint set at creation and
//!   checked at load.
//! - Raw headers never reach bdk: anchors and checkpoints are built from
//!   header bytes hashed via [`ChainParams::header_hash`], so PoCX's
//!   286-byte headers are handled exactly like everywhere else in the
//!   engine, and stock upstream bdk needs no fork.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Context, Result};
use bdk_wallet::chain::{BlockId, ChainPosition, CheckPoint, ConfirmationBlockTime, TxUpdate};
use bdk_wallet::rusqlite::Connection;
use bdk_wallet::{KeychainKind, PersistedWallet, SignOptions, Update, Wallet};
use bitcoin::{
    Amount, BlockHash, FeeRate, OutPoint, Psbt, ScriptBuf, Sequence, Transaction, TxOut, Txid,
};

use crate::chain::{ChainBackend, ElectrumBackend, SendFee, TxOutInfo, WalletTxInfo};
use crate::keys::PactSeed;
use crate::params::ChainParams;
use crate::registry;
use crate::wallet_worker::{SyncWorker, FIRST_SYNC_WAIT};

/// BIP-44 gap limit for the initial full scan of a restored seed. Every
/// address this wallet hands out is revealed-then-persisted, so steady-state
/// syncs never probe beyond the revealed set; the gap only matters when the
/// sqlite store is fresh for a seed that may have on-chain history
/// (restore on a new machine). Because address HANDOUT is capped (see
/// [`MAX_UNUSED_AHEAD`]), the real on-chain gap can never exceed that cap —
/// this scan width carries a safety margin on top, making a restore's
/// full scan complete BY CONSTRUCTION. Closes design doc O2 (no deep-rescan
/// affordance needed).
const STOP_GAP: u32 = 25;

/// Electrum-style handout cap: never let more than this many revealed-but-
/// unused external addresses accumulate. Past the cap, [`wallet_handout_spk`]
/// recycles the OLDEST unused address instead of revealing further — so a
/// pathological "clicked Receive 30 times, never got paid" session cannot
/// open a gap a restore's [`STOP_GAP`] scan would miss. Bitcoin Core solves
/// the same problem with brute width (a 1000-key keypool); Electrum's cap
/// gets the same guarantee without thousand-query restores. Reuse only ever
/// means "shown twice", and only past 20 outstanding unpaid addresses.
const MAX_UNUSED_AHEAD: usize = 20;

fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Encode a keychain scriptPubKey as an address string via [`ChainParams`]
/// (never via bdk/rust-bitcoin `Network` HRPs). The nodeless wallet is
/// BIP-86, so every keychain spk is P2TR.
fn spk_to_address(params: &ChainParams, spk: &ScriptBuf) -> Result<String> {
    anyhow::ensure!(
        spk.is_p2tr(),
        "nodeless wallet script is not P2TR: {}",
        hex::encode(spk.as_bytes())
    );
    let xonly = bitcoin::XOnlyPublicKey::from_slice(&spk.as_bytes()[2..])
        .context("P2TR output key in keychain spk")?;
    params.p2tr_address(&xonly)
}

/// One coin's open wallet: the bdk wallet plus the sqlite connection it
/// persists into. Always lock the pair together (single mutex in
/// [`WalletManager`]) — persisting after every mutation is what makes a
/// crash lose nothing but re-syncable chain data.
pub struct WalletEntry {
    pub(crate) wallet: PersistedWallet<Connection>,
    pub(crate) conn: Connection,
}

/// Shared handle to one coin's wallet.
pub type WalletHandle = Arc<Mutex<WalletEntry>>;

/// Per-coin cache of open bdk wallets. The engine holds one of these for
/// the merchant data dir: bdk wallets are stateful (sync position, revealed
/// indexes, sqlite store), so unlike the stateless RPC backends they must
/// NOT be rebuilt per [`crate::engine::Engine::backend`] call — the
/// per-call [`BdkWalletBackend`] borrows a handle instead (design doc D2).
pub struct WalletManager {
    wallet_dir: PathBuf,
    wallets: Mutex<BTreeMap<String, WalletHandle>>,
    /// One background [`SyncWorker`] per open nodeless coin (issue #87).
    /// Spawned by [`Self::ensure_worker`], told to exit when the manager —
    /// i.e. the engine, i.e. the loaded merchant — goes away.
    workers: Mutex<BTreeMap<String, Arc<SyncWorker>>>,
}

impl WalletManager {
    /// `data_dir` is the merchant data dir; wallets live under
    /// `<data_dir>/wallet/<coin_id>.sqlite`.
    pub fn new(data_dir: &Path) -> Self {
        Self {
            wallet_dir: data_dir.join("wallet"),
            wallets: Mutex::new(BTreeMap::new()),
            workers: Mutex::new(BTreeMap::new()),
        }
    }

    /// The coin's background sync worker, spawning it on first call. The
    /// worker holds only a `Weak` wallet handle plus the shared chain
    /// backend, so it can never keep a dropped engine's wallet alive; a
    /// reconfigured server URL is handed over via `set_chain` (the worker
    /// picks it up on its next wakeup).
    pub fn ensure_worker(
        &self,
        coin_id: &str,
        chain: Arc<ElectrumBackend>,
        wallet: &WalletHandle,
    ) -> Arc<SyncWorker> {
        let mut workers = self.workers.lock().expect("worker map poisoned");
        if let Some(worker) = workers.get(coin_id) {
            worker.set_chain(chain);
            return worker.clone();
        }
        let worker = SyncWorker::spawn(coin_id, chain, wallet);
        workers.insert(coin_id.to_string(), worker.clone());
        worker
    }

    /// Open (or create) the nodeless wallet for `coin_id`, deriving its
    /// descriptors from `seed`. Loads honor-checked: descriptors AND the
    /// coin's genesis hash must match what the store was created with.
    pub fn open(
        &self,
        coin_id: &str,
        params: &'static ChainParams,
        seed: &PactSeed,
    ) -> Result<WalletHandle> {
        let mut wallets = self.wallets.lock().expect("wallet cache poisoned");
        if let Some(handle) = wallets.get(coin_id) {
            return Ok(handle.clone());
        }

        std::fs::create_dir_all(&self.wallet_dir)
            .with_context(|| format!("creating {}", self.wallet_dir.display()))?;
        let db_path = self.wallet_dir.join(format!("{coin_id}.sqlite"));
        let mut conn = Connection::open(&db_path)
            .with_context(|| format!("opening wallet db {}", db_path.display()))?;

        let coin_type = registry::bip32_coin_type(coin_id)?;
        let (external, internal) = seed.wallet_descriptors(coin_type)?;
        let genesis = BlockHash::from_str(params.genesis_hash)
            .context("coin genesis hash is not a block hash")?;

        let loaded = Wallet::load()
            .descriptor(KeychainKind::External, Some(external.clone()))
            .descriptor(KeychainKind::Internal, Some(internal.clone()))
            .extract_keys()
            .check_genesis_hash(genesis)
            .load_wallet(&mut conn)
            .map_err(|e| anyhow!("loading wallet db {}: {e}", db_path.display()))?;
        let wallet = match loaded {
            Some(wallet) => wallet,
            None => Wallet::create(external, internal)
                .network(bitcoin::Network::Bitcoin) // constant; see module doc
                .genesis_hash(genesis)
                .create_wallet(&mut conn)
                .map_err(|e| anyhow!("creating wallet db {}: {e}", db_path.display()))?,
        };

        let handle: WalletHandle = Arc::new(Mutex::new(WalletEntry { wallet, conn }));
        wallets.insert(coin_id.to_string(), handle.clone());
        Ok(handle)
    }
}

impl Drop for WalletManager {
    /// Merchant unload / pactd stop: tell every worker to exit. No join —
    /// a worker may be mid-fetch (bounded by the socket timeouts) and drop
    /// must not block; it re-checks the flag before its next wallet write
    /// and its `Weak` handle dies with this manager's map anyway.
    fn drop(&mut self) {
        if let Ok(workers) = self.workers.lock() {
            for worker in workers.values() {
                worker.shutdown();
            }
        }
    }
}

// ---- chain source: raw Electrum → bdk updates ------------------------------
//
// The sync is split snapshot → fetch → apply (issue #87): the wallet-entry
// lock is held only for the pure-CPU snapshot and the final apply+persist,
// NEVER across network I/O — the background sync worker
// ([`crate::wallet_worker::SyncWorker`]) runs the fetch while RPC reads keep
// serving from the cache. The snapshot→fetch→apply gap is what bdk's
// monotonic `Update` merge is designed for: a chain update that no longer
// connects is rejected (and retried next tick), never corrupts.

/// What the fetch phase needs from the wallet, captured under a brief
/// entry lock.
struct SpkSnapshot {
    /// Fresh store (nothing ever revealed) → gap-limit scan for a restored
    /// seed's history. Steady state → revealed spks only.
    full_scan: bool,
    revealed: Vec<(KeychainKind, Vec<ScriptBuf>)>,
    local_tip: CheckPoint,
}

fn snapshot_spks(entry: &WalletEntry) -> SpkSnapshot {
    let full_scan = entry
        .wallet
        .derivation_index(KeychainKind::External)
        .is_none();
    let mut revealed = Vec::new();
    if !full_scan {
        for keychain in [KeychainKind::External, KeychainKind::Internal] {
            if let Some(last) = entry.wallet.derivation_index(keychain) {
                let spks = (0..=last)
                    .map(|i| {
                        entry
                            .wallet
                            .peek_address(keychain, i)
                            .address
                            .script_pubkey()
                    })
                    .collect();
                revealed.push((keychain, spks));
            }
        }
    }
    SpkSnapshot {
        full_scan,
        revealed,
        local_tip: entry.wallet.latest_checkpoint(),
    }
}

/// Every revealed spk of both keychains — the set the sync worker keeps
/// scripthash subscriptions on. Brief pure-CPU derivation, no chain I/O.
pub(crate) fn revealed_spks(entry: &WalletEntry) -> Vec<ScriptBuf> {
    snapshot_spks(entry)
        .revealed
        .into_iter()
        .flat_map(|(_, spks)| spks)
        .collect()
}

/// Fetch phase: scripthash histories of the snapshot's spks (or a STOP_GAP
/// full scan when the store is fresh), PoCX-safe anchors from raw headers,
/// and a checkpoint update that always connects to the wallet's local chain
/// (genesis at worst). This is the unforked-bdk chain source of design doc
/// D3. Network I/O happens with NO wallet lock held; the full-scan windows
/// re-take it briefly for pure-CPU address derivation only.
fn fetch_update(
    chain: &ElectrumBackend,
    handle: &WalletHandle,
    snap: SpkSnapshot,
) -> Result<Update> {
    let params = chain.params();

    // Phase A — scripthash histories, BATCHED (one round-trip per batch;
    // pre-batching this was one round-trip PER ADDRESS, which took tens of
    // seconds against a remote server while holding the global engine lock).
    let mut last_active: BTreeMap<KeychainKind, u32> = BTreeMap::new();
    let mut all_history: Vec<(String, i64)> = Vec::new();
    if snap.full_scan {
        for keychain in [KeychainKind::External, KeychainKind::Internal] {
            // Windowed gap scan: STOP_GAP spks per batch, stop once a full
            // STOP_GAP run of consecutive unused spks has been seen —
            // identical result to the old per-spk walk (a window may peek a
            // few spks past the stop point; pure reads, harmless).
            let (mut index, mut gap) = (0u32, 0u32);
            'windows: loop {
                let spks: Vec<ScriptBuf> = {
                    let entry = handle.lock().expect("wallet entry poisoned");
                    (index..index + STOP_GAP)
                        .map(|i| {
                            entry
                                .wallet
                                .peek_address(keychain, i)
                                .address
                                .script_pubkey()
                        })
                        .collect()
                };
                for (offset, history) in chain.histories(&spks)?.into_iter().enumerate() {
                    if history.is_empty() {
                        gap += 1;
                        if gap >= STOP_GAP {
                            break 'windows;
                        }
                    } else {
                        last_active.insert(keychain, index + offset as u32);
                        gap = 0;
                        all_history.extend(history);
                    }
                }
                index += STOP_GAP;
            }
        }
    } else {
        for (_, spks) in &snap.revealed {
            for history in chain.histories(spks)? {
                all_history.extend(history);
            }
        }
    }

    // Phase B — sort the finds: which tx bodies we still need (deduped),
    // which heights need headers for anchors, what sits in the mempool.
    // One timestamp for the whole sync pass: `seen_ats` is a SET of
    // (txid, ts) pairs (bdk_chain 0.23), so a tx surfacing in several spk
    // histories must insert the identical pair to dedupe.
    let sync_ts = now_ts();
    let mut fetched: HashSet<Txid> = HashSet::new();
    let mut need_txs: Vec<String> = Vec::new();
    let mut anchor_reqs: Vec<(Txid, u32)> = Vec::new();
    let mut seen_ats: HashSet<(Txid, u64)> = HashSet::new();
    for (txid_hex, height) in &all_history {
        let txid = Txid::from_str(txid_hex).context("electrum history txid")?;
        if fetched.insert(txid) {
            need_txs.push(txid_hex.clone());
        }
        if *height > 0 {
            let height = u32::try_from(*height).context("history height")?;
            anchor_reqs.push((txid, height));
        } else {
            // 0 = mempool, -1 = mempool with unconfirmed parents.
            seen_ats.insert((txid, sync_ts));
        }
    }

    // Phase C — tx bodies, one batch. Phase D — headers, one batch.
    let mut tx_update = TxUpdate::<ConfirmationBlockTime>::default();
    for tx in chain.get_raw_txs(&need_txs)? {
        tx_update.txs.push(Arc::new(tx));
    }
    let need_heights: Vec<u64> = anchor_reqs
        .iter()
        .map(|(_, h)| u64::from(*h))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut headers: BTreeMap<u32, (BlockHash, u64)> = BTreeMap::new();
    for (height, (hash_hex, time)) in need_heights.iter().zip(chain.headers_at(&need_heights)?) {
        headers.insert(
            u32::try_from(*height).context("header height")?,
            (BlockHash::from_str(&hash_hex)?, u64::from(time)),
        );
    }
    for (txid, height) in anchor_reqs {
        let (hash, time) = headers[&height];
        tx_update.anchors.insert((
            ConfirmationBlockTime {
                block_id: BlockId { height, hash },
                confirmation_time: time,
            },
            txid,
        ));
    }
    tx_update.seen_ats = seen_ats;

    let chain_cp = chain_update(chain, params, snap.local_tip, &headers)?;
    Ok(Update {
        last_active_indices: last_active,
        tx_update,
        chain: Some(chain_cp),
    })
}

/// One full sync pass: snapshot (brief lock) → fetch (no locks) → apply +
/// persist (brief lock). `abort` is checked between fetch and apply so a
/// shutting-down worker never writes into a store its manager already let
/// go of (merchant unload).
pub(crate) fn sync_wallet(
    handle: &WalletHandle,
    chain: &ElectrumBackend,
    abort: impl Fn() -> bool,
) -> Result<()> {
    let snap = {
        let entry = handle.lock().expect("wallet entry poisoned");
        snapshot_spks(&entry)
    };
    let update = fetch_update(chain, handle, snap)?;
    if abort() {
        return Ok(());
    }
    let mut guard = handle.lock().expect("wallet entry poisoned");
    let entry = &mut *guard;
    entry
        .wallet
        .apply_update(update)
        .map_err(|e| anyhow!("bdk chain update does not connect: {e}"))?;
    entry
        .wallet
        .persist(&mut entry.conn)
        .map_err(|e| anyhow!("persisting wallet: {e}"))?;
    Ok(())
}

/// Build the checkpoint update: every anchored block, the server tip, and a
/// point of agreement with the wallet's existing chain. Walking the local
/// checkpoints tip-down, a stale (reorged) hash is replaced by the server's
/// view and the walk continues until agreement — height 0 agrees by
/// construction (both sides pin the coin's genesis).
fn chain_update(
    chain: &ElectrumBackend,
    params: &ChainParams,
    local_tip: CheckPoint,
    anchored: &BTreeMap<u32, (BlockHash, u64)>,
) -> Result<CheckPoint> {
    let (tip_height, tip_raw) = chain.tip()?;
    let tip_height = u32::try_from(tip_height).context("tip height")?;
    let tip_hash = BlockHash::from_str(&params.header_hash(&tip_raw)?)?;

    let mut blocks: BTreeMap<u32, BlockHash> =
        anchored.iter().map(|(h, (hash, _))| (*h, *hash)).collect();
    blocks.insert(tip_height, tip_hash);

    for cp in local_tip.iter() {
        let height = cp.height();
        if height > tip_height {
            continue; // server is behind our stored tip — let agreement decide
        }
        let server = match blocks.get(&height) {
            Some(hash) => *hash,
            None => {
                let (hash_hex, _) = chain.header_at(u64::from(height))?;
                let hash = BlockHash::from_str(&hash_hex)?;
                blocks.insert(height, hash);
                hash
            }
        };
        if server == cp.hash() {
            break; // point of agreement found — the update connects here
        }
        // Reorged: blocks already holds the server's hash, displacing ours.
    }

    CheckPoint::from_block_ids(
        blocks
            .into_iter()
            .map(|(height, hash)| BlockId { height, hash }),
    )
    .map_err(|_| anyhow!("checkpoint heights not strictly ascending"))
}

// ---- the backend ------------------------------------------------------------

/// The nodeless primary backend: chain data over Electrum, wallet over bdk.
/// Sits at `backends[0]` of a `MultiBackend` when a coin is configured with
/// Electrum URLs only (design doc D5). `wallet` is `None` while the seed is
/// locked — chain reads keep working, wallet operations report the lock.
///
/// Since issue #87 the wallet operations NEVER touch the network for chain
/// data themselves: reads serve the bdk cache (~0ms — the background
/// [`SyncWorker`] keeps it fresh at its cadence plus pokes), and writes only
/// gate on the worker's first-sync latch so they can't build a spend from a
/// never-synced cache at boot. `chain` is the coin's pooled long-lived
/// Electrum connection, shared with the worker and every other engine call.
pub struct BdkWalletBackend {
    params: &'static ChainParams,
    chain: Arc<ElectrumBackend>,
    wallet: Option<(WalletHandle, Arc<SyncWorker>)>,
}

impl BdkWalletBackend {
    pub fn new(
        params: &'static ChainParams,
        chain: Arc<ElectrumBackend>,
        wallet: Option<(WalletHandle, Arc<SyncWorker>)>,
    ) -> Self {
        Self {
            params,
            chain,
            wallet,
        }
    }

    /// Nudge the sync worker: our own action changed (or is about to
    /// change) what the chain knows about us — pick it up now, not at the
    /// next tick.
    fn poke_worker(&self) {
        if let Some((_, worker)) = &self.wallet {
            worker.poke();
        }
    }

    /// Run a wallet operation on the CACHED wallet under the entry lock and
    /// persist any staged change afterwards — also on operation error, so
    /// anything already staged survives.
    fn with_wallet<T>(&self, f: impl FnOnce(&mut WalletEntry) -> Result<T>) -> Result<T> {
        let (handle, _) = self.wallet.as_ref().context(
            "wallet unavailable: the seed is locked — unlock before spending (nodeless wallet)",
        )?;
        let mut guard = handle.lock().expect("wallet entry poisoned");
        let entry = &mut *guard;
        let out = f(entry);
        let persisted = entry.wallet.persist(&mut entry.conn);
        match (out, persisted) {
            (Ok(v), Ok(_)) => Ok(v),
            (Err(e), _) => Err(e),
            (Ok(_), Err(e)) => Err(anyhow!("persisting wallet: {e}")),
        }
    }

    /// [`Self::with_wallet`] for operations that BUILD/SPEND: wait (bounded)
    /// for the worker's first completed sync of this run, so a spend can
    /// never coin-select from a cache that has not seen the chain at all
    /// (fresh restore, boot race). Steady state costs nothing — the latch
    /// is already set. A dead server surfaces as an honest error here
    /// instead of a silent double-spend risk.
    fn with_synced_wallet<T>(&self, f: impl FnOnce(&mut WalletEntry) -> Result<T>) -> Result<T> {
        let (_, worker) = self.wallet.as_ref().context(
            "wallet unavailable: the seed is locked — unlock before spending (nodeless wallet)",
        )?;
        worker.poke(); // an in-backoff worker should retry NOW
        anyhow::ensure!(
            worker.wait_first_sync(FIRST_SYNC_WAIT),
            "the nodeless wallet has not completed its first chain sync yet — check that \
             the coin's Electrum server is reachable, then retry"
        );
        self.with_wallet(f)
    }

    /// Build + sign a spend of `amount_sat` to `spk` priced by `fee` (market
    /// estimate at a target, or an explicit user rate), BIP125-replaceable
    /// like the Core path's `sendtoaddress`.
    fn build_signed(
        &self,
        entry: &mut WalletEntry,
        spk: ScriptBuf,
        amount_sat: u64,
        fee: SendFee,
        sequence: Sequence,
    ) -> Result<Transaction> {
        // resolve_send_fee is sat/kvB; bdk's FeeRate is sat/kwu = sat/kvB ÷ 4
        // (1 vB = 4 wu), so the estimator's fraction carries exactly.
        let feerate = FeeRate::from_sat_per_kwu((self.resolve_send_fee(fee)? + 2) / 4);
        let mut builder = entry.wallet.build_tx();
        builder
            .add_recipient(spk, Amount::from_sat(amount_sat))
            .fee_rate(feerate)
            .set_exact_sequence(sequence);
        let mut psbt = builder.finish().map_err(|e| anyhow!("building tx: {e}"))?;
        self.finalize(entry, &mut psbt)?;
        psbt.extract_tx().map_err(|e| anyhow!("extracting tx: {e}"))
    }

    fn finalize(&self, entry: &WalletEntry, psbt: &mut Psbt) -> Result<()> {
        let done = entry
            .wallet
            .sign(psbt, SignOptions::default())
            .map_err(|e| anyhow!("signing: {e}"))?;
        anyhow::ensure!(done, "wallet could not finalize the transaction");
        Ok(())
    }
}

impl ChainBackend for BdkWalletBackend {
    fn params(&self) -> &ChainParams {
        self.params
    }

    fn verify_chain(&self) -> Result<()> {
        self.chain.verify_chain()
    }

    fn broadcast(&self, tx: &Transaction) -> Result<Txid> {
        let txid = self.chain.broadcast(tx)?;
        // Swap txs (fundings, redeems, refunds) routinely touch our own
        // spks — let the worker fold them in now.
        self.poke_worker();
        Ok(txid)
    }

    fn get_txout(
        &self,
        outpoint: &OutPoint,
        expected_spk: &ScriptBuf,
    ) -> Result<Option<TxOutInfo>> {
        self.chain.get_txout(outpoint, expected_spk)
    }

    fn find_funding(&self, spk: &ScriptBuf) -> Result<Option<(OutPoint, TxOutInfo)>> {
        self.chain.find_funding(spk)
    }

    fn find_vout(&self, txid: &str, script_pubkey_hex: &str) -> Result<u32> {
        self.chain.find_vout(txid, script_pubkey_hex)
    }

    fn find_spend_witness(
        &self,
        outpoint: &OutPoint,
        watch_spk: &ScriptBuf,
        from_height: u64,
    ) -> Result<Option<Vec<Vec<u8>>>> {
        self.chain
            .find_spend_witness(outpoint, watch_spk, from_height)
    }

    fn tip_height(&self) -> Result<u64> {
        self.chain.tip_height()
    }

    fn tip_median_time(&self) -> Result<u64> {
        self.chain.tip_median_time()
    }

    fn tx_confirmations(&self, txid: &str, spk_hint: Option<&ScriptBuf>) -> Result<u64> {
        self.chain.tx_confirmations(txid, spk_hint)
    }

    fn fee_rate_for(&self, conf_target: u16, conservative: bool) -> Result<u64> {
        self.chain.fee_rate_for(conf_target, conservative)
    }

    fn fee_estimate(&self, conf_target: u16) -> Result<Option<u64>> {
        self.chain.fee_estimate(conf_target)
    }

    // -- the nine wallet operations (design doc §3) --

    fn wallet_new_address(&self) -> Result<String> {
        let address = self.with_wallet(|entry| {
            let spk = wallet_handout_spk(entry);
            spk_to_address(self.params, &spk)
        })?;
        // The Receive dialog is open: subscribe the (possibly fresh) spk and
        // refresh now, so the incoming payment is spotted promptly.
        self.poke_worker();
        Ok(address)
    }

    fn wallet_balance(&self) -> Result<u64> {
        self.with_wallet(|entry| Ok(entry.wallet.balance().trusted_spendable().to_sat()))
    }

    fn wallet_send(&self, address: &str, amount_sat: u64, fee: SendFee) -> Result<String> {
        let spk = self.params.parse_address(address)?;
        let txid = self.with_synced_wallet(|entry| {
            // User sends (and v1 HTLC fundings, which ride this path) signal
            // BIP125: the owner bumps sends, the nurse RBFs v1 fundings.
            let tx = self.build_signed(
                entry,
                spk,
                amount_sat,
                fee,
                Sequence::ENABLE_RBF_NO_LOCKTIME,
            )?;
            // Broadcast-before-persist (the rc6 commit rule): a crash after
            // broadcast re-learns the tx from our own spk history on the
            // next sync, never double-spends.
            let txid = self.chain.broadcast(&tx)?;
            entry.wallet.apply_unconfirmed_txs([(tx, now_ts())]);
            Ok(txid.to_string())
        })?;
        self.poke_worker();
        Ok(txid)
    }

    fn wallet_send_all(&self, address: &str, fee: SendFee) -> Result<String> {
        let spk = self.params.parse_address(address)?;
        let txid = self.with_synced_wallet(|entry| {
            // sat/kvB → sat/kwu, same as build_signed.
            let feerate = FeeRate::from_sat_per_kwu((self.resolve_send_fee(fee)? + 2) / 4);
            // drain_wallet + drain_to: every spendable UTXO in, one output
            // out, the fee off the swept amount (bdk's sweep). Inputs held by
            // built-but-unbroadcast v2 fundings are already out of the
            // canonical UTXO set (apply_unconfirmed_txs locked them), so the
            // drain cannot claw back a reservation.
            let mut builder = entry.wallet.build_tx();
            builder
                .drain_wallet()
                .drain_to(spk)
                .fee_rate(feerate)
                .set_exact_sequence(Sequence::ENABLE_RBF_NO_LOCKTIME);
            let mut psbt = builder
                .finish()
                .map_err(|e| anyhow!("building sweep: {e}"))?;
            self.finalize(entry, &mut psbt)?;
            let tx = psbt
                .extract_tx()
                .map_err(|e| anyhow!("extracting sweep: {e}"))?;
            // Broadcast-before-persist (the rc6 commit rule), same as
            // wallet_send.
            let txid = self.chain.broadcast(&tx)?;
            entry.wallet.apply_unconfirmed_txs([(tx, now_ts())]);
            Ok(txid.to_string())
        })?;
        self.poke_worker();
        Ok(txid)
    }

    fn wallet_build_funding(
        &self,
        address: &str,
        amount_sat: u64,
    ) -> Result<(String, u32, String)> {
        let spk = self.params.parse_address(address)?;
        let built = self.with_synced_wallet(|entry| {
            // Funding prices at the per-coin ~30-min target (see
            // funding_conf_target), mirroring the Core-RPC backend. It is
            // broadcast NON-replaceable (no BIP125 signal): the v2 funding
            // txid is committed into the pre-signed MuSig2 redeems, so it
            // must never be RBF'd — the nurse CPFPs it instead, and the
            // non-signal keeps external wallets from even offering a bump.
            let tx = self.build_signed(
                entry,
                spk.clone(),
                amount_sat,
                SendFee::Target(self.funding_conf_target()),
                Sequence::ENABLE_LOCKTIME_NO_RBF,
            )?;
            let txid = tx.compute_txid();
            let vout = tx
                .output
                .iter()
                .position(|o| o.script_pubkey == spk)
                .context("built funding pays no output to the target script")?
                as u32;
            let hex_tx = bitcoin::consensus::encode::serialize_hex(&tx);
            // Deliberately NOT broadcast (v2 two-phase funding, spec v2 §7).
            // Inserting it unconfirmed locks its inputs against reuse and
            // tracks the change — bdk's equivalent of Core `lockUnspents`.
            // A swap that dies pre-broadcast releases them via
            // wallet_cancel_funding on the engine's terminal paths.
            entry.wallet.apply_unconfirmed_txs([(tx, now_ts())]);
            Ok((txid.to_string(), vout, hex_tx))
        })?;
        // A change spk may have just been revealed — get it subscribed.
        self.poke_worker();
        Ok(built)
    }

    fn wallet_cancel_funding(&self, tx_hex: &str) -> Result<()> {
        let tx: Transaction = bitcoin::consensus::encode::deserialize(&hex::decode(tx_hex)?)
            .context("decode built funding tx for cancel")?;
        let txid = tx.compute_txid();
        // No sync: cancel runs on abort paths and must work while the Electrum
        // view is unreachable; evicting a never-broadcast tx needs no chain
        // data. (If the tx WAS broadcast after all, the next sync re-learns it
        // from our own spk histories — its inputs spend our spks — and a
        // fresher last_seen out-cancels the eviction, so this self-heals.)
        self.with_wallet(|entry| {
            // Belt-and-suspenders (the engine gates on this too): never evict
            // a funding the wallet knows is confirmed.
            if let Some(wtx) = entry.wallet.get_tx(txid) {
                anyhow::ensure!(
                    !matches!(wtx.chain_position, ChainPosition::Confirmed { .. }),
                    "refusing to cancel funding {txid}: it is confirmed on-chain"
                );
            }
            // Drop the phantom from the canonical set — this is what frees its
            // inputs — and unmark the change derivation index for reuse.
            entry.wallet.apply_evicted_txs([(txid, now_ts())]);
            entry.wallet.cancel_tx(&tx);
            Ok(())
        })
    }

    fn wallet_transactions(&self) -> Result<Vec<WalletTxInfo>> {
        self.with_wallet(|entry| Ok(wallet_activity(entry)))
    }

    fn wallet_locked(&self) -> Result<bool> {
        Ok(self.wallet.is_none())
    }

    fn wallet_sign_send(
        &self,
        tx: &Transaction,
        prevout_value_sat: u64,
        prevout_spk: &ScriptBuf,
    ) -> Result<Txid> {
        anyhow::ensure!(
            tx.input.len() == 1,
            "CPFP child must spend exactly the one wallet-owned prevout"
        );
        let outpoint = tx.input[0].previous_output;
        let txid = self.with_synced_wallet(|entry| {
            // The parent (our sweep/change output) is normally already in the
            // graph via sync; a floating txout covers the race where it
            // hasn't propagated to the Electrum server yet.
            if entry.wallet.get_utxo(outpoint).is_none() {
                entry.wallet.insert_txout(
                    outpoint,
                    TxOut {
                        value: Amount::from_sat(prevout_value_sat),
                        script_pubkey: prevout_spk.clone(),
                    },
                );
            }
            let utxo = entry
                .wallet
                .get_utxo(outpoint)
                .context("CPFP prevout is not owned by the nodeless wallet")?;
            let input = entry
                .wallet
                .get_psbt_input(utxo, None, true)
                .map_err(|e| anyhow!("psbt input for CPFP prevout: {e}"))?;
            let mut psbt =
                Psbt::from_unsigned_tx(tx.clone()).context("CPFP child is not unsigned")?;
            psbt.inputs[0] = input;
            let done = entry
                .wallet
                .sign(
                    &mut psbt,
                    SignOptions {
                        trust_witness_utxo: true,
                        ..Default::default()
                    },
                )
                .map_err(|e| anyhow!("signing CPFP child: {e}"))?;
            anyhow::ensure!(done, "wallet could not finalize the CPFP child");
            let tx = psbt
                .extract_tx()
                .map_err(|e| anyhow!("extracting CPFP child: {e}"))?;
            let txid = self.chain.broadcast(&tx)?;
            entry.wallet.apply_unconfirmed_txs([(tx, now_ts())]);
            Ok(txid)
        })?;
        self.poke_worker();
        Ok(txid)
    }

    fn wallet_tx_fee_vsize(&self, txid: &str) -> Result<(u64, u64)> {
        let txid = Txid::from_str(txid)?;
        self.with_wallet(|entry| {
            let tx = entry
                .wallet
                .get_tx(txid)
                .with_context(|| format!("tx {txid} not known to the nodeless wallet"))?
                .tx_node
                .tx
                .clone();
            let fee = entry
                .wallet
                .calculate_fee(&tx)
                .map_err(|e| anyhow!("fee of {txid}: {e}"))?;
            Ok((fee.to_sat(), tx.vsize() as u64))
        })
    }

    fn wallet_change_output(
        &self,
        funding_txid: &str,
        htlc_spk: &ScriptBuf,
    ) -> Result<Option<(u32, u64, ScriptBuf)>> {
        let txid = Txid::from_str(funding_txid)?;
        self.with_wallet(|entry| {
            let tx = entry
                .wallet
                .get_tx(txid)
                .with_context(|| format!("funding {txid} not known to the nodeless wallet"))?
                .tx_node
                .tx
                .clone();
            for (vout, out) in tx.output.iter().enumerate() {
                if out.script_pubkey == *htlc_spk {
                    continue; // the HTLC output is never ours
                }
                if entry.wallet.is_mine(out.script_pubkey.clone()) {
                    return Ok(Some((
                        vout as u32,
                        out.value.to_sat(),
                        out.script_pubkey.clone(),
                    )));
                }
            }
            Ok(None)
        })
    }

    fn wallet_bumpfee(&self, txid: &str, feerate_sat_vb: u64) -> Result<String> {
        let txid = Txid::from_str(txid)?;
        let feerate = FeeRate::from_sat_per_vb(feerate_sat_vb).context("feerate overflow")?;
        let txid = self.with_synced_wallet(|entry| {
            let mut builder = entry
                .wallet
                .build_fee_bump(txid)
                .map_err(|e| anyhow!("bumpfee {txid}: {e}"))?;
            builder.fee_rate(feerate);
            let mut psbt = builder
                .finish()
                .map_err(|e| anyhow!("building bump: {e}"))?;
            self.finalize(entry, &mut psbt)?;
            let tx = psbt
                .extract_tx()
                .map_err(|e| anyhow!("extracting bump: {e}"))?;
            let new_txid = self.chain.broadcast(&tx)?;
            entry.wallet.apply_unconfirmed_txs([(tx, now_ts())]);
            Ok(new_txid.to_string())
        })?;
        self.poke_worker();
        Ok(txid)
    }
}

/// Hand out an external address spk under the [`MAX_UNUSED_AHEAD`] cap:
/// reveal a fresh one while fewer than the cap are outstanding unused, else
/// recycle the oldest unused one (`next_unused_address` picks the lowest
/// unused index; it reveals only when nothing is unused). Free function so
/// the cap invariant is unit-testable without an Electrum server.
fn wallet_handout_spk(entry: &mut WalletEntry) -> ScriptBuf {
    let unused = entry
        .wallet
        .list_unused_addresses(KeychainKind::External)
        .count();
    let info = if unused >= MAX_UNUSED_AHEAD {
        entry.wallet.next_unused_address(KeychainKind::External)
    } else {
        entry.wallet.reveal_next_address(KeychainKind::External)
    };
    info.address.script_pubkey()
}

/// The activity feed off one wallet's canonical tx set (`listtransactions`,
/// design doc §4), newest first. Free function (no chain I/O — the caller
/// syncs) so it is unit-testable without an Electrum server.
fn wallet_activity(entry: &WalletEntry) -> Vec<WalletTxInfo> {
    let tip = entry.wallet.latest_checkpoint().height();
    let mut out = Vec::new();
    for wtx in entry.wallet.transactions() {
        let tx = &wtx.tx_node.tx;
        let (sent, received) = entry.wallet.sent_and_received(tx);
        let (sent, received) = (sent.to_sat(), received.to_sat());
        let fee_sat = entry.wallet.calculate_fee(tx).ok().map(Amount::to_sat);
        let (direction, amount_sat) = if sent > received {
            // Net send: inputs minus change minus the fee = what the
            // recipient got. Unknown fee (foreign input) degrades to the
            // net outflow, fee included.
            let net_out = sent - received;
            ("sent", net_out - fee_sat.unwrap_or(0).min(net_out))
        } else {
            ("received", received - sent)
        };
        let (confirmations, timestamp) = match wtx.chain_position {
            ChainPosition::Confirmed { anchor, .. } => (
                u64::from((tip + 1).saturating_sub(anchor.block_id.height)),
                Some(anchor.confirmation_time),
            ),
            ChainPosition::Unconfirmed {
                first_seen,
                last_seen,
            } => (0, first_seen.or(last_seen)),
        };
        out.push(WalletTxInfo {
            txid: wtx.tx_node.txid.to_string(),
            direction: direction.into(),
            amount_sat,
            fee_sat,
            vsize: tx.vsize() as u64,
            confirmations,
            timestamp,
        });
    }
    // Newest first: mempool/pending (0 confs) ahead of shallow ahead of
    // deep; ties by descending time. A built-but-unbroadcast v2 funding has
    // no timestamp and sorts to the very front.
    out.sort_by_key(|t| {
        (
            t.confirmations,
            std::cmp::Reverse(t.timestamp.unwrap_or(u64::MAX)),
        )
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::Network;
    use std::sync::atomic::{AtomicU32, Ordering};

    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    fn temp_data_dir() -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "pact-wallet-bdk-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn btc_mainnet() -> &'static ChainParams {
        registry::get("btc")
            .expect("built-in btc")
            .params(Network::Mainnet)
            .expect("btc mainnet params")
    }

    #[test]
    fn first_address_matches_bip86_vector_and_persists() {
        let dir = temp_data_dir();
        let seed = PactSeed::from_mnemonic(TEST_MNEMONIC, "").unwrap();
        let params = btc_mainnet();

        let manager = WalletManager::new(&dir);
        let handle = manager.open("btc", params, &seed).unwrap();
        {
            let mut guard = handle.lock().unwrap();
            let entry = &mut *guard;
            let spk = entry
                .wallet
                .reveal_next_address(KeychainKind::External)
                .address
                .script_pubkey();
            // BIP-86 first receiving address (m/86'/0'/0'/0/0) of the
            // standard test mnemonic — COIN_BTC = 0, so this is the
            // published vector, encoded through OUR params (hrp "bc").
            assert_eq!(
                spk_to_address(params, &spk).unwrap(),
                "bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr"
            );
            let persisted = entry.wallet.persist(&mut entry.conn).unwrap();
            assert!(persisted);
        }

        // A fresh manager on the same data dir resumes at index 1: the
        // revealed index survived, and the descriptors/genesis checks pass.
        let manager2 = WalletManager::new(&dir);
        let handle2 = manager2.open("btc", params, &seed).unwrap();
        {
            let mut entry = handle2.lock().unwrap();
            assert_eq!(
                entry.wallet.derivation_index(KeychainKind::External),
                Some(0)
            );
            let next = entry
                .wallet
                .reveal_next_address(KeychainKind::External)
                .address
                .script_pubkey();
            assert_eq!(
                spk_to_address(params, &next).unwrap(),
                // BIP-86 second receiving address (m/86'/0'/0'/0/1).
                "bc1p4qhjn9zdvkux4e44uhx8tc55attvtyu358kutcqkudyccelu0was9fqzwh"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn built_funding_reserves_inputs_and_cancel_releases_them() {
        use bitcoin::hashes::Hash;

        let dir = temp_data_dir();
        let params = btc_mainnet();
        let seed = PactSeed::from_mnemonic(TEST_MNEMONIC, "").unwrap();
        let manager = WalletManager::new(&dir);
        let handle = manager.open("btc", params, &seed).unwrap();
        let mut guard = handle.lock().unwrap();
        let entry = &mut *guard;

        // Fund the wallet: a confirmed (non-coinbase) foreign tx paying our
        // first address, anchored in a fabricated block 1.
        let spk0 = entry
            .wallet
            .reveal_next_address(KeychainKind::External)
            .address
            .script_pubkey();
        let funding = Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: bitcoin::absolute::LockTime::ZERO,
            input: vec![bitcoin::TxIn {
                previous_output: OutPoint {
                    txid: Txid::from_byte_array([2u8; 32]),
                    vout: 0,
                },
                ..Default::default()
            }],
            output: vec![TxOut {
                value: Amount::from_sat(100_000),
                script_pubkey: spk0,
            }],
        };
        let fund_txid = funding.compute_txid();
        let h1 = BlockHash::from_byte_array([1u8; 32]);
        let genesis = entry.wallet.latest_checkpoint().block_id();
        let cp = CheckPoint::from_block_ids([
            genesis,
            BlockId {
                height: 1,
                hash: h1,
            },
        ])
        .unwrap();
        let mut tx_update = TxUpdate::<ConfirmationBlockTime>::default();
        tx_update.txs.push(Arc::new(funding));
        tx_update.anchors.insert((
            ConfirmationBlockTime {
                block_id: BlockId {
                    height: 1,
                    hash: h1,
                },
                confirmation_time: 1_000,
            },
            fund_txid,
        ));
        entry
            .wallet
            .apply_update(Update {
                last_active_indices: BTreeMap::new(),
                tx_update,
                chain: Some(cp),
            })
            .unwrap();
        assert_eq!(entry.wallet.balance().trusted_spendable().to_sat(), 100_000);

        // Build-and-hold a "swap leg" funding (v2 two-phase, spec §7): sign,
        // insert unbroadcast — its inputs must now be reserved.
        let leg_spk = ScriptBuf::from_hex(&format!("5120{}", "ab".repeat(32))).unwrap();
        let mut builder = entry.wallet.build_tx();
        builder
            .add_recipient(leg_spk, Amount::from_sat(40_000))
            .fee_rate(FeeRate::from_sat_per_vb(2).unwrap())
            .set_exact_sequence(Sequence::ENABLE_RBF_NO_LOCKTIME);
        let mut psbt = builder.finish().unwrap();
        assert!(entry
            .wallet
            .sign(&mut psbt, SignOptions::default())
            .unwrap());
        let built = psbt.extract_tx().unwrap();
        let built_txid = built.compute_txid();
        let fee = entry.wallet.calculate_fee(&built).unwrap().to_sat();
        entry.wallet.apply_unconfirmed_txs([(built.clone(), 2_000)]);
        assert_eq!(
            entry.wallet.balance().trusted_spendable().to_sat(),
            100_000 - 40_000 - fee,
            "built-but-unbroadcast funding must reserve its inputs"
        );

        // Activity feed: the pending send sorts first, then the receive.
        let act = wallet_activity(entry);
        assert_eq!(act.len(), 2);
        assert_eq!(
            (
                act[0].direction.as_str(),
                act[0].amount_sat,
                act[0].confirmations
            ),
            ("sent", 40_000, 0)
        );
        assert_eq!(act[0].fee_sat, Some(fee));
        assert_eq!(
            (
                act[1].direction.as_str(),
                act[1].amount_sat,
                act[1].confirmations
            ),
            ("received", 100_000, 1)
        );

        // Cancel — the exact wallet_cancel_funding sequence (evict from the
        // canonical set + unmark the change index): the inputs are spendable
        // again and the phantom leaves the activity feed.
        entry.wallet.apply_evicted_txs([(built_txid, 3_000)]);
        entry.wallet.cancel_tx(&built);
        assert_eq!(
            entry.wallet.balance().trusted_spendable().to_sat(),
            100_000,
            "cancel must release the reserved inputs"
        );
        let act = wallet_activity(entry);
        assert_eq!(act.len(), 1);
        assert_eq!(act[0].txid, fund_txid.to_string());

        drop(guard);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn handout_cap_bounds_the_gap_and_recycles_oldest_unused() {
        use bitcoin::hashes::Hash;

        let dir = temp_data_dir();
        let params = btc_mainnet();
        let seed = PactSeed::from_mnemonic(TEST_MNEMONIC, "").unwrap();
        let handle = WalletManager::new(&dir).open("btc", params, &seed).unwrap();
        let mut guard = handle.lock().unwrap();
        let entry = &mut *guard;

        // 30 handouts, nothing ever paid: exactly MAX_UNUSED_AHEAD distinct
        // addresses are revealed, then the OLDEST unused one (index 0)
        // recycles — the on-chain gap is bounded by construction.
        let mut spks = Vec::new();
        for _ in 0..30 {
            spks.push(wallet_handout_spk(entry));
        }
        let distinct: std::collections::HashSet<_> = spks.iter().cloned().collect();
        assert_eq!(distinct.len(), MAX_UNUSED_AHEAD);
        assert_eq!(
            entry.wallet.derivation_index(KeychainKind::External),
            Some((MAX_UNUSED_AHEAD - 1) as u32),
            "no reveal past the cap"
        );
        assert_eq!(
            spks[MAX_UNUSED_AHEAD], spks[0],
            "recycles the oldest unused"
        );
        assert!(
            STOP_GAP as usize > MAX_UNUSED_AHEAD,
            "scan must out-reach the cap"
        );

        // Pay the oldest one: it leaves the unused set, so the next handout
        // reveals a FRESH address again (index 20).
        let paid = Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: bitcoin::absolute::LockTime::ZERO,
            input: vec![bitcoin::TxIn {
                previous_output: OutPoint {
                    txid: Txid::from_byte_array([9u8; 32]),
                    vout: 0,
                },
                ..Default::default()
            }],
            output: vec![TxOut {
                value: Amount::from_sat(1_000),
                script_pubkey: spks[0].clone(),
            }],
        };
        entry.wallet.apply_unconfirmed_txs([(paid, 1_000)]);
        let next = wallet_handout_spk(entry);
        assert!(
            !distinct.contains(&next),
            "a payment frees the cap for a fresh reveal"
        );
        assert_eq!(
            entry.wallet.derivation_index(KeychainKind::External),
            Some(MAX_UNUSED_AHEAD as u32)
        );

        drop(guard);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn revealed_spks_track_reveals_across_keychains() {
        let dir = temp_data_dir();
        let params = btc_mainnet();
        let seed = PactSeed::from_mnemonic(TEST_MNEMONIC, "").unwrap();
        let handle = WalletManager::new(&dir).open("btc", params, &seed).unwrap();
        let mut guard = handle.lock().unwrap();
        let entry = &mut *guard;

        // Fresh store: nothing revealed — the sync side treats this as the
        // full-scan case and the worker has nothing to subscribe yet.
        assert!(revealed_spks(entry).is_empty());

        entry.wallet.reveal_next_address(KeychainKind::External);
        entry.wallet.reveal_next_address(KeychainKind::External);
        entry.wallet.reveal_next_address(KeychainKind::Internal);
        let spks = revealed_spks(entry);
        assert_eq!(spks.len(), 3, "2 external + 1 internal");
        assert_eq!(
            spks.iter().collect::<std::collections::HashSet<_>>().len(),
            3,
            "all distinct"
        );

        drop(guard);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ensure_worker_reuses_per_coin_and_shuts_down_with_manager() {
        let dir = temp_data_dir();
        let params = btc_mainnet();
        let seed = PactSeed::from_mnemonic(TEST_MNEMONIC, "").unwrap();
        let manager = WalletManager::new(&dir);
        let handle = manager.open("btc", params, &seed).unwrap();
        // Lazy backend pointed at a dead port: the worker just backs off.
        let chain = Arc::new(ElectrumBackend::new(params, "tcp://127.0.0.1:1").unwrap());

        let w1 = manager.ensure_worker("btc", chain.clone(), &handle);
        let w2 = manager.ensure_worker("btc", chain, &handle);
        assert!(Arc::ptr_eq(&w1, &w2), "one worker per coin");

        // No sync ever completed against the dead server: the write gate
        // reports honestly instead of hanging.
        assert!(!w1.wait_first_sync(std::time::Duration::from_millis(10)));

        // Manager drop = merchant unload: the latch releases as NOT synced,
        // so a racing write errors instead of blocking its full timeout.
        drop(manager);
        assert!(!w1.wait_first_sync(std::time::Duration::from_secs(30)));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn wrong_seed_fails_descriptor_check() {
        let dir = temp_data_dir();
        let params = btc_mainnet();
        let seed = PactSeed::from_mnemonic(TEST_MNEMONIC, "").unwrap();
        WalletManager::new(&dir).open("btc", params, &seed).unwrap();

        // Same store, different seed (passphrase changes everything) — the
        // load-time descriptor check must refuse, not silently mix keys.
        let other = PactSeed::from_mnemonic(TEST_MNEMONIC, "different").unwrap();
        let err = WalletManager::new(&dir).open("btc", params, &other);
        assert!(err.is_err(), "descriptor mismatch must fail the load");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
