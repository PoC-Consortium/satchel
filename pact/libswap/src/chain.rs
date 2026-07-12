//! Chain backends — spec §10.
//!
//! All backend data is an untrusted hint: scripts and amounts are verified
//! against locally reconstructed bytes, and refund scheduling is purely
//! clock-driven. A lying backend can withhold or delay, never steal.
//!
//! The [`ChainBackend`] trait, the Core-RPC backend (the user's own pocx
//! node / bitcoind; wallet-qualified URL = the user's core wallet on that
//! node) and the [`MultiBackend`] quorum live here. The Electrum transport
//! itself was extracted to the `electrum-btcx` crate ([`ElectrumBackend`],
//! [`ElectrumPool`]); this module re-exports it and adapts it to the trait.

use anyhow::{bail, Context, Result};
use bitcoin::{OutPoint, ScriptBuf, Transaction, Txid};
use serde_json::{json, Value};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::params::{ChainParams, Network};
use crate::rpc::{RpcClient, RpcError};

// The Electrum layer and the shared send/fee/txout types moved to the
// extracted crates; re-exported so `crate::chain::…` callers are unchanged.
pub use electrum_btcx::{
    ChainMismatch, ElectrumBackend, ElectrumPool, SendFee, SendFeeEstimates, TxOutInfo,
};
pub use wallet_btcx::WalletTxInfo;

use electrum_btcx::backend::{btc_kvb_to_sat_kvb, kvb_to_vb_round};
use electrum_btcx::SANITY_MAX_SAT_PER_VB;

/// Test-only market-feerate override (sat/vB); 0 = unset. Set via the
/// regtest-gated `_settestfeerate` RPC and honored by [`RpcBackend::fee_rate_sat_per_vb`]
/// ONLY on regtest, so the harness can manufacture the market-vs-broadcast gap
/// the fee-bump nurse reacts to. estimatesmartfee can't be primed cheaply on
/// regtest and `settxfee` is gone in Core v31, so this is the only deterministic
/// lever. NEVER consulted on mainnet/testnet.
static TEST_FEERATE_OVERRIDE_SAT_VB: AtomicU64 = AtomicU64::new(0);

/// Set the regtest-only market-feerate override (sat/vB; 0 clears it). Safe to
/// call on any network — [`RpcBackend::fee_rate_sat_per_vb`] only reads it on
/// regtest — but the `_settestfeerate` RPC additionally refuses off regtest.
pub fn set_test_feerate(sat_vb: u64) {
    TEST_FEERATE_OVERRIDE_SAT_VB.store(sat_vb, Ordering::Relaxed);
}

/// Wall-clock cap (seconds) for how long a funding lock should wait to confirm.
/// Divided by the coin's block spacing to derive `estimatesmartfee`'s
/// `conf_target` for funding (see [`ChainBackend::funding_conf_target`]), so the
/// funding feerate targets confirmation within ~30 min on any coin instead of a
/// blind 6-block target — which is a full hour on Bitcoin's 10-min blocks.
pub(crate) const FUNDING_TARGET_SECS: u32 = 1800;

/// Ceiling on the derived funding `conf_target`: the established
/// `estimatesmartfee(6)` baseline. The wall-clock cap only pulls the target
/// *faster* (lower) on slow chains; it never targets a cheaper/slower confirmation
/// than the historical default on fast chains (e.g. Litecoin's 2.5-min blocks
/// would give 12, clamped back to 6).
pub(crate) const FUNDING_CONF_TARGET_MAX: u16 = 6;

/// Pure per-coin funding `conf_target` derivation (see
/// [`ChainBackend::funding_conf_target`]): the number of blocks that fits the
/// [`FUNDING_TARGET_SECS`] wall-clock budget at this coin's block spacing,
/// clamped to `1..=FUNDING_CONF_TARGET_MAX`. A zero spacing is guarded to 1.
pub(crate) fn funding_conf_target_for(target_spacing_secs: u32) -> u16 {
    let spacing = target_spacing_secs.max(1);
    ((FUNDING_TARGET_SECS / spacing) as u16).clamp(1, FUNDING_CONF_TARGET_MAX)
}

/// Is this broadcast error really "the tx is already in the chain / mempool"?
/// Re-broadcasting an already-confirmed tx (a refund/redeem the scheduler keeps
/// nudging, or a funding the maker re-sends) must be a no-op success, not an
/// error that loops forever. Core returns code -27 (outputs already in utxo
/// set / transaction already in block chain); other versions/backends phrase
/// it as an "already in ..." / "already known" message, so we also match text.
fn is_already_broadcast(err: &anyhow::Error) -> bool {
    if let Some(rpc) = err.downcast_ref::<RpcError>() {
        if rpc.code == -27 {
            return true;
        }
    }
    let msg = format!("{err:#}").to_ascii_lowercase();
    msg.contains("already in") || msg.contains("already known")
}

pub trait ChainBackend: Send + Sync {
    fn params(&self) -> &ChainParams;

    /// Verify the backend serves the expected chain (genesis hash check,
    /// spec §3.3). MUST be called before any funding decision. Wrong-chain
    /// answers are reported as [`ChainMismatch`] so multi-view quorums can
    /// tell disagreement (fatal) from absence (skippable).
    fn verify_chain(&self) -> Result<()>;

    /// The health cell of the SERVER this backend's chain reads ride on —
    /// `None` for backends without one (Core RPC has no breaker; its
    /// failures are handled per call). `MultiBackend` skips a view whose
    /// cell is inside a failure-backoff window instead of paying its
    /// connect timeout on every request (issue #98).
    fn view_health(&self) -> Option<Arc<crate::server_health::ServerHealth>> {
        None
    }

    fn broadcast(&self, tx: &Transaction) -> Result<Txid>;

    /// `None` if the outpoint does not exist or is already spent.
    /// `expected_spk` is the script the output is supposed to pay —
    /// Electrum can only look up outputs by script, and Core-RPC backends
    /// may use it as a cross-check.
    fn get_txout(&self, outpoint: &OutPoint, expected_spk: &ScriptBuf)
        -> Result<Option<TxOutInfo>>;

    /// Find an unspent output paying `spk` (the locally derivable HTLC
    /// scriptPubKey) — chain-watched funding detection for when the `funded`
    /// relay message is absent or hasn't arrived. Returns the outpoint + its
    /// info, or `None` if nothing pays `spk` yet. Like every backend read this
    /// is a hint; callers re-verify value/script and apply the confirmation gate.
    fn find_funding(&self, spk: &ScriptBuf) -> Result<Option<(OutPoint, TxOutInfo)>>;

    /// Locate the output of `txid` paying `script_pubkey_hex` (funding-tx
    /// vout discovery; the tx is in our own node's mempool/wallet).
    fn find_vout(&self, txid: &str, script_pubkey_hex: &str) -> Result<u32>;

    /// Witness items of the input spending `outpoint` (an output paying
    /// `watch_spk`), searching the mempool and recent blocks (preimage
    /// extraction, spec §9.4). `None` if no spend is visible yet.
    fn find_spend_witness(
        &self,
        outpoint: &OutPoint,
        watch_spk: &ScriptBuf,
        from_height: u64,
    ) -> Result<Option<Vec<Vec<u8>>>>;

    /// Full transaction history of a script — funding AND spends, past or
    /// present, confirmed or mempool — as `(txid, height)` pairs in server
    /// order (oldest first, mempool entries last with `height <= 0`). This is
    /// what lets swap state be reconstructed for outputs that are ALREADY
    /// SPENT, which no live-UTXO read (`find_funding`/`get_txout`) can see.
    ///
    /// `Ok(None)` = this backend has no script index (Core RPC — `txindex`
    /// is txid→tx only, there is no address index), the capability floor
    /// callers degrade from: live reads while in flight + the timelock
    /// age-out for terminal decisions. Like every backend read the entries
    /// are hints — callers verify the referenced transactions against
    /// locally reconstructed scripts/amounts.
    fn spk_history(&self, _spk: &ScriptBuf) -> Result<Option<Vec<(String, i64)>>> {
        Ok(None)
    }

    /// Fetch a full transaction by txid, for inspecting the outputs/witnesses
    /// of history entries discovered via [`Self::spk_history`]. `Ok(None)`
    /// when the backend cannot see the tx (transport hiccup, or Core without
    /// `-txindex` once a tx leaves the mempool); callers treat that as
    /// inconclusive, never as evidence. Answers are self-verifying — the
    /// caller re-checks `tx.compute_txid()` and the scripts it cares about.
    fn fetch_tx(&self, _txid: &str) -> Result<Option<Transaction>> {
        Ok(None)
    }

    /// The NODE WALLET's own transactions (full, decoded) with their block
    /// heights (0 = unconfirmed), from `since_height` to the tip — the
    /// wallet-assisted reconstruction source (#171, docs/STATE_RECONSTRUCTION.md
    /// §4). The multi-machine backup-session contract makes this history
    /// SHARED across a merchant's machines (a backup MUST point at the same
    /// wallet — takeovers fund from it and v2 sweeps pay into it), so it
    /// covers every transaction the merchant side ever made: fundings we
    /// sent, claims/refunds we received — even long after the outputs are
    /// spent, which no live-UTXO read can see.
    ///
    /// POSITIVE-ONLY evidence: the counterparty's transactions are not
    /// wallet transactions, so absence here proves nothing — callers may use
    /// returned data to prove a funding/spend happened, never to conclude
    /// one didn't. `Ok(None)` = this backend has no wallet.
    fn wallet_txs_since(&self, _since_height: u64) -> Result<Option<Vec<(Transaction, u64)>>> {
        Ok(None)
    }

    fn tip_height(&self) -> Result<u64>;

    /// Median-time-past of the tip — what CLTV is evaluated against.
    fn tip_median_time(&self) -> Result<u64>;

    /// Confirmations of a transaction (0 if unconfirmed or unknown).
    /// `spk_hint` is a script the transaction pays (our sweep output) —
    /// required by Electrum backends, which can only search by script.
    fn tx_confirmations(&self, txid: &str, spk_hint: Option<&ScriptBuf>) -> Result<u64>;

    /// Feerate in sat/vB from the node's estimator for a given confirmation
    /// target and estimate mode, with a conservative fallback when the estimator
    /// has no data (fresh chains, regtest). `conservative = false` preserves the
    /// original baseline exactly (Core's default estimate_mode); `conservative =
    /// true` requests Core's CONSERVATIVE mode for a robuster (higher) estimate —
    /// the lever the deadline-aware redeem nurse escalates, together with a tighter
    /// `conf_target`, as a timelock approaches. Backends without a mode distinction
    /// (Electrum) ignore `conservative`.
    fn fee_rate_for(&self, conf_target: u16, conservative: bool) -> Result<u64>;

    /// Feerate for the default "normal" target (6 blocks, economical) — the
    /// baseline every non-deadline-pressured nurse and estimate uses.
    fn fee_rate_sat_per_vb(&self) -> Result<u64> {
        self.fee_rate_for(6, false)
    }

    /// The estimator's RAW answer for `conf_target` in sat/vB — `None` when it
    /// has no data (fresh chain, quiet mempool, regtest), where
    /// [`Self::fee_rate_for`] would silently substitute the 1 sat/vB fallback.
    /// The send form needs the distinction to mirror phoenix's preset logic:
    /// estimate-less presets are disabled and the form falls back to a custom
    /// rate at the coin floor. Estimates are floored to `min_feerate_sat_vb`.
    /// Chain-data-less backends report no estimate.
    fn fee_estimate(&self, _conf_target: u16) -> Result<Option<u64>> {
        Ok(None)
    }

    /// Precise market estimate in sat/kvB (the estimator's native
    /// resolution). Default derives from the integer sat/vB path so backends
    /// without an estimator stay correct; Core/Electrum override with the
    /// exact value.
    fn fee_estimate_kvb(&self, conf_target: u16) -> Result<Option<u64>> {
        Ok(self.fee_estimate(conf_target)?.map(|vb| vb * 1000))
    }

