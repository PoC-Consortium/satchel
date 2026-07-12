//! Nodeless on-chain wallet — bdk over the raw Electrum machinery
//! (docs/NODELESS_WALLET.md, epic #58).
//!
//! The wallet itself was extracted to the `wallet-btcx` crate:
//! [`WalletManager`] (per-coin bdk wallet cache + background sync workers)
//! and [`wallet_btcx::BdkWalletBackend`] (chain data over Electrum, wallet
//! ops over bdk, with the swap-funding primitives under its `swap-support`
//! feature). What stays here is the engine-facing adapter: a thin
//! [`BdkWalletBackend`] wrapper implementing the [`ChainBackend`] trait —
//! chain reads delegate to the wallet HOME server's pooled Electrum
//! connection, wallet operations to the extracted backend.
//!
//! Syncing is a BACKGROUND job (issue #87): the per-coin
//! [`SyncWorker`](crate::wallet_bdk::SyncWorker) keeps the bdk cache fresh
//! over the coin's one long-lived Electrum connection, so the `wallet_*`
//! operations never perform chain I/O for the wallet's own state — reads
//! serve the cache as-is, writes only gate on the worker's first-sync latch.

use anyhow::{Context, Result};
use bitcoin::{OutPoint, ScriptBuf, Transaction, Txid};
use std::sync::Arc;

use crate::chain::{ChainBackend, ElectrumBackend, SendFee, TxOutInfo, WalletTxInfo};
use crate::params::ChainParams;

// The extracted wallet layer, re-exported under the old module path.
pub use electrum_btcx::sync::{WalletEntry, WalletHandle};
pub use electrum_btcx::worker::SyncWorker;
pub use wallet_btcx::WalletManager;

/// The nodeless primary backend: [`wallet_btcx::BdkWalletBackend`] adapted
/// to the engine's [`ChainBackend`] trait. Chain reads ride the wallet HOME
/// server's pooled connection; the nine `wallet_*` operations — the ones
/// only a Core-RPC wallet URL could serve until the nodeless wallet — are
/// served by bdk.
pub struct BdkWalletBackend {
    backend: wallet_btcx::BdkWalletBackend,
}

impl BdkWalletBackend {
    /// `chain` is the coin's pooled long-lived Electrum connection (the
    /// elected HOME server), `views` the coin's other ACTIVE view servers
    /// (broadcast fallbacks), `wallet` the open wallet handle + its sync
    /// worker — `None` while the seed is locked (chain reads keep working,
    /// wallet operations report the lock).
    pub fn new(
        params: &'static ChainParams,
        chain: Arc<ElectrumBackend>,
        views: Vec<Arc<ElectrumBackend>>,
        wallet: Option<(WalletHandle, Arc<SyncWorker>)>,
    ) -> Self {
        Self {
            backend: wallet_btcx::BdkWalletBackend::new(params, chain, views, wallet),
        }
    }

    /// The wallet HOME server's pooled connection — the wrapper's chain
    /// reads (and its view health) ride this. Returned as the bare backend
    /// so calls resolve to `electrum-btcx`'s inherent methods, not back
    /// into the [`ChainBackend`] impl on `Arc<ElectrumBackend>`.
    fn chain(&self) -> &ElectrumBackend {
        self.backend.chain().as_ref()
    }
}

impl ChainBackend for BdkWalletBackend {
    fn params(&self) -> &ChainParams {
        self.backend.params()
    }

    fn view_health(&self) -> Option<Arc<crate::server_health::ServerHealth>> {
        // Chain reads here ride the wallet HOME server's pooled connection
        // — its health is this backend's view health, so a MultiBackend
        // quorum skips a down home instead of stalling on it. (Wallet OPS
        // are cache reads and unaffected; the send path surfaces the home
        // being down honestly at broadcast.)
        Some(self.chain().health().clone())
    }

    fn verify_chain(&self) -> Result<()> {
        self.chain().verify_chain()
    }

    fn broadcast(&self, tx: &Transaction) -> Result<Txid> {
        // Swap txs (fundings, redeems, refunds) routinely touch our own
        // spks — the backend pokes the worker to fold them in now.
        self.backend.broadcast(tx)
    }

    fn get_txout(
        &self,
        outpoint: &OutPoint,
        expected_spk: &ScriptBuf,
    ) -> Result<Option<TxOutInfo>> {
        self.chain().get_txout(outpoint, expected_spk)
    }

    fn find_funding(&self, spk: &ScriptBuf) -> Result<Option<(OutPoint, TxOutInfo)>> {
        self.chain().find_funding(spk)
    }

    fn find_vout(&self, txid: &str, script_pubkey_hex: &str) -> Result<u32> {
        // Home first; a failing home falls over to any available view (#99)
        // — the fallback lives in the extracted backend.
        self.backend.find_vout(txid, script_pubkey_hex)
    }

