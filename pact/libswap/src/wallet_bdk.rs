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

use crate::chain::{ChainBackend, ElectrumBackend, TxOutInfo, WalletTxInfo};
use crate::keys::PactSeed;
use crate::params::ChainParams;
use crate::registry;

/// BIP-44 gap limit for the initial full scan of a restored seed. Every
/// address this wallet hands out is revealed-then-persisted, so steady-state
/// syncs never probe beyond the revealed set; the gap only matters when the
/// sqlite store is fresh for a seed that may have on-chain history
/// (restore on a new machine). Deep-rescan affordance: design doc O2.
const STOP_GAP: u32 = 20;

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
}

impl WalletManager {
    /// `data_dir` is the merchant data dir; wallets live under
    /// `<data_dir>/wallet/<coin_id>.sqlite`.
    pub fn new(data_dir: &Path) -> Self {
        Self {
            wallet_dir: data_dir.join("wallet"),
            wallets: Mutex::new(BTreeMap::new()),
        }
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

// ---- chain source: raw Electrum → bdk updates ------------------------------

/// Sync `entry`'s wallet from the Electrum backend: scripthash histories of
/// the revealed spks (or a STOP_GAP full scan when the store is fresh),
/// PoCX-safe anchors from raw headers, and a checkpoint update that always
/// connects to the wallet's local chain (genesis at worst). This is the
/// unforked-bdk chain source of design doc D3.
fn sync_entry(entry: &mut WalletEntry, chain: &ElectrumBackend) -> Result<()> {
    let params = chain.params();
    // Fresh store (nothing ever revealed) → gap-limit scan for a restored
    // seed's history. Steady state → revealed spks only.
    let full_scan = entry
        .wallet
        .derivation_index(KeychainKind::External)
        .is_none();

    let mut tx_update = TxUpdate::<ConfirmationBlockTime>::default();
    let mut fetched: HashSet<Txid> = HashSet::new();
    let mut headers: BTreeMap<u32, (BlockHash, u64)> = BTreeMap::new();
    // One timestamp for the whole sync pass: `seen_ats` is a SET of
    // (txid, ts) pairs (bdk_chain 0.23), so a tx surfacing in several spk
    // histories must insert the identical pair to dedupe.
    let sync_ts = now_ts();
    let mut seen_ats: HashSet<(Txid, u64)> = HashSet::new();
    let mut last_active: BTreeMap<KeychainKind, u32> = BTreeMap::new();

    let mut process_spk = |entry: &WalletEntry,
                           tx_update: &mut TxUpdate<ConfirmationBlockTime>,
                           spk: &ScriptBuf|
     -> Result<bool> {
        let _ = entry; // spks are peeked from the wallet by the callers
        let history = chain.history(spk)?;
        for (txid_hex, height) in &history {
            let txid = Txid::from_str(txid_hex).context("electrum history txid")?;
            if fetched.insert(txid) {
                tx_update.txs.push(Arc::new(chain.get_raw_tx(txid_hex)?));
            }
            if *height > 0 {
                let height = u32::try_from(*height).context("history height")?;
                let (hash, time) = match headers.entry(height) {
                    std::collections::btree_map::Entry::Occupied(e) => *e.get(),
                    std::collections::btree_map::Entry::Vacant(v) => {
                        let (hash_hex, time) = chain.header_at(u64::from(height))?;
                        *v.insert((BlockHash::from_str(&hash_hex)?, u64::from(time)))
                    }
                };
                tx_update.anchors.insert((
                    ConfirmationBlockTime {
                        block_id: BlockId { height, hash },
                        confirmation_time: time,
                    },
                    txid,
                ));
            } else {
                // 0 = mempool, -1 = mempool with unconfirmed parents.
                seen_ats.insert((txid, sync_ts));
            }
        }
        Ok(!history.is_empty())
    };

    for keychain in [KeychainKind::External, KeychainKind::Internal] {
        if full_scan {
            let (mut index, mut gap) = (0u32, 0u32);
            while gap < STOP_GAP {
                let spk = entry
                    .wallet
                    .peek_address(keychain, index)
                    .address
                    .script_pubkey();
                if process_spk(entry, &mut tx_update, &spk)? {
                    last_active.insert(keychain, index);
                    gap = 0;
                } else {
                    gap += 1;
                }
                index += 1;
            }
        } else if let Some(last) = entry.wallet.derivation_index(keychain) {
            for index in 0..=last {
                let spk = entry
                    .wallet
                    .peek_address(keychain, index)
                    .address
                    .script_pubkey();
                process_spk(entry, &mut tx_update, &spk)?;
            }
        }
    }
    tx_update.seen_ats = seen_ats;

    let chain_cp = chain_update(chain, params, entry.wallet.latest_checkpoint(), &headers)?;
    entry
        .wallet
        .apply_update(Update {
            last_active_indices: last_active,
            tx_update,
            chain: Some(chain_cp),
        })
        .map_err(|e| anyhow!("bdk chain update does not connect: {e}"))?;
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
/// Electrum URLs only (design doc D5). `entry` is `None` while the seed is
/// locked — chain reads keep working, wallet operations report the lock.
pub struct BdkWalletBackend {
    params: &'static ChainParams,
    chain: ElectrumBackend,
    entry: Option<WalletHandle>,
}

impl BdkWalletBackend {
    pub fn new(
        params: &'static ChainParams,
        electrum_url: &str,
        entry: Option<WalletHandle>,
    ) -> Result<Self> {
        Ok(Self {
            params,
            chain: ElectrumBackend::new(params, electrum_url)?,
            entry,
        })
    }

    /// Run a wallet operation under the entry lock, syncing first when
    /// `sync` is set, and persist any staged change afterwards — also on
    /// operation error, so chain data learned during the sync survives.
    fn with_wallet<T>(
        &self,
        sync: bool,
        f: impl FnOnce(&mut WalletEntry) -> Result<T>,
    ) -> Result<T> {
        let handle = self.entry.as_ref().context(
            "wallet unavailable: the seed is locked — unlock before spending (nodeless wallet)",
        )?;
        let mut guard = handle.lock().expect("wallet entry poisoned");
        let entry = &mut *guard;
        let out = (|| -> Result<T> {
            if sync {
                sync_entry(entry, &self.chain)?;
            }
            f(entry)
        })();
        let persisted = entry.wallet.persist(&mut entry.conn);
        match (out, persisted) {
            (Ok(v), Ok(_)) => Ok(v),
            (Err(e), _) => Err(e),
            (Ok(_), Err(e)) => Err(anyhow!("persisting wallet: {e}")),
        }
    }

    /// Build + sign a spend of `amount_sat` to `spk` at the current market
    /// feerate, BIP125-replaceable like the Core path's `sendtoaddress`.
    fn build_signed(
        &self,
        entry: &mut WalletEntry,
        spk: ScriptBuf,
        amount_sat: u64,
        conf_target: u16,
    ) -> Result<Transaction> {
        let feerate = FeeRate::from_sat_per_vb(self.chain.fee_rate_for(conf_target, false)?)
            .context("feerate overflow")?;
        let mut builder = entry.wallet.build_tx();
        builder
            .add_recipient(spk, Amount::from_sat(amount_sat))
            .fee_rate(feerate)
            .set_exact_sequence(Sequence::ENABLE_RBF_NO_LOCKTIME);
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
        self.chain.broadcast(tx)
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

    // -- the nine wallet operations (design doc §3) --

    fn wallet_new_address(&self) -> Result<String> {
        self.with_wallet(false, |entry| {
            let spk = entry
                .wallet
                .reveal_next_address(KeychainKind::External)
                .address
                .script_pubkey();
            spk_to_address(self.params, &spk)
        })
    }

    fn wallet_balance(&self) -> Result<u64> {
        self.with_wallet(true, |entry| {
            Ok(entry.wallet.balance().trusted_spendable().to_sat())
        })
    }

    fn wallet_send(&self, address: &str, amount_sat: u64, conf_target: u16) -> Result<String> {
        let spk = self.params.parse_address(address)?;
        self.with_wallet(true, |entry| {
            let tx = self.build_signed(entry, spk, amount_sat, conf_target)?;
            // Broadcast-before-persist (the rc6 commit rule): a crash after
            // broadcast re-learns the tx from our own spk history on the
            // next sync, never double-spends.
            let txid = self.chain.broadcast(&tx)?;
            entry.wallet.apply_unconfirmed_txs([(tx, now_ts())]);
            Ok(txid.to_string())
        })
    }

    fn wallet_build_funding(
        &self,
        address: &str,
        amount_sat: u64,
    ) -> Result<(String, u32, String)> {
        let spk = self.params.parse_address(address)?;
        self.with_wallet(true, |entry| {
            // Funding prices at the per-coin ~30-min target (see
            // funding_conf_target), mirroring the Core-RPC backend.
            let tx =
                self.build_signed(entry, spk.clone(), amount_sat, self.funding_conf_target())?;
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
        })
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
        self.with_wallet(false, |entry| {
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
        self.with_wallet(true, |entry| Ok(wallet_activity(entry)))
    }

    fn wallet_locked(&self) -> Result<bool> {
        Ok(self.entry.is_none())
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
        self.with_wallet(true, |entry| {
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
        })
    }

    fn wallet_tx_fee_vsize(&self, txid: &str) -> Result<(u64, u64)> {
        let txid = Txid::from_str(txid)?;
        self.with_wallet(false, |entry| {
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
        self.with_wallet(false, |entry| {
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
        self.with_wallet(true, |entry| {
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
        })
    }
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