    /// [`ChainBackend::fee_rate_for`] at sat/kvB resolution (same fallback
    /// semantics). Sends and swap funding price off THIS — the fraction is
    /// real queue priority at the bottom of the market (rc10 field report), and
    /// the bump nurses use it so a sub-integer market move isn't rounded away
    /// (which stranded funding: the RBF then landed below the node's fractional
    /// Rule-4 minimum). `conservative` mirrors [`Self::fee_rate_for`] — the
    /// deadline-escalated claim bands ask for the robuster estimate.
    fn fee_rate_for_kvb(&self, conf_target: u16, conservative: bool) -> Result<u64> {
        Ok(self.fee_rate_for(conf_target, conservative)? * 1000)
    }

    /// Resolve a [`SendFee`] to the sat/kvB rate a send prices itself at:
    /// market estimate (with fallback) for a target, or the explicit rate
    /// clamped to the coin floor and the sanity max.
    fn resolve_send_fee(&self, fee: SendFee) -> Result<u64> {
        match fee {
            SendFee::Target(conf_target) => self.fee_rate_for_kvb(conf_target, false),
            SendFee::RatePerKvb(rate) => Ok(rate
                .clamp(1, SANITY_MAX_SAT_PER_VB * 1000)
                .max(self.params().min_feerate_sat_vb * 1000)),
        }
    }

    /// Funding-specific `estimatesmartfee` target (blocks), derived per coin from a
    /// fixed wall-clock cap: `clamp(FUNDING_TARGET_SECS / target_spacing_secs, 1,
    /// FUNDING_CONF_TARGET_MAX)`. Bitcoin's 10-min blocks → 3 (a 30-min budget);
    /// faster chains keep the standard 6 (Litecoin's 6 blocks ≈ 15 min is already
    /// inside the budget). Used everywhere funding picks a feerate — the initial
    /// broadcast, the funding nurse's market term, and the funds-gate headroom — so
    /// a lock doesn't sit an hour on a slow chain, with no per-coin config. Redeem
    /// and refund keep their own (deadline-aware / flat-6) targets.
    fn funding_conf_target(&self) -> u16 {
        funding_conf_target_for(self.params().target_spacing_secs)
    }

    /// Whether `txid` is currently in this node's mempool. The bump loop uses
    /// `!is_in_mempool` as the *only* trigger to re-broadcast an unchanged tx
    /// (recover from eviction), so steady state stays silent. Defaults to
    /// `true` (assume present → don't churn) for backends that can't see a
    /// mempool; mempool-aware backends report real eviction.
    fn is_in_mempool(&self, _txid: &str) -> Result<bool> {
        Ok(true)
    }

    /// The node's `incrementalrelayfee` (sat/vB, rounded up, min 1) — the
    /// minimum a BIP125 replacement must beat the replaced tx by (Rule 4).
    /// Defaults to 1 sat/vB when the node can't report it.
    fn incremental_relay_feerate(&self) -> Result<u64> {
        Ok(1)
    }

    /// [`Self::incremental_relay_feerate`] at **sat/kvB** resolution — the
    /// minimum a BIP125 replacement must beat the replaced tx by, kept at the
    /// estimator's native granularity so the funding-RBF Rule-4 floor is exact
    /// (`incrementalrelayfee` is quoted in BTC/kvB). Defaults to the integer
    /// sat/vB value ×1000; Core overrides with the precise node value.
    fn incremental_relay_feerate_kvb(&self) -> Result<u64> {
        Ok(self
            .incremental_relay_feerate()?
            .saturating_mul(1000)
            .max(1000))
    }

    /// Fresh receive address from the user's core wallet (sweep target).
    fn wallet_new_address(&self) -> Result<String>;

    /// Confirmed core-wallet balance in base units.
    fn wallet_balance(&self) -> Result<u64>;

    /// Fund `address` with exactly `amount_sat` via the core wallet
    /// (HTLC funding is a normal send, spec §6.1). `fee` is how the send
    /// prices itself — funding callers pass
    /// `SendFee::Target(funding_conf_target())` (the per-coin ~30-min
    /// target); the user send passes the form's preset target or custom rate.
    fn wallet_send(&self, address: &str, amount_sat: u64, fee: SendFee) -> Result<String>;

    /// Sweep the whole wallet to `address` ("send everything", phoenix
    /// parity): every spendable UTXO in one tx with the fee taken out of the
    /// swept amount — the recipient receives balance − fee and the wallet is
    /// left empty. UTXOs reserved by built-but-unbroadcast v2 fundings are
    /// not spendable, so a sweep can never claw back a reservation.
    fn wallet_send_all(&self, _address: &str, _fee: SendFee) -> Result<String> {
        bail!("this backend cannot sweep the wallet")
    }

    /// Build + sign (but DO NOT broadcast) a funding tx paying `amount_sat` to
    /// `address`, returning `(txid, vout, signed_tx_hex)`. The selected inputs
    /// are locked so nothing else spends them before we broadcast. Used by the v2
    /// two-phase funding (spec v2 §7): the funding txid must be known to pre-sign
    /// the redeems, but real funds are committed only at broadcast — after the
    /// adaptor signatures verify and (for the participant) the counterparty leg is
    /// confirmed. Default: unsupported (only the Core wallet backend builds txs).
    fn wallet_build_funding(
        &self,
        _address: &str,
        _amount_sat: u64,
    ) -> Result<(String, u32, String)> {
        bail!("this backend cannot build funding transactions without broadcasting")
    }

    /// Release the input reservation of a [`Self::wallet_build_funding`] tx that
    /// will NEVER be broadcast (the swap went terminal before its two-phase
    /// broadcast). `tx_hex` is the exact signed tx the build returned. Without
    /// this, the built tx's inputs stay reserved forever — Core keeps them in
    /// `lockunspent` until a node restart, and the bdk wallet persists the
    /// phantom unbroadcast tx across restarts. Callers gate on the tx being
    /// absent from the chain AND our own broadcast flag; implementations may
    /// add their own refusal for an on-chain tx. Default: no-op (chain-only
    /// backends never built anything).
    fn wallet_cancel_funding(&self, _tx_hex: &str) -> Result<()> {
        Ok(())
    }

    /// The wallet's transaction history, newest first — the activity feed behind
    /// the `listtransactions` RPC (design doc §4). Only the nodeless bdk wallet
    /// serves this (Core-backed coins stay read-only in Satchel by design), so
    /// the default refuses.
    fn wallet_transactions(&self) -> Result<Vec<WalletTxInfo>> {
        bail!("wallet activity requires a nodeless (Electrum-backed) coin")
    }

    /// Whether the node's wallet is encrypted AND currently locked — it can read
    /// balances but cannot SIGN, so a funding `wallet_send` would fail with RPC
    /// -13 ("walletpassphrase first"). `Ok(false)` for unencrypted wallets or
    /// backends with no wallet concept (only the Core primary overrides this).
    fn wallet_locked(&self) -> Result<bool> {
        Ok(false)
    }

    /// Sign `tx`'s input(s) with the node wallet and broadcast it, given the
    /// value + scriptPubKey of its single prevout so a segwit/taproot input can
    /// be signed before that prevout confirms. Used to CPFP-bump an unconfirmed
    /// cooperative redeem (v2+): a self-funded child spending the redeem's own
    /// (wallet-owned sweep) output. Only the wallet-backed Core primary
    /// implements it; chain-only backends cannot sign.
    fn wallet_sign_send(
        &self,
        _tx: &Transaction,
        _prevout_value_sat: u64,
        _prevout_spk: &ScriptBuf,
    ) -> Result<Txid> {
        bail!("this backend has no wallet; cannot sign a CPFP child")
    }

    /// A wallet tx's fee (sat) + vsize (vB), for recomputing a funding's broadcast
    /// feerate at bump time (`fee / vsize`) without persisting it. Wallet-backed
    /// Core primary only (the funding nurse).
    fn wallet_tx_fee_vsize(&self, _txid: &str) -> Result<(u64, u64)> {
        bail!("this backend has no wallet; cannot read a tx fee/vsize")
    }

    /// The wallet-OWNED change output of `funding_txid` — `(vout, value_sat, spk)`
    /// — for a CPFP child on a v2 funding. Identified positively by `ismine` (the
    /// HTLC output is a P2WSH/P2TR script the wallet does NOT own, so `ismine`
    /// cleanly selects the change). `None` when the funding has no change output
    /// (exact-UTXO funding → can't CPFP). `htlc_spk` is skipped explicitly as a
    /// belt-and-suspenders cross-check. Wallet-backed Core primary only.
    fn wallet_change_output(
        &self,
        _funding_txid: &str,
        _htlc_spk: &ScriptBuf,
    ) -> Result<Option<(u32, u64, ScriptBuf)>> {
        bail!("this backend has no wallet; cannot find a change output")
    }

    /// RBF-bump a wallet-owned tx via the node's `bumpfee`, targeting `feerate`
    /// (**sat/kvB**, the estimator's native resolution — so a 1.004 sat/vB tx is
    /// bumped to a precise rate that clears the node's fractional Rule-4 minimum,
    /// not rounded to a whole sat/vB that lands just below it); returns the
    /// replacement txid. The v1 funding nurse: the funding is wallet-owned and
    /// broadcast BIP125-replaceable. Errors if not replaceable or the wallet
    /// can't afford the higher fee. Wallet-backed Core primary only.
    fn wallet_bumpfee(&self, _txid: &str, _feerate_sat_kvb: u64) -> Result<String> {
        bail!("this backend has no wallet; cannot bumpfee")
    }
}

/// A shared backend behaves exactly like the backend itself. EVERY method is
/// forwarded — including the ones with trait defaults — so a backend's
/// overrides are never shadowed by the defaults through the `Arc`. Lets the
/// engine put pooled long-lived [`ElectrumBackend`]s (one connection per
/// server, shared with the wallet sync worker, issue #87) into a
/// [`MultiBackend`] alongside owned per-call backends.
impl<T: ChainBackend + ?Sized> ChainBackend for std::sync::Arc<T> {
    fn params(&self) -> &ChainParams {
        (**self).params()
    }
    fn verify_chain(&self) -> Result<()> {
        (**self).verify_chain()
    }
    fn view_health(&self) -> Option<Arc<crate::server_health::ServerHealth>> {
        (**self).view_health()
    }
    fn broadcast(&self, tx: &Transaction) -> Result<Txid> {
        (**self).broadcast(tx)
    }
    fn get_txout(
        &self,
        outpoint: &OutPoint,
        expected_spk: &ScriptBuf,
    ) -> Result<Option<TxOutInfo>> {
        (**self).get_txout(outpoint, expected_spk)
    }
    fn find_funding(&self, spk: &ScriptBuf) -> Result<Option<(OutPoint, TxOutInfo)>> {
        (**self).find_funding(spk)
    }
    fn find_vout(&self, txid: &str, script_pubkey_hex: &str) -> Result<u32> {
        (**self).find_vout(txid, script_pubkey_hex)
    }
    fn find_spend_witness(
        &self,
        outpoint: &OutPoint,
        watch_spk: &ScriptBuf,
        from_height: u64,
    ) -> Result<Option<Vec<Vec<u8>>>> {
        (**self).find_spend_witness(outpoint, watch_spk, from_height)
    }
    fn spk_history(&self, spk: &ScriptBuf) -> Result<Option<Vec<(String, i64)>>> {
        (**self).spk_history(spk)
    }
    fn fetch_tx(&self, txid: &str) -> Result<Option<Transaction>> {
        (**self).fetch_tx(txid)
    }
    fn wallet_txs_since(&self, since_height: u64) -> Result<Option<Vec<(Transaction, u64)>>> {
        (**self).wallet_txs_since(since_height)
    }
    fn tip_height(&self) -> Result<u64> {
        (**self).tip_height()
    }
    fn tip_median_time(&self) -> Result<u64> {
        (**self).tip_median_time()
    }
    fn tx_confirmations(&self, txid: &str, spk_hint: Option<&ScriptBuf>) -> Result<u64> {
        (**self).tx_confirmations(txid, spk_hint)
    }
    fn fee_rate_for(&self, conf_target: u16, conservative: bool) -> Result<u64> {
        (**self).fee_rate_for(conf_target, conservative)
    }
    fn fee_rate_sat_per_vb(&self) -> Result<u64> {
        (**self).fee_rate_sat_per_vb()
    }
    fn fee_estimate(&self, conf_target: u16) -> Result<Option<u64>> {
        (**self).fee_estimate(conf_target)
    }
    fn fee_estimate_kvb(&self, conf_target: u16) -> Result<Option<u64>> {
        (**self).fee_estimate_kvb(conf_target)
    }
    fn fee_rate_for_kvb(&self, conf_target: u16, conservative: bool) -> Result<u64> {
        (**self).fee_rate_for_kvb(conf_target, conservative)
    }
    fn resolve_send_fee(&self, fee: SendFee) -> Result<u64> {
        (**self).resolve_send_fee(fee)
    }
    fn funding_conf_target(&self) -> u16 {
        (**self).funding_conf_target()
    }
    fn is_in_mempool(&self, txid: &str) -> Result<bool> {
        (**self).is_in_mempool(txid)
    }
    fn incremental_relay_feerate(&self) -> Result<u64> {
        (**self).incremental_relay_feerate()
    }
    fn incremental_relay_feerate_kvb(&self) -> Result<u64> {
        (**self).incremental_relay_feerate_kvb()
    }
    fn wallet_new_address(&self) -> Result<String> {
        (**self).wallet_new_address()
    }
    fn wallet_balance(&self) -> Result<u64> {
        (**self).wallet_balance()
    }
    fn wallet_send(&self, address: &str, amount_sat: u64, fee: SendFee) -> Result<String> {
        (**self).wallet_send(address, amount_sat, fee)
    }
    fn wallet_send_all(&self, address: &str, fee: SendFee) -> Result<String> {
        (**self).wallet_send_all(address, fee)
    }
    fn wallet_build_funding(
        &self,
        address: &str,
        amount_sat: u64,
    ) -> Result<(String, u32, String)> {
        (**self).wallet_build_funding(address, amount_sat)
    }
    fn wallet_cancel_funding(&self, tx_hex: &str) -> Result<()> {
        (**self).wallet_cancel_funding(tx_hex)
    }
    fn wallet_transactions(&self) -> Result<Vec<WalletTxInfo>> {
        (**self).wallet_transactions()
    }
    fn wallet_locked(&self) -> Result<bool> {
        (**self).wallet_locked()
    }
    fn wallet_sign_send(
        &self,
        tx: &Transaction,
        prevout_value_sat: u64,
        prevout_spk: &ScriptBuf,
    ) -> Result<Txid> {
        (**self).wallet_sign_send(tx, prevout_value_sat, prevout_spk)
    }
    fn wallet_tx_fee_vsize(&self, txid: &str) -> Result<(u64, u64)> {
        (**self).wallet_tx_fee_vsize(txid)
    }
    fn wallet_change_output(
        &self,
        funding_txid: &str,
        htlc_spk: &ScriptBuf,
    ) -> Result<Option<(u32, u64, ScriptBuf)>> {
        (**self).wallet_change_output(funding_txid, htlc_spk)
    }
    fn wallet_bumpfee(&self, txid: &str, feerate_sat_kvb: u64) -> Result<String> {
        (**self).wallet_bumpfee(txid, feerate_sat_kvb)
    }
}