    fn find_spend_witness(
        &self,
        outpoint: &OutPoint,
        watch_spk: &ScriptBuf,
        _from_height: u64,
    ) -> Result<Option<Vec<Vec<u8>>>> {
        // Electrum searches by script history — no height hint needed.
        self.chain().find_spend_witness(outpoint, watch_spk)
    }

    fn spk_history(&self, spk: &ScriptBuf) -> Result<Option<Vec<(String, i64)>>> {
        Ok(Some(self.chain().history(spk)?))
    }

    fn fetch_tx(&self, txid: &str) -> Result<Option<Transaction>> {
        // Fetch failures read as "cannot see it" (inconclusive) — see the
        // ElectrumBackend trait impl.
        Ok(self.chain().get_raw_tx(txid).ok())
    }

    fn tip_height(&self) -> Result<u64> {
        self.chain().tip_height()
    }

    fn tip_median_time(&self) -> Result<u64> {
        self.chain().tip_median_time()
    }

    fn tx_confirmations(&self, txid: &str, spk_hint: Option<&ScriptBuf>) -> Result<u64> {
        let spk = spk_hint.context(
            "Electrum backend can only locate transactions by script — spk hint required",
        )?;
        self.chain().tx_confirmations(txid, spk)
    }

    fn fee_rate_for(&self, conf_target: u16, _conservative: bool) -> Result<u64> {
        // Electrum has no economical/conservative mode distinction.
        self.chain().fee_rate_for(conf_target)
    }

    fn fee_rate_for_kvb(&self, conf_target: u16, _conservative: bool) -> Result<u64> {
        // Delegate to the Electrum chain's PRECISE sat/kvB estimate (not the
        // integer-rounded trait default) — the funding nurse prices its RBF off
        // this, and the field-stranded funding was on this bdk/Electrum path.
        self.chain().fee_rate_for_kvb(conf_target)
    }

    fn fee_estimate(&self, conf_target: u16) -> Result<Option<u64>> {
        self.chain().fee_estimate(conf_target)
    }

    fn resolve_send_fee(&self, fee: SendFee) -> Result<u64> {
        // Same math as the trait default — delegate so it lives once, in the
        // crate, next to the estimator it prices off.
        self.chain().resolve_send_fee(fee)
    }

    // -- the nine wallet operations (design doc §3) --

    fn wallet_new_address(&self) -> Result<String> {
        self.backend.wallet_new_address()
    }

    fn wallet_balance(&self) -> Result<u64> {
        self.backend.wallet_balance()
    }

    fn wallet_send(&self, address: &str, amount_sat: u64, fee: SendFee) -> Result<String> {
        self.backend.wallet_send(address, amount_sat, fee)
    }

    fn wallet_send_all(&self, address: &str, fee: SendFee) -> Result<String> {
        self.backend.wallet_send_all(address, fee)
    }

    fn wallet_build_funding(
        &self,
        address: &str,
        amount_sat: u64,
    ) -> Result<(String, u32, String)> {
        // Funding prices at the per-coin ~30-min target (see
        // funding_conf_target), mirroring the Core-RPC backend; the
        // build-and-hold mechanics (input reservation, NON-replaceable
        // broadcast signal) live in the extracted backend.
        self.backend.wallet_build_funding(
            address,
            amount_sat,
            SendFee::Target(self.funding_conf_target()),
        )
    }

    fn wallet_cancel_funding(&self, tx_hex: &str) -> Result<()> {
        self.backend.wallet_cancel_funding(tx_hex)
    }

    fn wallet_transactions(&self) -> Result<Vec<WalletTxInfo>> {
        self.backend.wallet_transactions()
    }

    fn wallet_locked(&self) -> Result<bool> {
        Ok(self.backend.wallet_locked())
    }

    fn wallet_sign_send(
        &self,
        tx: &Transaction,
        prevout_value_sat: u64,
        prevout_spk: &ScriptBuf,
    ) -> Result<Txid> {
        self.backend
            .wallet_sign_send(tx, prevout_value_sat, prevout_spk)
    }

    fn wallet_tx_fee_vsize(&self, txid: &str) -> Result<(u64, u64)> {
        self.backend.wallet_tx_fee_vsize(txid)
    }

    fn wallet_change_output(
        &self,
        funding_txid: &str,
        htlc_spk: &ScriptBuf,
    ) -> Result<Option<(u32, u64, ScriptBuf)>> {
        self.backend.wallet_change_output(funding_txid, htlc_spk)
    }

    fn wallet_bumpfee(&self, txid: &str, feerate_sat_kvb: u64) -> Result<String> {
        self.backend.wallet_bumpfee(txid, feerate_sat_kvb)
    }
}