/// bitcoind-style cookie auto-discovery (#162): the default node-cookie
/// locations for `params`' coin+network, used when a Core-RPC URL carries
/// neither credentials nor a `__cookiefile__:` path — the same convention
/// `bitcoin-cli` applies with no `-rpcuser`/`-rpccookiefile`. Candidates are
/// PROBED at call time (first existing file wins), so a node started later is
/// picked up and a restart's fresh cookie is re-read via the 401 self-heal.
///
/// Node data-dir name: the coin's registry display name with spaces
/// hyphenated ("Bitcoin PoCX" → "Bitcoin-PoCX"), matching the shipped
/// coins.toml `%NODEDIR%/<Name>` defaults — %LOCALAPPDATA%\<Name> (modern
/// Core + bitcoin-pocx) or the older %APPDATA%\<Name> on Windows,
/// ~/Library/Application Support/<Name> on macOS, ~/.<name lowercased>
/// elsewhere. Testnet subdir naming differs per coin family (Bitcoin
/// `testnet3`, Litecoin `testnet4`, PoCX `testnet`) — all are probed;
/// existence at call time disambiguates.
pub fn default_cookie_candidates(params: &ChainParams) -> Vec<std::path::PathBuf> {
    use std::path::PathBuf;
    let Some(def) = crate::registry::get(params.coin_id) else {
        return Vec::new();
    };
    let name = def.display_name.replace(' ', "-");
    let mut bases: Vec<PathBuf> = Vec::new();
    if cfg!(windows) {
        for var in ["LOCALAPPDATA", "APPDATA"] {
            if let Some(v) = std::env::var_os(var) {
                bases.push(PathBuf::from(v).join(&name));
            }
        }
    } else if cfg!(target_os = "macos") {
        if let Some(home) = std::env::var_os("HOME") {
            bases.push(
                PathBuf::from(home)
                    .join("Library/Application Support")
                    .join(&name),
            );
        }
    } else if let Some(home) = std::env::var_os("HOME") {
        bases.push(PathBuf::from(home).join(format!(".{}", name.to_lowercase())));
    }
    let subdirs: &[&str] = match params.network {
        Network::Mainnet => &[""],
        Network::Testnet => &["testnet", "testnet3", "testnet4"],
        Network::Regtest => &["regtest"],
    };
    let mut out = Vec::new();
    for base in &bases {
        for sub in subdirs {
            let dir = if sub.is_empty() {
                base.clone()
            } else {
                base.join(sub)
            };
            out.push(dir.join(".cookie"));
        }
    }
    out
}

/// Bitcoin Core / pocx-node JSON-RPC backend.
pub struct CoreRpcBackend {
    params: &'static ChainParams,
    rpc: RpcClient,
}

impl CoreRpcBackend {
    /// Build from a `http://…` URL. Auth resolution (#162, bitcoind
    /// precedence): explicit credentials in the URL are used verbatim; a
    /// `__cookiefile__:<path>` userinfo names the node cookie FILE (read live,
    /// re-read once on a 401); a URL with NO userinfo auto-discovers the
    /// cookie in this coin's platform-default node data dir
    /// ([`default_cookie_candidates`]) — `bitcoin-cli`'s no-flags behavior.
    pub fn new(params: &'static ChainParams, url: &str) -> Result<Self> {
        Ok(Self {
            params,
            rpc: RpcClient::from_url_or_cookie(url, default_cookie_candidates(params))?,
        })
    }

    /// Raw `estimatesmartfee` answer in sat/vB, or `None` when the node has no
    /// estimate (fresh/quiet chain). Shared by `fee_rate_for` (which adds the
    /// 1 sat/vB fallback) and `fee_estimate` (which surfaces the `None`).
    fn smart_fee_estimate_kvb(&self, conf_target: u16, conservative: bool) -> Option<u64> {
        // Regtest-only test override: the harness injects a market feerate to
        // create a market-vs-broadcast gap the bump nurse reacts to (see
        // `set_test_feerate`). Never honored off regtest.
        if self.params.network == Network::Regtest {
            let ov = TEST_FEERATE_OVERRIDE_SAT_VB.load(Ordering::Relaxed);
            if ov > 0 {
                return Some(
                    (ov * 1000)
                        .clamp(1000, SANITY_MAX_SAT_PER_VB * 1000)
                        .max(self.params.min_feerate_sat_vb * 1000),
                );
            }
        }
        // Preserve the original baseline EXACTLY: `estimatesmartfee(conf_target)`
        // with no mode arg → Core's default estimate. Only the deadline-escalated
        // bands pass an explicit CONSERVATIVE mode for a robuster (higher) estimate.
        let args = if conservative {
            vec![json!(conf_target), json!("CONSERVATIVE")]
        } else {
            vec![json!(conf_target)]
        };
        // estimatesmartfee already honors the node's mempool/relay floor WHEN it
        // returns an estimate (Core src/rpc/fees.cpp), but some wallets reject
        // anything below a higher baked-in `-mintxfee` that no RPC exposes
        // (Litecoin's is ~10 sat/vB), giving -6 "lower than the minimum fee rate
        // setting". `min_feerate_sat_vb` carries that per-coin floor (coins.toml
        // for file coins, 1 for the built-ins); applied AFTER the sanity clamp so
        // the coin's floor always wins.
        self.rpc
            .call("estimatesmartfee", &args)
            .ok()
            .and_then(|r| r["feerate"].as_f64()) // BTC per kvB
            .map(btc_kvb_to_sat_kvb)
            .map(|est| {
                est.clamp(1, SANITY_MAX_SAT_PER_VB * 1000)
                    .max(self.params.min_feerate_sat_vb * 1000)
            })
    }

    fn vin_matches(vin: &Value, outpoint: &OutPoint) -> bool {
        vin["txid"].as_str() == Some(outpoint.txid.to_string().as_str())
            && vin["vout"].as_u64() == Some(u64::from(outpoint.vout))
    }

    fn witness_of(vin: &Value) -> Result<Vec<Vec<u8>>> {
        let items = vin["txinwitness"].as_array().cloned().unwrap_or_default();
        items
            .iter()
            .map(|item| hex::decode(item.as_str().unwrap_or_default()).context("bad witness hex"))
            .collect()
    }
}

impl ChainBackend for CoreRpcBackend {
    fn params(&self) -> &ChainParams {
        self.params
    }

    fn verify_chain(&self) -> Result<()> {
        let genesis = self.rpc.call("getblockhash", &[json!(0)])?;
        let genesis = genesis.as_str().context("getblockhash: non-string")?;
        if genesis != self.params.genesis_hash {
            return Err(anyhow::Error::new(ChainMismatch(format!(
                "backend serves the wrong chain: genesis {genesis}, expected {} ({} {:?})",
                self.params.genesis_hash, self.params.coin_id, self.params.network
            ))));
        }
        Ok(())
    }

    fn broadcast(&self, tx: &Transaction) -> Result<Txid> {
        let hex = bitcoin::consensus::encode::serialize_hex(tx);
        match self.rpc.call("sendrawtransaction", &[json!(hex)]) {
            Ok(txid) => Ok(Txid::from_str(
                txid.as_str().context("sendrawtransaction: non-string")?,
            )?),
            // Already mined / in the mempool: the tx is on its way, not an error.
            Err(e) if is_already_broadcast(&e) => Ok(tx.compute_txid()),
            Err(e) => Err(e),
        }
    }

    fn get_txout(
        &self,
        outpoint: &OutPoint,
        _expected_spk: &ScriptBuf,
    ) -> Result<Option<TxOutInfo>> {
        let result = self.rpc.call(
            "gettxout",
            &[
                json!(outpoint.txid.to_string()),
                json!(outpoint.vout),
                json!(true),
            ],
        )?;
        if result.is_null() {
            return Ok(None);
        }
        let btc_value = result["value"].as_f64().context("gettxout: no value")?;
        Ok(Some(TxOutInfo {
            // Round-trip through the node's 8-decimal float is exact for
            // amounts < 2^53 / 1e8 (~90M coins) — fine for swap sizes.
            value_sat: (btc_value * 1e8).round() as u64,
            script_pubkey_hex: result["scriptPubKey"]["hex"]
                .as_str()
                .context("gettxout: no scriptPubKey hex")?
                .to_string(),
            confirmations: result["confirmations"].as_u64().unwrap_or(0),
        }))
    }

    fn find_funding(&self, spk: &ScriptBuf) -> Result<Option<(OutPoint, TxOutInfo)>> {
        // `scantxoutset` reads the UTXO set (no -txindex, no wallet); a
        // `raw(<spk>)` descriptor matches the exact HTLC script. It returns
        // confirmed outputs only — fine, since we gate on confirmations anyway.
        let desc = format!("raw({})", hex::encode(spk.as_bytes()));
        let result = self
            .rpc
            .call("scantxoutset", &[json!("start"), json!([desc])])?;
        let tip = result["height"].as_u64().unwrap_or(0);
        let Some(u) = result["unspents"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .next()
        else {
            return Ok(None);
        };
        let txid = Txid::from_str(u["txid"].as_str().context("scantxoutset: no txid")?)?;
        let vout = u["vout"].as_u64().context("scantxoutset: no vout")? as u32;
        let btc_value = u["amount"].as_f64().context("scantxoutset: no amount")?;
        let height = u["height"].as_u64().unwrap_or(0);
        let confirmations = if height > 0 && tip >= height {
            tip - height + 1
        } else {
            0
        };
        Ok(Some((
            OutPoint { txid, vout },
            TxOutInfo {
                value_sat: (btc_value * 1e8).round() as u64,
                script_pubkey_hex: hex::encode(spk.as_bytes()),
                confirmations,
            },
        )))
    }

    fn find_vout(&self, txid: &str, script_pubkey_hex: &str) -> Result<u32> {
        let tx = self
            .rpc
            .call("getrawtransaction", &[json!(txid), json!(true)])?;
        for vout in tx["vout"]
            .as_array()
            .context("getrawtransaction: no vout")?
        {
            if vout["scriptPubKey"]["hex"].as_str() == Some(script_pubkey_hex) {
                return Ok(vout["n"].as_u64().context("vout without n")? as u32);
            }
        }
        bail!("transaction {txid} has no output paying the expected script");
    }

    fn find_spend_witness(
        &self,
        outpoint: &OutPoint,
        watch_spk: &ScriptBuf,
        from_height: u64,
    ) -> Result<Option<Vec<Vec<u8>>>> {
        // Cheap gate: while the HTLC output is still unspent — the whole wait,
        // hours of ticks — `gettxout` (include_mempool) answers in ONE call, so we
        // never enumerate the mempool. It needs no -txindex (reads the UTXO set),
        // and only returns `None` once the output is actually spent (mempool OR a
        // block). Only then do we do the heavier search for the spending witness.
        if self.get_txout(outpoint, watch_spk)?.is_some() {
            return Ok(None);
        }
        // Mempool first: with frequent polling the reveal is normally caught here,
        // still unconfirmed, before it is mined. Tolerate the eviction race — a
        // txid from the `getrawmempool` snapshot can be mined/evicted before we
        // fetch it, and `getrawtransaction` then returns -5 ("No such mempool
        // transaction") with no -txindex; skip that tx instead of aborting the
        // whole scan (which would never reach the block fallback below).
        let mempool = self.rpc.call("getrawmempool", &[])?;
        for txid in mempool.as_array().cloned().unwrap_or_default() {
            let Ok(tx) = self
                .rpc
                .call("getrawtransaction", &[txid.clone(), json!(true)])
            else {
                continue; // evicted/mined since the snapshot — not the spend we seek
            };
            for vin in tx["vin"].as_array().cloned().unwrap_or_default() {
                if Self::vin_matches(&vin, outpoint) {
                    return Ok(Some(Self::witness_of(&vin)?));
                }
            }
        }
        // Fallback: the spend is already mined (we were down/slow during the
        // unconfirmed window). Scan blocks from the HTLC's funding height to the
        // tip — full blocks include witnesses, so this needs no -txindex either.
        let tip = self.tip_height()?;
        for height in from_height..=tip {
            let hash = self.rpc.call("getblockhash", &[json!(height)])?;
            let block = self.rpc.call("getblock", &[hash, json!(2)])?;
            for tx in block["tx"].as_array().cloned().unwrap_or_default() {
                for vin in tx["vin"].as_array().cloned().unwrap_or_default() {
                    if Self::vin_matches(&vin, outpoint) {
                        return Ok(Some(Self::witness_of(&vin)?));
                    }
                }
            }
        }
        Ok(None)
    }

    fn tip_height(&self) -> Result<u64> {
        self.rpc
            .call("getblockcount", &[])?
            .as_u64()
            .context("getblockcount: non-numeric")
    }

    fn tip_median_time(&self) -> Result<u64> {
        self.rpc.call("getblockchaininfo", &[])?["mediantime"]
            .as_u64()
            .context("getblockchaininfo: no mediantime")
    }

    fn tx_confirmations(&self, txid: &str, _spk_hint: Option<&ScriptBuf>) -> Result<u64> {
        // Our redeems/refunds pay the node's own wallet, so the wallet
        // knows them even after mining (getrawtransaction would need
        // -txindex once a tx leaves the mempool).
        if let Ok(tx) = self.rpc.call("gettransaction", &[json!(txid)]) {
            return Ok(tx["confirmations"].as_u64().unwrap_or(0));
        }
        match self
            .rpc
            .call("getrawtransaction", &[json!(txid), json!(true)])
        {
            Ok(tx) => Ok(tx["confirmations"].as_u64().unwrap_or(0)),
            Err(_) => Ok(0), // unknown to this node yet
        }
    }

    fn fee_rate_for(&self, conf_target: u16, conservative: bool) -> Result<u64> {
        // No estimate (empty/low-traffic mempool, or the node can't estimate) →
        // the fee market is effectively empty, so the relay minimum suffices
        // (floored to the coin's own minimum, see smart_fee_estimate). The bump
        // nurse covers the rare case where this later under-prices. The sanity
        // clamp inside smart_fee_estimate is an overflow guard only — NOT the
        // fee ceiling; the real caps live downstream in `FeeBumpPolicy`.
        Ok(self
            .smart_fee_estimate_kvb(conf_target, conservative)
            .map(|kvb| kvb_to_vb_round(kvb).max(1))
            .unwrap_or(self.params.min_feerate_sat_vb.max(1)))
    }

    fn fee_estimate(&self, conf_target: u16) -> Result<Option<u64>> {
        Ok(self
            .smart_fee_estimate_kvb(conf_target, false)
            .map(|kvb| kvb_to_vb_round(kvb).max(1)))
    }

    fn fee_estimate_kvb(&self, conf_target: u16) -> Result<Option<u64>> {
        Ok(self.smart_fee_estimate_kvb(conf_target, false))
    }

    fn fee_rate_for_kvb(&self, conf_target: u16, conservative: bool) -> Result<u64> {
        Ok(self
            .smart_fee_estimate_kvb(conf_target, conservative)
            .unwrap_or(self.params.min_feerate_sat_vb.max(1) * 1000))
    }

    fn is_in_mempool(&self, txid: &str) -> Result<bool> {
        // getmempoolentry succeeds iff the tx is in the mempool right now.
        Ok(self.rpc.call("getmempoolentry", &[json!(txid)]).is_ok())
    }

    fn incremental_relay_feerate(&self) -> Result<u64> {
        // Integer sat/vB view (ceil, min 1) derived from the precise sat/kvB one.
        let kvb = self.incremental_relay_feerate_kvb()?;
        Ok(kvb.div_ceil(1000).max(1))
    }

    fn incremental_relay_feerate_kvb(&self) -> Result<u64> {
        // `incrementalrelayfee` is quoted in BTC/kvB — keep it at that native
        // resolution (ceil, never round a required increment down), floored at
        // the 1000 sat/kvB (1 sat/vB) default when the node can't report it.
        let rate = self
            .rpc
            .call("getmempoolinfo", &[])
            .ok()
            .and_then(|r| r["incrementalrelayfee"].as_f64()) // BTC per kvB
            .map(|btc_kvb| (btc_kvb * 1e8).ceil() as u64)
            .unwrap_or(1000);
        Ok(rate.max(1000))
    }

    fn wallet_new_address(&self) -> Result<String> {
        Ok(self
            .rpc
            .call("getnewaddress", &[])?
            .as_str()
            .context("getnewaddress: non-string")?
            .to_string())
    }

    fn wallet_send(&self, address: &str, amount_sat: u64, fee: SendFee) -> Result<String> {
        // Amount as a decimal string: exact, no float in our code path.
        let amount = format!(
            "{}.{:08}",
            amount_sat / 100_000_000,
            amount_sat % 100_000_000
        );
        // Choose the feerate OURSELVES — the caller's explicit rate, or a market
        // estimate at the caller's target with the 1 sat/vB fallback when the
        // node can't estimate (a brand-new chain like BTCX with no fee history)
        // — and pass it explicitly. Otherwise the send leans on the node's
        // wallet estimator + `-fallbackfee`, which is disabled (0) by default on
        // mainnet, so `sendtoaddress` would *error* on a fee-history-less chain.
        // This mirrors how redeem/refund pick their rate (and phoenix-pocx's
        // 1 sat/vB fallback), so the fee tracks the same policy as everything
        // else instead of the node's config.
        // sat/kvB → the node's decimal sat/vB (Core accepts 3 decimals).
        let fee_rate = self.resolve_send_fee(fee)? as f64 / 1000.0;
        // Funding sends are RBF-bumped by the funding nurse and user sends by
        // the owner, so broadcast explicitly BIP125-replaceable rather than
        // relying on the node's -walletrbf default. Positional sendtoaddress args (Core 0.21+):
        // address, amount, comment, comment_to, subtractfeefromamount, replaceable,
        // conf_target, estimate_mode, avoid_reuse, fee_rate (sat/vB). conf_target is
        // left null (estimate_mode "unset") so the explicit fee_rate is what's used.
        let txid = self.rpc.call(
            "sendtoaddress",
            &[
                json!(address),
                json!(amount),
                json!(""),
                json!(""),
                json!(false),
                json!(true),
                json!(null),
                json!("unset"),
                json!(false),
                json!(fee_rate),
            ],
        )?;
        Ok(txid
            .as_str()
            .context("sendtoaddress: non-string")?
            .to_string())
    }

    fn wallet_send_all(&self, address: &str, fee: SendFee) -> Result<String> {
        let balance_sat = self.wallet_balance()?;
        anyhow::ensure!(balance_sat > 0, "wallet is empty — nothing to sweep");
        let amount = format!(
            "{}.{:08}",
            balance_sat / 100_000_000,
            balance_sat % 100_000_000
        );
        // sat/kvB → the node's decimal sat/vB (Core accepts 3 decimals).
        let fee_rate = self.resolve_send_fee(fee)? as f64 / 1000.0;
        // Same positional args + explicit-feerate/RBF policy as wallet_send,
        // except subtractfeefromamount=true: amount is the FULL confirmed
        // balance and Core takes the fee out of it — the sweep semantics.
        let txid = self.rpc.call(
            "sendtoaddress",
            &[
                json!(address),
                json!(amount),
                json!(""),
                json!(""),
                json!(true),
                json!(true),
                json!(null),
                json!("unset"),
                json!(false),
                json!(fee_rate),
            ],
        )?;
        Ok(txid
            .as_str()
            .context("sendtoaddress: non-string")?
            .to_string())
    }

    fn wallet_build_funding(
        &self,
        address: &str,
        amount_sat: u64,
    ) -> Result<(String, u32, String)> {
        let amount = format!(
            "{}.{:08}",
            amount_sat / 100_000_000,
            amount_sat % 100_000_000
        );
        // Funding prices at the per-coin ~30-min target (see funding_conf_target),
        // not a blind 6-block target — at full sat/kvB resolution, passed to the
        // node as decimal sat/vB (the fraction is real queue priority).
        let fee_rate = self.fee_rate_for_kvb(self.funding_conf_target(), false)? as f64 / 1000.0;
        // 1. raw tx carrying only the funding output (no inputs yet). The output
        //    key is the funding address, so build the object with a dynamic key.
        let mut outputs = serde_json::Map::new();
        outputs.insert(address.to_string(), json!(amount));
        let raw = self
            .rpc
            .call("createrawtransaction", &[json!([]), Value::Object(outputs)])?;
        let raw_hex = raw.as_str().context("createrawtransaction: non-string")?;
        // 2. select inputs + change; lock the inputs so nothing else spends them
        //    before we broadcast; our explicit funding feerate. NON-replaceable
        //    (no BIP125 signal): the v2 funding txid is committed into the
        //    pre-signed MuSig2 redeems, so it must never be RBF'd — the nurse
        //    CPFPs it instead, and the non-signal keeps external wallets from
        //    even offering a bump.
        let funded = self.rpc.call(
            "fundrawtransaction",
            &[
                json!(raw_hex),
                json!({ "lockUnspents": true, "fee_rate": fee_rate, "replaceable": false }),
            ],
        )?;
        let funded_hex = funded["hex"]
            .as_str()
            .context("fundrawtransaction: no hex")?;
        // 3. sign with the wallet — the txid is final once fully signed.
        let signed = self
            .rpc
            .call("signrawtransactionwithwallet", &[json!(funded_hex)])?;
        anyhow::ensure!(
            signed["complete"].as_bool() == Some(true),
            "funding tx did not sign to completion"
        );
        let signed_hex = signed["hex"]
            .as_str()
            .context("signrawtransactionwithwallet: no hex")?
            .to_string();
        // 4. decode locally to recover the txid and the vout paying `address` —
        //    fundrawtransaction inserts change at a random position, so match the
        //    output by scriptPubKey rather than assuming an index.
        let tx: Transaction = bitcoin::consensus::encode::deserialize(&hex::decode(&signed_hex)?)
            .context("decode built funding tx")?;
        let want_spk = self.params.parse_address(address)?;
        let vout = tx
            .output
            .iter()
            .position(|o| o.script_pubkey == want_spk)
            .context("built funding tx has no output paying the funding address")?
            as u32;
        Ok((tx.compute_txid().to_string(), vout, signed_hex))
    }

    fn wallet_cancel_funding(&self, tx_hex: &str) -> Result<()> {
        // Undo wallet_build_funding's `lockUnspents`: unlock exactly the built
        // tx's inputs. Per-input and error-tolerant — an input already unlocked
        // (node restarted; Core's locks are memory-only) must not fail the
        // cancel of the rest.
        let tx: Transaction = bitcoin::consensus::encode::deserialize(&hex::decode(tx_hex)?)
            .context("decode built funding tx for cancel")?;
        for input in &tx.input {
            let op = &input.previous_output;
            let _ = self.rpc.call(
                "lockunspent",
                &[
                    json!(true),
                    json!([{ "txid": op.txid.to_string(), "vout": op.vout }]),
                ],
            );
        }
        Ok(())
    }

    fn wallet_balance(&self) -> Result<u64> {
        let balance = self.rpc.call("getbalance", &[])?;
        let coins = balance.as_f64().context("getbalance: non-numeric")?;
        Ok((coins * 1e8).round() as u64)
    }

    fn wallet_locked(&self) -> Result<bool> {
        // `unlocked_until` is ABSENT on an unencrypted wallet, `0` when encrypted
        // and locked, and a future timestamp when encrypted and unlocked. Only a
        // hard `0` means "can read balance but cannot sign".
        let info = self.rpc.call("getwalletinfo", &[])?;
        Ok(info.get("unlocked_until").and_then(|v| v.as_u64()) == Some(0))
    }

    fn wallet_txs_since(&self, since_height: u64) -> Result<Option<Vec<(Transaction, u64)>>> {
        // One `listsinceblock` + one `gettransaction` per unique wallet tx —
        // callers bound `since_height` to the swap's era, and the follow
        // evaluator caches/throttles so this is a transition cost, not a
        // per-tick one (#171).
        let tip = self.tip_height()?;
        let hash = self
            .rpc
            .call("getblockhash", &[json!(since_height.min(tip))])?;
        let res = self.rpc.call("listsinceblock", &[hash])?;
        let mut seen = std::collections::BTreeMap::<String, u64>::new();
        for entry in res["transactions"].as_array().cloned().unwrap_or_default() {
            let Some(txid) = entry["txid"].as_str() else {
                continue;
            };
            let height = entry["blockheight"].as_u64().unwrap_or(0);
            seen.entry(txid.to_string()).or_insert(height);
        }
        // A wallet busier than this since the swap started is no longer a
        // swap-era scan — inconclusive beats silently truncated evidence.
        anyhow::ensure!(
            seen.len() <= 512,
            "wallet has {} transactions since height {since_height} — refusing an unbounded scan",
            seen.len()
        );
        let mut out = Vec::with_capacity(seen.len());
        for (txid, height) in seen {
            let tx = self.rpc.call("gettransaction", &[json!(txid)])?;
            let Some(hex_str) = tx["hex"].as_str() else {
                continue;
            };
            let Ok(raw) = hex::decode(hex_str) else {
                continue;
            };
            let Ok(decoded) = bitcoin::consensus::encode::deserialize::<Transaction>(&raw) else {
                continue;
            };
            out.push((decoded, height));
        }
        Ok(Some(out))
    }

    fn wallet_sign_send(
        &self,
        tx: &Transaction,
        prevout_value_sat: u64,
        prevout_spk: &ScriptBuf,
    ) -> Result<Txid> {
        let unsigned = bitcoin::consensus::encode::serialize_hex(tx);
        // The prevout is unconfirmed (the parent redeem in the mempool), so the
        // wallet needs its amount + spk explicitly to produce a segwit/taproot
        // signature. The wallet holds the key (the sweep address it issued).
        let prevout = &tx.input[0].previous_output;
        let amount = format!(
            "{}.{:08}",
            prevout_value_sat / 100_000_000,
            prevout_value_sat % 100_000_000
        );
        let prevtxs = json!([{
            "txid": prevout.txid.to_string(),
            "vout": prevout.vout,
            "scriptPubKey": hex::encode(prevout_spk.as_bytes()),
            "amount": amount,
        }]);
        let signed = self
            .rpc
            .call("signrawtransactionwithwallet", &[json!(unsigned), prevtxs])?;
        anyhow::ensure!(
            signed["complete"].as_bool() == Some(true),
            "wallet could not fully sign the CPFP child (is the redeem swept to a \
             wallet-owned address?): {signed}"
        );
        let signed_hex = signed["hex"]
            .as_str()
            .context("signrawtransactionwithwallet: no hex")?;
        match self.rpc.call("sendrawtransaction", &[json!(signed_hex)]) {
            Ok(txid) => Ok(Txid::from_str(
                txid.as_str().context("sendrawtransaction: non-string")?,
            )?),
            // An unchanged child re-sent each tick is already in the mempool.
            Err(e) if is_already_broadcast(&e) => Ok(tx.compute_txid()),
            Err(e) => Err(e),
        }
    }

    fn wallet_tx_fee_vsize(&self, txid: &str) -> Result<(u64, u64)> {
        // verbose `gettransaction` includes the `decoded` tx (for vsize) and the
        // wallet-computed `fee` (negative BTC for a send).
        let tx = self
            .rpc
            .call("gettransaction", &[json!(txid), json!(true), json!(true)])?;
        let fee_btc = tx["fee"]
            .as_f64()
            .context("gettransaction: no fee (not a wallet tx?)")?;
        let fee_sat = (fee_btc.abs() * 1e8).round() as u64;
        let vsize = tx["decoded"]["vsize"]
            .as_u64()
            .context("gettransaction: no decoded.vsize")?;
        Ok((fee_sat, vsize))
    }

    fn wallet_change_output(
        &self,
        funding_txid: &str,
        htlc_spk: &ScriptBuf,
    ) -> Result<Option<(u32, u64, ScriptBuf)>> {
        let tx = self.rpc.call(
            "gettransaction",
            &[json!(funding_txid), json!(true), json!(true)],
        )?;
        let htlc_hex = hex::encode(htlc_spk.as_bytes());
        let vouts = tx["decoded"]["vout"]
            .as_array()
            .context("gettransaction: no decoded.vout")?;
        for vout in vouts {
            let spk_hex = vout["scriptPubKey"]["hex"].as_str().unwrap_or_default();
            if spk_hex.is_empty() || spk_hex == htlc_hex {
                continue; // the HTLC output (not wallet-owned) — skip
            }
            // Positive ownership check: the HTLC output is a script the wallet does
            // not own, so `ismine` selects the change output unambiguously. Resolve
            // the address from `address` (Core ≥ 22) or `addresses[0]` (older Core),
            // so the check isn't silently defeated on an older node.
            let addr = vout["scriptPubKey"]["address"]
                .as_str()
                .or_else(|| vout["scriptPubKey"]["addresses"][0].as_str());
            let is_mine = addr
                .and_then(|addr| self.rpc.call("getaddressinfo", &[json!(addr)]).ok())
                .and_then(|info| info["ismine"].as_bool())
                .unwrap_or(false);
            if is_mine {
                let n = vout["n"].as_u64().context("vout without n")? as u32;
                let value_sat =
                    (vout["value"].as_f64().context("vout without value")? * 1e8).round() as u64;
                let spk = ScriptBuf::from_bytes(hex::decode(spk_hex)?);
                return Ok(Some((n, value_sat, spk)));
            }
        }
        Ok(None) // no wallet-owned change (exact-UTXO funding)
    }

    fn wallet_bumpfee(&self, txid: &str, feerate_sat_kvb: u64) -> Result<String> {
        // Core's `bumpfee` `fee_rate` option is sat/vB (Core ≥ 0.21) and accepts
        // 3 decimals — pass the exact sat/kvB rate as decimal sat/vB so a
        // fractional target (e.g. 2.004) clears the node's Rule-4 minimum
        // instead of being truncated to a whole "2" that lands below it.
        let fee_rate = feerate_sat_kvb as f64 / 1000.0;
        let res = self
            .rpc
            .call("bumpfee", &[json!(txid), json!({ "fee_rate": fee_rate })])?;
        Ok(res["txid"]
            .as_str()
            .context("bumpfee: no replacement txid")?
            .to_string())
    }
}

/// [`ChainBackend`] for the extracted Electrum transport
/// ([`electrum_btcx::ElectrumBackend`]) — a 1:1 delegation to its inherent
/// methods. Chain data only: it has no wallet, so it cannot be the primary
/// backend (funding and sweep addresses come from a Core-RPC wallet URL).
impl ChainBackend for ElectrumBackend {
    fn params(&self) -> &ChainParams {
        ElectrumBackend::params(self)
    }

    fn view_health(&self) -> Option<Arc<crate::server_health::ServerHealth>> {
        Some(self.health().clone())
    }

    fn verify_chain(&self) -> Result<()> {
        ElectrumBackend::verify_chain(self)
    }

    fn broadcast(&self, tx: &Transaction) -> Result<Txid> {
        ElectrumBackend::broadcast(self, tx)
    }

    fn get_txout(
        &self,
        outpoint: &OutPoint,
        expected_spk: &ScriptBuf,
    ) -> Result<Option<TxOutInfo>> {
        ElectrumBackend::get_txout(self, outpoint, expected_spk)
    }

    fn find_funding(&self, spk: &ScriptBuf) -> Result<Option<(OutPoint, TxOutInfo)>> {
        ElectrumBackend::find_funding(self, spk)
    }

    fn find_vout(&self, txid: &str, script_pubkey_hex: &str) -> Result<u32> {
        ElectrumBackend::find_vout(self, txid, script_pubkey_hex)
    }

    fn find_spend_witness(
        &self,
        outpoint: &OutPoint,
        watch_spk: &ScriptBuf,
        _from_height: u64,
    ) -> Result<Option<Vec<Vec<u8>>>> {
        // The scripthash history contains both the funding tx and any spend
        // of it — no block scanning, so the height hint is not needed.
        ElectrumBackend::find_spend_witness(self, outpoint, watch_spk)
    }

    fn spk_history(&self, spk: &ScriptBuf) -> Result<Option<Vec<(String, i64)>>> {
        // `blockchain.scripthash.get_history`: funding + spends, confirmed +
        // mempool — the full per-script record swap reconstruction needs.
        Ok(Some(ElectrumBackend::history(self, spk)?))
    }

    fn fetch_tx(&self, txid: &str) -> Result<Option<Transaction>> {
        // A fetch failure (unknown tx, transport hiccup) reads as "cannot
        // see it" — the caller treats `None` as inconclusive and retries,
        // exactly the conservative direction.
        Ok(ElectrumBackend::get_raw_tx(self, txid).ok())
    }

    fn tip_height(&self) -> Result<u64> {
        ElectrumBackend::tip_height(self)
    }

    fn tip_median_time(&self) -> Result<u64> {
        ElectrumBackend::tip_median_time(self)
    }

    fn tx_confirmations(&self, txid: &str, spk_hint: Option<&ScriptBuf>) -> Result<u64> {
        let spk = spk_hint.context(
            "Electrum backend can only locate transactions by script — spk hint required",
        )?;
        ElectrumBackend::tx_confirmations(self, txid, spk)
    }

    fn fee_rate_for(&self, conf_target: u16, _conservative: bool) -> Result<u64> {
        // Electrum's estimatefee takes only a block target (no economical/
        // conservative distinction), so `_conservative` is honored via the
        // tighter `conf_target` alone.
        ElectrumBackend::fee_rate_for(self, conf_target)
    }

    fn fee_estimate(&self, conf_target: u16) -> Result<Option<u64>> {
        ElectrumBackend::fee_estimate(self, conf_target)
    }

    fn fee_estimate_kvb(&self, conf_target: u16) -> Result<Option<u64>> {
        ElectrumBackend::fee_estimate_kvb(self, conf_target)
    }

    fn fee_rate_for_kvb(&self, conf_target: u16, _conservative: bool) -> Result<u64> {
        ElectrumBackend::fee_rate_for_kvb(self, conf_target)
    }

    fn resolve_send_fee(&self, fee: SendFee) -> Result<u64> {
        // Same math as the trait default — delegate so it lives once, in the
        // crate, next to the estimator it prices off.
        ElectrumBackend::resolve_send_fee(self, fee)
    }

    fn wallet_new_address(&self) -> Result<String> {
        anyhow::bail!(
            "the Electrum backend is chain-data only — the primary backend must be a \
             Core-RPC wallet URL (http://...)"
        )
    }

    fn wallet_send(&self, _address: &str, _amount_sat: u64, _fee: SendFee) -> Result<String> {
        anyhow::bail!(
            "the Electrum backend is chain-data only — the primary backend must be a \
             Core-RPC wallet URL (http://...)"
        )
    }

    fn wallet_balance(&self) -> Result<u64> {
        anyhow::bail!(
            "the Electrum backend is chain-data only — the primary backend must be a \
             Core-RPC wallet URL (http://...)"
        )
    }
}

/// Several independent backends for one chain — spec §10 requires ≥ 2
/// chain views during a live swap so a single lying/withholding server
/// cannot blind us. Mixed kinds are the intended production shape:
/// primary = own node over Core RPC (wallet), secondaries = Electrum
/// servers (independent views).
///
/// Semantics: wallet operations and own-tx lookups go to the *primary*
/// (first) backend. Chain reads fan out: spend-search takes the first
/// positive answer (withholding-resistant; witnesses are self-verifying),
/// funded-output verification demands agreement (substitution-resistant),
/// and clocks/fees take the most conservative value. Locktime correctness
/// is ultimately enforced by node consensus — MTP reads here only gate
/// our own behavior, so "most advanced clock" is the safe direction for
/// deadline refusals.
pub struct MultiBackend {
    backends: Vec<Box<dyn ChainBackend>>,
}

impl MultiBackend {
    /// `urls`: comma-separated; `http://…` → Core RPC, `tcp://…`/`ssl://…`
    /// → Electrum. The first is the primary (must be a wallet-qualified
    /// Core-RPC URL — it funds HTLCs and receives sweeps).
    pub fn new(params: &'static ChainParams, urls: &str) -> Result<Self> {
        let backends = urls
            .split(',')
            .map(str::trim)
            .filter(|u| !u.is_empty())
            .map(|url| -> Result<Box<dyn ChainBackend>> {
                if url.starts_with("http://") {
                    Ok(Box::new(CoreRpcBackend::new(params, url)?))
                } else if url.starts_with("tcp://") || url.starts_with("ssl://") {
                    Ok(Box::new(ElectrumBackend::new(params, url)?))
                } else {
                    anyhow::bail!(
                        "unsupported backend URL scheme in {url:?} (http:// | tcp:// | ssl://)"
                    )
                }
            })
            .collect::<Result<Vec<_>>>()?;
        anyhow::ensure!(!backends.is_empty(), "no RPC URLs given");
        Ok(Self { backends })
    }

    /// Assemble from prebuilt backends — the nodeless path builds its own
    /// primary (a `wallet_bdk::BdkWalletBackend`) before the remaining
    /// Electrum views join (docs/NODELESS_WALLET.md D5). `backends[0]` is
    /// the primary, exactly as with [`MultiBackend::new`].
    pub fn from_backends(backends: Vec<Box<dyn ChainBackend>>) -> Result<Self> {
        anyhow::ensure!(!backends.is_empty(), "no backends given");
        Ok(Self { backends })
    }

    fn primary(&self) -> &dyn ChainBackend {
        self.backends[0].as_ref()
    }

    /// Fan `op` over every backend not inside a health backoff window, in
    /// parallel (scoped threads — every backend owns its own socket, so
    /// this never puts two callers on one connection), collecting the
    /// responders' answers and the non-responders' errors. Absence
    /// semantics live here (issue #98): a skipped or erroring view is one
    /// fewer sample, never fatal by itself — the caller decides what
    /// quorum it needs via [`Self::require_responders`].
    fn fan_out<T: Send>(
        &self,
        op: impl Fn(&dyn ChainBackend) -> Result<T> + Sync,
    ) -> (Vec<T>, Vec<anyhow::Error>, usize) {
        // Skip views whose breaker is open — no thread, no connect timeout.
        let slots: Vec<Option<&dyn ChainBackend>> = self
            .backends
            .iter()
            .map(|b| {
                let skip = b.view_health().is_some_and(|h| !h.available());
                if skip {
                    None
                } else {
                    Some(b.as_ref())
                }
            })
            .collect();
        let skipped = slots.iter().filter(|s| s.is_none()).count();

        let results: Vec<Result<T>> = if slots.iter().flatten().count() <= 1 {
            // 0 or 1 live view: no thread ceremony (regtest single-server).
            slots.into_iter().flatten().map(&op).collect()
        } else {
            let op = &op;
            std::thread::scope(|s| {
                let handles: Vec<_> = slots
                    .into_iter()
                    .flatten()
                    .map(|b| s.spawn(move || op(b)))
                    .collect();
                handles
                    .into_iter()
                    .map(|h| {
                        h.join()
                            .unwrap_or_else(|_| Err(anyhow::anyhow!("chain view panicked")))
                    })
                    .collect()
            })
        };

        let mut hits = Vec::new();
        let mut errors = Vec::new();
        for r in results {
            match r {
                Ok(v) => hits.push(v),
                Err(e) => errors.push(e),
            }
        }
        (hits, errors, skipped)
    }

    /// Responder-quorum gate: at least `need` responders or a clear N-of-M
    /// error instead of a fabricated answer. Display aggregates need 1
    /// (one honest sample beats none); money-adjacent reads need
    /// [`Self::integrity_quorum`].
    fn require_responders<T>(
        &self,
        what: &str,
        need: usize,
        (hits, errors, skipped): (Vec<T>, Vec<anyhow::Error>, usize),
    ) -> Result<Vec<T>> {
        if hits.len() < need {
            let total = self.backends.len();
            let detail = errors
                .first()
                .map(|e| format!("; first error: {e:#}"))
                .unwrap_or_default();
            bail!(
                "{what}: {} of {total} chain view(s) answered, {need} needed \
                 ({skipped} in failure backoff, {} errored{detail})",
                hits.len(),
                errors.len()
            );
        }
        Ok(hits)
    }

    /// How many independent responders a MONEY-ADJACENT read needs (spec
    /// §10, #101): TWO on mainnet when the primary rides an untrusted
    /// public Electrum server (nodeless — it exposes a `view_health` cell)
    /// and a second view is even configured; ONE when the primary is the
    /// user's own Core node (a trusted sole view by definition) or on test
    /// networks / single-server setups. Governs the deadline clocks
    /// (`tip_median_time`, `tip_median_time_min`), the finality depth
    /// (`tx_confirmations_min`), and the positive side of `get_txout` —
    /// never plain display reads, which stay at 1.
    fn integrity_quorum(&self) -> usize {
        let untrusted_primary = self.backends[0].view_health().is_some();
        if untrusted_primary
            && self.backends.len() >= 2
            && self.primary().params().network == Network::Mainnet
        {
            2
        } else {
            1
        }
    }

    /// The wallet backend's OWN chain view must answer. The quorum reads
    /// above deliberately mask a dead server behind its healthy siblings —
    /// correct for display, but the swap-initiation gate must not be
    /// fooled: the primary funds HTLCs and receives sweeps, and until
    /// re-election lands (#99) it has exactly one server. Called by
    /// [`crate::engine::Engine::ensure_chains_live`].
    pub fn wallet_view_live(&self) -> Result<u64> {
        self.primary().tip_height()
    }

    /// FINALITY depth of our own spend: the MINIMUM confirmations over
    /// `integrity_quorum` responding views (#101). The trait method takes
    /// the max — right for display and for widening scan bounds, but a
    /// single lying view inflating the max would stop the fee-bump nurse
    /// and mark a swap Completed while the spend is still unconfirmed.
    /// Min over a quorum is the safe direction: a laggy view only keeps
    /// the nurse working a little longer.
    pub fn tx_confirmations_min(&self, txid: &str, spk_hint: Option<&ScriptBuf>) -> Result<u64> {
        let hits = self.require_responders(
            "tx finality",
            self.integrity_quorum(),
            self.fan_out(|b| b.tx_confirmations(txid, spk_hint)),
        )?;
        Ok(hits.into_iter().min().expect("nonempty by quorum"))
    }

    /// The *least*-advanced MTP across responding views — the conservative
    /// clock for deciding our own CLTV refund is spendable. The trait
    /// [`tip_median_time`] takes the max (refuse deadline-sensitive actions
    /// earliest, the safe direction for "stop acting in time"); for refund
    /// *readiness* the safe direction is the opposite: only believe a
    /// refund is final once even the laggiest responding view's MTP has
    /// reached the locktime, so the broadcast can't hit `non-final` on the
    /// node that will actually mine it. Single-backend setups collapse to
    /// the same value.
    ///
    /// [`tip_median_time`]: ChainBackend::tip_median_time
    pub fn tip_median_time_min(&self) -> Result<u64> {
        let hits = self.require_responders(
            "tip mtp",
            self.integrity_quorum(),
            self.fan_out(|b| b.tip_median_time()),
        )?;
        Ok(hits.into_iter().min().expect("nonempty by quorum"))
    }
}

impl ChainBackend for MultiBackend {
    fn params(&self) -> &ChainParams {
        self.primary().params()
    }

    fn verify_chain(&self) -> Result<()> {
        // Quorum health check (issue #98): the coin is live while ≥1 view
        // serves the RIGHT chain. A view that answers with the wrong
        // genesis / pruned history is disagreement — fail hard however
        // many healthy siblings it has; a view that doesn't answer is
        // absent — skip it.
        let (hits, errors, skipped) = self.fan_out(|b| b.verify_chain());
        for err in errors.iter() {
            if err.downcast_ref::<ChainMismatch>().is_some() {
                bail!("{err:#}");
            }
        }
        self.require_responders("verify chain", 1, (hits, errors, skipped))?;
        Ok(())
    }

    fn broadcast(&self, tx: &Transaction) -> Result<Txid> {
        // Best-effort to all live views in parallel; success if any
        // accepts (self-verifying: more relays = stronger withholding
        // resistance, and a rejecting minority can't veto).
        let (hits, errors, skipped) = self.fan_out(|b| b.broadcast(tx));
        match hits.into_iter().next() {
            Some(txid) => Ok(txid),
            None => match errors.into_iter().next() {
                Some(err) => Err(err),
                None => bail!(
                    "broadcast: all {} chain view(s) are in failure backoff ({skipped} skipped)",
                    self.backends.len()
                ),
            },
        }
    }

    fn get_txout(
        &self,
        outpoint: &OutPoint,
        expected_spk: &ScriptBuf,
    ) -> Result<Option<TxOutInfo>> {
        // THE money-agreement read (#101): responders must agree on the
        // output's script and value — any disagreement halts, never
        // majority-rules (an attacker running 3 of 6 servers must not win
        // a Sybil vote). Absence is skippable like everywhere else, BUT a
        // POSITIVE (the funding exists) is only trusted with
        // `integrity_quorum` agreeing responders: one public server alone
        // must never talk us into treating an output as real. Any
        // responding view of "spent/missing" stays a conservative veto,
        // and confirmations take the minimum over the agreeing views.
        let hits = self.require_responders(
            "verify txout",
            1,
            self.fan_out(|b| b.get_txout(outpoint, expected_spk)),
        )?;
        if hits.iter().any(|h| h.is_none()) {
            return Ok(None); // any view of "spent/missing" wins (conservative)
        }
        let mut agreed: Option<TxOutInfo> = None;
        let mut positives = 0usize;
        for info in hits.into_iter().flatten() {
            positives += 1;
            match &mut agreed {
                None => agreed = Some(info),
                Some(existing) => {
                    if existing.script_pubkey_hex != info.script_pubkey_hex
                        || existing.value_sat != info.value_sat
                    {
                        bail!(
                            "chain backends disagree about {outpoint} — refusing to proceed (spec §10)"
                        );
                    }
                    existing.confirmations = existing.confirmations.min(info.confirmations);
                }
            }
        }
        let need = self.integrity_quorum();
        anyhow::ensure!(
            positives >= need,
            "only {positives} chain view(s) confirm {outpoint} — need {need} independent \
             views before trusting a funding (spec §10); check the coin's Electrum servers"
        );
        Ok(agreed)
    }

    fn find_funding(&self, spk: &ScriptBuf) -> Result<Option<(OutPoint, TxOutInfo)>> {
        // Discovery only — any view that sees a paying output wins. The
        // caller re-verifies the located outpoint via `get_txout` (which
        // demands backend agreement), so one lying server can't substitute
        // a funding. Zero responders is an ERROR, not a "not funded yet" —
        // an outage must not read as an answer (issue #98).
        let hits =
            self.require_responders("find funding", 1, self.fan_out(|b| b.find_funding(spk)))?;
        Ok(hits.into_iter().flatten().next())
    }

    fn find_vout(&self, txid: &str, script_pubkey_hex: &str) -> Result<u32> {
        self.primary().find_vout(txid, script_pubkey_hex)
    }

    fn find_spend_witness(
        &self,
        outpoint: &OutPoint,
        watch_spk: &ScriptBuf,
        from_height: u64,
    ) -> Result<Option<Vec<Vec<u8>>>> {
        // Withholding-resistant: any positive answer wins. The witness
        // is self-verifying (preimage hashes to H), so a lying server
        // cannot fabricate one. Zero responders errors (see find_funding).
        let hits = self.require_responders(
            "find spend witness",
            1,
            self.fan_out(|b| b.find_spend_witness(outpoint, watch_spk, from_height)),
        )?;
        Ok(hits.into_iter().flatten().next())
    }

    fn spk_history(&self, spk: &ScriptBuf) -> Result<Option<Vec<(String, i64)>>> {
        // Discovery — the first CAPABLE view's history wins (entries are
        // hints; the caller verifies every referenced tx against local
        // bytes). `Ok(None)` from a view means "no script index" (Core),
        // which must not mask a capable-but-erroring Electrum view: with
        // zero positive answers and any error, report the error so an
        // outage reads as an outage, not as tier-L (issue #98 discipline).
        let (hits, errors, skipped) = self.fan_out(|b| b.spk_history(spk));
        let mut saw_responder = false;
        for h in hits {
            saw_responder = true;
            if h.is_some() {
                return Ok(h);
            }
        }
        if let Some(err) = errors.into_iter().next() {
            return Err(err.context("script history"));
        }
        if !saw_responder && skipped > 0 {
            bail!(
                "script history: all {} chain view(s) are in failure backoff",
                self.backends.len()
            );
        }
        Ok(None)
    }

    fn fetch_tx(&self, txid: &str) -> Result<Option<Transaction>> {
        // Any view's positive answer wins, hash-verified — a lying server
        // cannot substitute a different tx for the requested txid.
        let want = Txid::from_str(txid)?;
        let (hits, errors, _) = self.fan_out(|b| b.fetch_tx(txid));
        for tx in hits.into_iter().flatten() {
            if tx.compute_txid() == want {
                return Ok(Some(tx));
            }
        }
        if let Some(err) = errors.into_iter().next() {
            return Err(err.context("fetch tx"));
        }
        Ok(None)
    }

    fn wallet_txs_since(&self, since_height: u64) -> Result<Option<Vec<(Transaction, u64)>>> {
        // Wallet reads go to the primary, like every wallet operation.
        self.primary().wallet_txs_since(since_height)
    }

    fn tip_height(&self) -> Result<u64> {
        let hits = self.require_responders("tip height", 1, self.fan_out(|b| b.tip_height()))?;
        Ok(hits.into_iter().max().expect("nonempty by quorum"))
    }

    fn tip_median_time(&self) -> Result<u64> {
        // Most advanced responding clock: refuses deadline-sensitive
        // actions earliest.
        let hits = self.require_responders(
            "tip mtp",
            self.integrity_quorum(),
            self.fan_out(|b| b.tip_median_time()),
        )?;
        Ok(hits.into_iter().max().expect("nonempty by quorum"))
    }

    fn tx_confirmations(&self, txid: &str, spk_hint: Option<&ScriptBuf>) -> Result<u64> {
        let hits = self.require_responders(
            "tx confirmations",
            1,
            self.fan_out(|b| b.tx_confirmations(txid, spk_hint)),
        )?;
        Ok(hits.into_iter().max().expect("nonempty by quorum"))
    }

    fn fee_rate_for(&self, conf_target: u16, conservative: bool) -> Result<u64> {
        let hits = self.require_responders(
            "fee rate",
            1,
            self.fan_out(|b| b.fee_rate_for(conf_target, conservative)),
        )?;
        Ok(hits.into_iter().max().expect("nonempty by quorum").max(1))
    }

    fn fee_rate_for_kvb(&self, conf_target: u16, conservative: bool) -> Result<u64> {
        // Precise sat/kvB counterpart of `fee_rate_for` — the bump nurses read
        // THIS, so keep the fan-out at native resolution (don't fall through to
        // the integer-rounded default). Most conservative responding view wins.
        let hits = self.require_responders(
            "fee rate (kvB)",
            1,
            self.fan_out(|b| b.fee_rate_for_kvb(conf_target, conservative)),
        )?;
        Ok(hits
            .into_iter()
            .max()
            .expect("nonempty by quorum")
            .max(1000))
    }

    fn fee_estimate(&self, conf_target: u16) -> Result<Option<u64>> {
        // Most conservative responding view wins, like fee_rate_for; "no
        // estimate" only when no RESPONDER has one (the send form's
        // fallback) — an unreachable fleet is an error, not "no estimate".
        let hits = self.require_responders(
            "fee estimate",
            1,
            self.fan_out(|b| b.fee_estimate(conf_target)),
        )?;
        Ok(hits.into_iter().flatten().max())
    }

    fn is_in_mempool(&self, txid: &str) -> Result<bool> {
        // Authoritative on the primary — the wallet node we broadcast through
        // and must keep the tx anchored in. Chain-only watchers don't hold our
        // mempool, so polling them would mask a real eviction on our own node.
        self.primary().is_in_mempool(txid)
    }

    fn incremental_relay_feerate(&self) -> Result<u64> {
        // The replacement is broadcast to all backends, but the primary is the
        // node enforcing RBF acceptance for our wallet; its floor governs.
        self.primary().incremental_relay_feerate()
    }

    fn incremental_relay_feerate_kvb(&self) -> Result<u64> {
        self.primary().incremental_relay_feerate_kvb()
    }

    fn wallet_new_address(&self) -> Result<String> {
        self.primary().wallet_new_address()
    }

    fn wallet_send(&self, address: &str, amount_sat: u64, fee: SendFee) -> Result<String> {
        self.primary().wallet_send(address, amount_sat, fee)
    }

    fn wallet_send_all(&self, address: &str, fee: SendFee) -> Result<String> {
        self.primary().wallet_send_all(address, fee)
    }

    fn wallet_build_funding(
        &self,
        address: &str,
        amount_sat: u64,
    ) -> Result<(String, u32, String)> {
        // Wallet op: the primary (Core) backend owns the funding UTXOs.
        self.primary().wallet_build_funding(address, amount_sat)
    }

    fn wallet_cancel_funding(&self, tx_hex: &str) -> Result<()> {
        self.primary().wallet_cancel_funding(tx_hex)
    }

    fn wallet_transactions(&self) -> Result<Vec<WalletTxInfo>> {
        self.primary().wallet_transactions()
    }

    fn wallet_balance(&self) -> Result<u64> {
        self.primary().wallet_balance()
    }

    fn wallet_locked(&self) -> Result<bool> {
        self.primary().wallet_locked()
    }

    fn wallet_sign_send(
        &self,
        tx: &Transaction,
        prevout_value_sat: u64,
        prevout_spk: &ScriptBuf,
    ) -> Result<Txid> {
        // Wallet op: the primary (Core) backend owns the sweep key.
        self.primary()
            .wallet_sign_send(tx, prevout_value_sat, prevout_spk)
    }

    fn wallet_tx_fee_vsize(&self, txid: &str) -> Result<(u64, u64)> {
        self.primary().wallet_tx_fee_vsize(txid)
    }

    fn wallet_change_output(
        &self,
        funding_txid: &str,
        htlc_spk: &ScriptBuf,
    ) -> Result<Option<(u32, u64, ScriptBuf)>> {
        self.primary().wallet_change_output(funding_txid, htlc_spk)
    }

    fn wallet_bumpfee(&self, txid: &str, feerate_sat_kvb: u64) -> Result<String> {
        self.primary().wallet_bumpfee(txid, feerate_sat_kvb)
    }
}

#[cfg(test)]
mod multi_backend_tests {
    use super::*;
    use crate::params::Network;
    use crate::registry;
    use crate::server_health::server_health;
    use std::sync::atomic::AtomicU64 as TestCounter;

    fn btc_params() -> &'static ChainParams {
        registry::get("btc")
            .expect("built-in btc")
            .params(Network::Mainnet)
            .expect("btc mainnet params")
    }

    #[test]
    fn default_cookie_candidates_follow_bitcoind_layout() {
        // #162: the no-credentials fallback must look where the coin's node
        // actually writes its .cookie. CI always has the platform base env
        // (LOCALAPPDATA/APPDATA or HOME), so candidates are non-empty.
        let btcx = registry::get("btcx")
            .unwrap()
            .params(Network::Regtest)
            .unwrap();
        let candidates = default_cookie_candidates(btcx);
        assert!(!candidates.is_empty(), "platform base env missing?");
        for c in &candidates {
            let s = c.display().to_string().replace('\\', "/");
            // Hyphenated registry display name ("Bitcoin PoCX" → Bitcoin-PoCX;
            // lowercased dotdir on Linux) + the network subdir + .cookie.
            assert!(
                s.contains("Bitcoin-PoCX") || s.contains(".bitcoin-pocx"),
                "{s}"
            );
            assert!(s.ends_with("regtest/.cookie"), "{s}");
        }

        // Mainnet lives at the data-dir root; testnet probes the per-family
        // dir names (Bitcoin testnet3 / Litecoin testnet4 / PoCX testnet).
        let btc_main = default_cookie_candidates(btc_params());
        assert!(btc_main
            .iter()
            .all(|c| !c.display().to_string().contains("testnet")
                && !c.display().to_string().contains("regtest")));
        let btc_test = default_cookie_candidates(
            registry::get("btc")
                .unwrap()
                .params(Network::Testnet)
                .unwrap(),
        );
        let joined = btc_test
            .iter()
            .map(|c| c.display().to_string().replace('\\', "/"))
            .collect::<Vec<_>>()
            .join("\n");
        for sub in ["testnet/.cookie", "testnet3/.cookie", "testnet4/.cookie"] {
            assert!(joined.contains(sub), "missing {sub} in {joined}");
        }
    }

    /// A scriptable chain view: answers with `tip`, or errors ("absent"),
    /// or answers with the WRONG chain (`ChainMismatch` — disagreement).
    /// Counts calls so tests can prove a Down view was skipped entirely.
    struct TestView {
        tip: u64,
        fail: bool,
        mismatch: bool,
        calls: TestCounter,
        health: Option<Arc<crate::server_health::ServerHealth>>,
        txout: Option<TxOutInfo>,
    }

    impl TestView {
        fn ok(tip: u64) -> Self {
            Self {
                tip,
                fail: false,
                mismatch: false,
                calls: TestCounter::new(0),
                health: None,
                txout: None,
            }
        }
        /// This view answers `get_txout` positively with `value_sat` at
        /// `confirmations` (script "aa").
        fn sees_txout(mut self, value_sat: u64, confirmations: u64) -> Self {
            self.txout = Some(TxOutInfo {
                value_sat,
                script_pubkey_hex: "aa".into(),
                confirmations,
            });
            self
        }
        /// Mark this view as riding an untrusted public Electrum server
        /// (a health cell exists) — flips the primary-slot trust rule.
        fn untrusted(mut self, key: &str) -> Self {
            self.health = Some(server_health(key, "tcp://test:1"));
            self
        }
        fn absent() -> Self {
            Self {
                fail: true,
                ..Self::ok(0)
            }
        }
        fn wrong_chain(tip: u64) -> Self {
            Self {
                mismatch: true,
                ..Self::ok(tip)
            }
        }
        fn touch(&self) -> Result<()> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.mismatch {
                return Err(anyhow::Error::new(ChainMismatch(
                    "test server serves the wrong chain".into(),
                )));
            }
            if self.fail {
                bail!("io: connection refused (test)");
            }
            Ok(())
        }
    }

    impl ChainBackend for TestView {
        fn params(&self) -> &ChainParams {
            btc_params()
        }
        fn view_health(&self) -> Option<Arc<crate::server_health::ServerHealth>> {
            self.health.clone()
        }
        fn verify_chain(&self) -> Result<()> {
            self.touch()
        }
        fn broadcast(&self, tx: &Transaction) -> Result<Txid> {
            self.touch()?;
            Ok(tx.compute_txid())
        }
        fn get_txout(&self, _o: &OutPoint, _s: &ScriptBuf) -> Result<Option<TxOutInfo>> {
            self.touch()?;
            Ok(self.txout.clone())
        }
        fn find_funding(&self, _spk: &ScriptBuf) -> Result<Option<(OutPoint, TxOutInfo)>> {
            self.touch()?;
            Ok(None)
        }
        fn find_vout(&self, _txid: &str, _spk: &str) -> Result<u32> {
            bail!("unused in these tests")
        }
        fn find_spend_witness(
            &self,
            _o: &OutPoint,
            _w: &ScriptBuf,
            _h: u64,
        ) -> Result<Option<Vec<Vec<u8>>>> {
            self.touch()?;
            Ok(None)
        }
        fn tip_height(&self) -> Result<u64> {
            self.touch()?;
            Ok(self.tip)
        }
        fn tip_median_time(&self) -> Result<u64> {
            self.touch()?;
            Ok(self.tip * 100)
        }
        fn tx_confirmations(&self, _txid: &str, _spk: Option<&ScriptBuf>) -> Result<u64> {
            self.touch()?;
            Ok(self.tip)
        }
        fn fee_rate_for(&self, _t: u16, _c: bool) -> Result<u64> {
            self.touch()?;
            Ok(self.tip.max(1))
        }
        fn wallet_new_address(&self) -> Result<String> {
            bail!("no wallet")
        }
        fn wallet_balance(&self) -> Result<u64> {
            bail!("no wallet")
        }
        fn wallet_send(&self, _a: &str, _v: u64, _f: SendFee) -> Result<String> {
            bail!("no wallet")
        }
    }

    fn multi(views: Vec<TestView>) -> MultiBackend {
        MultiBackend::from_backends(
            views
                .into_iter()
                .map(|v| Box::new(v) as Box<dyn ChainBackend>)
                .collect(),
        )
        .unwrap()
    }

    #[test]
    fn aggregates_tolerate_absent_views() {
        // The issue-#98 incident shape: one healthy view, one dead — the
        // coin must stay fully readable off the healthy one.
        let mb = multi(vec![TestView::ok(10), TestView::absent()]);
        assert!(mb.verify_chain().is_ok());
        assert_eq!(mb.tip_height().unwrap(), 10);
        assert_eq!(mb.tip_median_time().unwrap(), 1000);
        assert_eq!(mb.tip_median_time_min().unwrap(), 1000);
        assert_eq!(mb.tx_confirmations("txid", None).unwrap(), 10);
        assert_eq!(mb.fee_rate_for(6, false).unwrap(), 10);
        assert!(mb.find_funding(&ScriptBuf::new()).unwrap().is_none());
    }

    #[test]
    fn aggregates_take_the_conservative_value_over_responders() {
        let mb = multi(vec![TestView::ok(10), TestView::ok(12), TestView::absent()]);
        assert_eq!(mb.tip_height().unwrap(), 12, "max over responders");
        assert_eq!(
            mb.tip_median_time_min().unwrap(),
            1000,
            "min over responders"
        );
    }

    #[test]
    fn below_quorum_is_a_clear_error_not_an_answer() {
        let mb = multi(vec![TestView::absent(), TestView::absent()]);
        let err = format!("{:#}", mb.tip_height().unwrap_err());
        assert!(
            err.contains("0 of 2 chain view(s) answered"),
            "want the N-of-M message, got: {err}"
        );
        // Discovery reads error too — an outage must not read as "not
        // funded yet" / "no spend visible".
        assert!(mb.find_funding(&ScriptBuf::new()).is_err());
        assert!(mb
            .find_spend_witness(&OutPoint::null(), &ScriptBuf::new(), 0)
            .is_err());
    }

    #[test]
    fn wrong_chain_fails_hard_despite_healthy_siblings() {
        // Disagreement is never tolerated (§10): a wrong-genesis answer
        // fails the coin even with a healthy majority — absence and
        // disagreement must never be conflated.
        let mb = multi(vec![
            TestView::ok(10),
            TestView::ok(11),
            TestView::wrong_chain(12),
        ]);
        let err = format!("{:#}", mb.verify_chain().unwrap_err());
        assert!(err.contains("wrong chain"), "got: {err}");
    }

    #[test]
    fn down_view_is_skipped_without_a_single_call() {
        // A view inside its backoff window is never even asked — its
        // WOULD-BE answer (tip 99, higher than the healthy view's) must
        // not appear in the aggregate, and no connect timeout is paid.
        // The Down state is manufactured through the public surface: a
        // dead-server dial (refused instantly on loopback) records the
        // connect failure into the shared health cell — the recorder
        // itself is private to electrum-btcx. Unique URL — the health
        // cells are process-global, keyed by (coin, url).
        let dead = ElectrumBackend::new(btc_params(), "tcp://127.0.0.1:1").unwrap();
        assert!(dead.tip_height().is_err(), "the dead dial must fail");
        let health = dead.health().clone();
        assert!(!health.available(), "failed dial opens the backoff window");
        let mut skipped = TestView::ok(99);
        skipped.health = Some(health);
        let mb = multi(vec![TestView::ok(10), skipped]);
        assert_eq!(
            mb.tip_height().unwrap(),
            10,
            "the Down view's tip 99 leaking in means it was consulted"
        );
        assert!(mb.verify_chain().is_ok());
    }

    #[test]
    fn txout_positive_needs_two_agreeing_views_when_primary_is_untrusted() {
        // Nodeless mainnet shape: the primary rides a public Electrum
        // server. TWO agreeing positives → trusted, min confirmations.
        let op = OutPoint::null();
        let spk = ScriptBuf::new();
        let mb = multi(vec![
            TestView::ok(10).sees_txout(5000, 7).untrusted("test-q2-a"),
            TestView::ok(10).sees_txout(5000, 3),
        ]);
        let info = mb.get_txout(&op, &spk).unwrap().expect("agreed positive");
        assert_eq!(info.value_sat, 5000);
        assert_eq!(info.confirmations, 3, "min over agreeing views");

        // The second view drops out: ONE public server alone must not
        // talk us into treating the funding as real.
        let mb = multi(vec![
            TestView::ok(10).sees_txout(5000, 7).untrusted("test-q2-b"),
            TestView::absent(),
        ]);
        let err = format!("{:#}", mb.get_txout(&op, &spk).unwrap_err());
        assert!(err.contains("need 2 independent views"), "got: {err}");
    }

    #[test]
    fn txout_trusted_core_primary_stands_alone() {
        // Node-backed shape: the primary is the user's own Core node (no
        // health cell) — a trusted sole view; a dead public sibling must
        // not block verification.
        let mb = multi(vec![
            TestView::ok(10).sees_txout(5000, 7),
            TestView::absent(),
        ]);
        let info = mb
            .get_txout(&OutPoint::null(), &ScriptBuf::new())
            .unwrap()
            .expect("own node suffices");
        assert_eq!(info.confirmations, 7);
    }

    #[test]
    fn txout_disagreement_halts_and_none_vetoes() {
        let op = OutPoint::null();
        let spk = ScriptBuf::new();
        // Value disagreement between responders: halt, never majority.
        let mb = multi(vec![
            TestView::ok(10).sees_txout(5000, 7),
            TestView::ok(10).sees_txout(4999, 7),
            TestView::ok(10).sees_txout(4999, 7),
        ]);
        let err = format!("{:#}", mb.get_txout(&op, &spk).unwrap_err());
        assert!(err.contains("disagree"), "got: {err}");
        // Any responding "spent/missing" stays a conservative veto.
        let mb = multi(vec![
            TestView::ok(10).sees_txout(5000, 7),
            TestView::ok(10), // sees nothing
        ]);
        assert!(mb.get_txout(&op, &spk).unwrap().is_none());
        // All views absent: an outage is an error, not an answer.
        let mb = multi(vec![TestView::absent(), TestView::absent()]);
        assert!(mb.get_txout(&op, &spk).is_err());
    }

    #[test]
    fn finality_takes_min_over_quorum_not_display_max() {
        // One view inflates confirmations (lying or glitching): the
        // display read (max) shows it, the FINALITY read (min) does not —
        // the fee-bump nurse keeps working.
        let mb = multi(vec![TestView::ok(1), TestView::ok(99)]);
        assert_eq!(
            mb.tx_confirmations("txid", None).unwrap(),
            99,
            "display max"
        );
        assert_eq!(
            mb.tx_confirmations_min("txid", None).unwrap(),
            1,
            "finality min"
        );
    }

    #[test]
    fn deadline_clocks_need_quorum_when_primary_is_untrusted() {
        // Nodeless mainnet with the sibling view dead: a SOLE public clock
        // must not gate deadline decisions (spec §10) — while plain
        // display reads (tip height) still work off one responder.
        let mb = multi(vec![
            TestView::ok(10).untrusted("test-clock-q"),
            TestView::absent(),
        ]);
        assert_eq!(mb.tip_height().unwrap(), 10, "display read: quorum 1");
        assert!(mb.tip_median_time().is_err(), "clock read: quorum 2");
        assert!(mb.tip_median_time_min().is_err(), "refund clock: quorum 2");
        // Both views live: clocks work again.
        let mb = multi(vec![
            TestView::ok(10).untrusted("test-clock-q"),
            TestView::ok(12),
        ]);
        assert_eq!(mb.tip_median_time().unwrap(), 1200);
        assert_eq!(mb.tip_median_time_min().unwrap(), 1000);
    }

    #[test]
    fn broadcast_succeeds_on_any_accepting_view() {
        let mb = multi(vec![TestView::absent(), TestView::ok(1)]);
        let tx = Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: bitcoin::absolute::LockTime::ZERO,
            input: vec![],
            output: vec![],
        };
        assert!(mb.broadcast(&tx).is_ok());
        // All views absent: the error surfaces instead of a silent drop.
        let mb = multi(vec![TestView::absent(), TestView::absent()]);
        assert!(mb.broadcast(&tx).is_err());
    }
}

#[cfg(test)]
mod funding_conf_target_tests {
    use super::funding_conf_target_for;

    #[test]
    fn derives_per_coin_target_from_30min_cap() {
        // Bitcoin: 10-min blocks → 6 blocks would be an hour, so cap at 3 (30 min).
        assert_eq!(funding_conf_target_for(600), 3);
        // Litecoin: 2.5-min blocks → 1800/150 = 12, clamped back to the standard 6
        // (6 LTC blocks ≈ 15 min is already inside the budget).
        assert_eq!(funding_conf_target_for(150), 6);
        // BTCX: 2-min blocks → 1800/120 = 15, clamped to 6.
        assert_eq!(funding_conf_target_for(120), 6);
        // A slow chain gets pulled tighter: 20-min blocks → 1.
        assert_eq!(funding_conf_target_for(1200), 1);
        // Never below 1: 60-min blocks → 1800/3600 = 0, floored to 1.
        assert_eq!(funding_conf_target_for(3600), 1);
        // A nonsense 0 spacing can't occur (coins require it), but the guard must
        // not divide by zero — it falls back to the standard baseline (6).
        assert_eq!(funding_conf_target_for(0), 6);
    }
}
