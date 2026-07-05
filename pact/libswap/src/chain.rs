//! Chain backends — spec §10.
//!
//! All backend data is an untrusted hint: scripts and amounts are verified
//! against locally reconstructed bytes, and refund scheduling is purely
//! clock-driven. A lying backend can withhold or delay, never steal.
//!
//! v1 implements the Core-RPC backend (the user's own pocx node /
//! bitcoind; wallet-qualified URL = the user's core wallet on that node).
//! The Electrum backend is Phase 1.1 — same trait, both chains.

use anyhow::{bail, Context, Result};
use bitcoin::{OutPoint, ScriptBuf, Transaction, Txid};
use serde_json::{json, Value};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::params::{ChainParams, Network};
use crate::rpc::{RpcClient, RpcError};

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

/// Overflow/glitch guard on user-supplied and estimator feerates — NOT the
/// fee ceiling (the real caps live in `FeeBumpPolicy`). Shared by the send
/// fee resolution and the per-backend estimators.
pub(crate) const SANITY_MAX_SAT_PER_VB: u64 = 10_000;

/// Estimator answer (BTC per kvB, Core `estimatesmartfee` and Electrum
/// `blockchain.estimatefee` alike) → integer sat/vB, ROUNDED to nearest
/// (phoenix parity — its send form shows `round(feerate·1e8/100)/10`).
/// `ceil` here silently DOUBLED every fee at the bottom of the market:
/// a 1.01 sat/vB estimate became 2 on both the send presets and the whole
/// trading path (funding, redeem, refund, nurse market term). Rounding down
/// by a fraction is safe everywhere this feeds — every trading tx class has
/// an escalation path (v1 funding RBF, v2 funding/redeem CPFP, refund
/// rebuild, user-send bump), and callers floor the result at 1 /
/// `min_feerate_sat_vb`. The one conversion that must NEVER round down —
/// the BIP125 incremental-relay increment — keeps its own `ceil`.
fn btc_kvb_to_sat_vb(btc_kvb: f64) -> u64 {
    ((btc_kvb * 1e8) / 1000.0).round() as u64
}

/// Electrum socket bounds: TCP connect and per-request read/write. Generous —
/// a single request is one JSON line each way — but FINITE: a stalled remote
/// server must error out instead of hanging an engine call (and with it every
/// queued RPC).
const ELECTRUM_CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
const ELECTRUM_IO_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// How a wallet send prices itself: a market estimate at a block target
/// (funding and the Slow/Normal/Fast presets) or an explicit user-chosen
/// rate (the send form's Custom field, and the phoenix-style fallback when
/// the estimator has no data).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendFee {
    /// Market estimate at this conf target, with the 1 sat/vB fallback.
    Target(u16),
    /// Explicit rate in sat/vB, clamped to the coin floor / sanity max.
    RateSatVb(u64),
}

/// The send form's fee preview (`estimatesendfee` RPC): raw estimator answers
/// for the three phoenix-parity presets, `None` where the estimator has no
/// data, plus the coin's feerate floor (the custom field's minimum/default).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SendFeeEstimates {
    pub min_sat_per_vb: u64,
    /// 1-block target.
    pub fast: Option<u64>,
    /// 6-block target — the preselected preset.
    pub normal: Option<u64>,
    /// 144-block target.
    pub slow: Option<u64>,
}

/// What `gettxout` tells us about an unspent output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxOutInfo {
    pub value_sat: u64,
    pub script_pubkey_hex: String,
    pub confirmations: u64,
}

/// One entry of the nodeless wallet's activity feed (`listtransactions`,
/// design doc §4): direction + net amount from the wallet's point of view.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct WalletTxInfo {
    pub txid: String,
    /// `"sent"` or `"received"` — the wallet's NET direction. A self-transfer
    /// nets to a pure fee payment and reads as `"sent"` with `amount_sat` 0.
    pub direction: String,
    /// Net value moved, excluding the fee on sends: what the recipient got
    /// (sent) or what landed in our keychains (received).
    pub amount_sat: u64,
    /// `None` when the wallet doesn't own every input (a receive — the fee was
    /// paid by the sender and isn't ours to report).
    pub fee_sat: Option<u64>,
    /// Virtual size in vB — with `fee_sat` this yields the effective feerate
    /// an RBF bump has to beat.
    pub vsize: u64,
    pub confirmations: u64,
    /// Block time for confirmed txs, first-seen time for mempool ones. `None`
    /// only for a built-but-unbroadcast funding awaiting its two-phase release.
    pub timestamp: Option<u64>,
}

pub trait ChainBackend {
    fn params(&self) -> &ChainParams;

    /// Verify the backend serves the expected chain (genesis hash check,
    /// spec §3.3). MUST be called before any funding decision.
    fn verify_chain(&self) -> Result<()>;

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

    /// Resolve a [`SendFee`] to the sat/vB rate a send prices itself at:
    /// market estimate (with fallback) for a target, or the explicit rate
    /// clamped to the coin floor and the sanity max.
    fn resolve_send_fee(&self, fee: SendFee) -> Result<u64> {
        match fee {
            SendFee::Target(conf_target) => self.fee_rate_for(conf_target, false),
            SendFee::RateSatVb(rate) => Ok(rate
                .clamp(1, SANITY_MAX_SAT_PER_VB)
                .max(self.params().min_feerate_sat_vb)),
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
    /// (sat/vB); returns the replacement txid. The v1 funding nurse: the funding is
    /// wallet-owned and broadcast BIP125-replaceable. Errors if not replaceable or
    /// the wallet can't afford the higher fee. Wallet-backed Core primary only.
    fn wallet_bumpfee(&self, _txid: &str, _feerate_sat_vb: u64) -> Result<String> {
        bail!("this backend has no wallet; cannot bumpfee")
    }
}

/// Bitcoin Core / pocx-node JSON-RPC backend.
pub struct CoreRpcBackend {
    params: &'static ChainParams,
    rpc: RpcClient,
}

impl CoreRpcBackend {
    pub fn new(params: &'static ChainParams, url: &str) -> Result<Self> {
        Ok(Self {
            params,
            rpc: RpcClient::from_url(url)?,
        })
    }

    /// Raw `estimatesmartfee` answer in sat/vB, or `None` when the node has no
    /// estimate (fresh/quiet chain). Shared by `fee_rate_for` (which adds the
    /// 1 sat/vB fallback) and `fee_estimate` (which surfaces the `None`).
    fn smart_fee_estimate(&self, conf_target: u16, conservative: bool) -> Option<u64> {
        // Regtest-only test override: the harness injects a market feerate to
        // create a market-vs-broadcast gap the bump nurse reacts to (see
        // `set_test_feerate`). Never honored off regtest.
        if self.params.network == Network::Regtest {
            let ov = TEST_FEERATE_OVERRIDE_SAT_VB.load(Ordering::Relaxed);
            if ov > 0 {
                return Some(
                    ov.clamp(1, SANITY_MAX_SAT_PER_VB)
                        .max(self.params.min_feerate_sat_vb),
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
            .map(btc_kvb_to_sat_vb)
            .map(|est| {
                est.clamp(1, SANITY_MAX_SAT_PER_VB)
                    .max(self.params.min_feerate_sat_vb)
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
            bail!(
                "backend serves the wrong chain: genesis {genesis}, expected {} ({} {:?})",
                self.params.genesis_hash,
                self.params.coin_id,
                self.params.network
            );
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
            .smart_fee_estimate(conf_target, conservative)
            .unwrap_or(self.params.min_feerate_sat_vb.max(1)))
    }

    fn fee_estimate(&self, conf_target: u16) -> Result<Option<u64>> {
        Ok(self.smart_fee_estimate(conf_target, false))
    }

    fn is_in_mempool(&self, txid: &str) -> Result<bool> {
        // getmempoolentry succeeds iff the tx is in the mempool right now.
        Ok(self.rpc.call("getmempoolentry", &[json!(txid)]).is_ok())
    }

    fn incremental_relay_feerate(&self) -> Result<u64> {
        let rate = self
            .rpc
            .call("getmempoolinfo", &[])
            .ok()
            .and_then(|r| r["incrementalrelayfee"].as_f64()) // BTC per kvB
            .map(|btc_kvb| ((btc_kvb * 1e8) / 1000.0).ceil() as u64)
            .unwrap_or(1);
        Ok(rate.max(1))
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
        let fee_rate = self.resolve_send_fee(fee)?;
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
        let fee_rate = self.resolve_send_fee(fee)?;
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
        // not a blind 6-block target.
        let fee_rate = self.fee_rate_for(self.funding_conf_target(), false)?;
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

    fn wallet_bumpfee(&self, txid: &str, feerate_sat_vb: u64) -> Result<String> {
        // Core's `bumpfee` `fee_rate` option is sat/vB (Core ≥ 0.21).
        let res = self.rpc.call(
            "bumpfee",
            &[json!(txid), json!({ "fee_rate": feerate_sat_vb })],
        )?;
        Ok(res["txid"]
            .as_str()
            .context("bumpfee: no replacement txid")?
            .to_string())
    }
}

/// Chain-data backend speaking the Electrum protocol — the same client
/// for BTC (any public Electrum server) and PoCX (`electrs-pocx`, the
/// dedicated Electrum server; the explorer's indexer
/// `esplora-electrs-pocx` also serves Electrum RPC).
///
/// Chain data only: it has no wallet, so it cannot be the primary backend
/// (funding and sweep addresses come from a Core-RPC wallet URL).
///
/// PoCX caveat baked in: PoCX block headers are 286 bytes with extra
/// consensus fields and a generator signature that is *excluded* from the
/// block hash, so all header handling goes through
/// [`ChainParams::header_hash`]/[`ChainParams::header_time`] on raw bytes
/// — never through `electrum-client`'s Bitcoin-typed header API.
/// One live Electrum connection. `tcp://` is the crate's plaintext client;
/// `ssl://` is OUR rustls setup (see [`connect_electrum_ssl`]) — the crate's
/// own no-validation mode sends an EMPTY `signature_algorithms` extension
/// (its verifier returns no schemes), which strict servers answer with a
/// fatal `DecodeError` alert or a hangup. Backends are rebuilt per engine
/// call, so the plain `RawClient` (no auto-reconnect) is the right shape.
enum ElectrumConn {
    Tcp(
        electrum_client::raw_client::RawClient<
            electrum_client::raw_client::ElectrumPlaintextStream,
        >,
    ),
    Ssl(electrum_client::raw_client::RawClient<electrum_client::raw_client::ElectrumSslStream>),
}

impl ElectrumConn {
    fn raw_call(
        &self,
        method: &str,
        params: Vec<electrum_client::Param>,
    ) -> std::result::Result<Value, electrum_client::Error> {
        use electrum_client::ElectrumApi;
        match self {
            Self::Tcp(c) => c.raw_call(method, params),
            Self::Ssl(c) => c.raw_call(method, params),
        }
    }
}

/// Accept-any-certificate verifier that still advertises the provider's REAL
/// signature schemes (a correct ClientHello). Electrum-ecosystem convention:
/// most public servers are self-signed — often X.509 v1, which strict rustls
/// verification rejects outright ("UnsupportedCertVersion"). TLS is transport
/// privacy here, NOT server authentication — that job belongs to the genesis
/// check, the server.version capability handshake, and the ≥2-server rule on
/// mainnet, which cross-check the chain data itself.
#[derive(Debug)]
struct AcceptAnyServerCert {
    schemes: Vec<rustls::SignatureScheme>,
}

impl rustls::client::danger::ServerCertVerifier for AcceptAnyServerCert {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer,
        _intermediates: &[rustls::pki_types::CertificateDer],
        _server_name: &rustls::pki_types::ServerName,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.schemes.clone()
    }
}

/// Connect `host:port` over TLS with SNI, a full signature-scheme list, and
/// the accept-any verifier above, and hand the stream to the crate's
/// `RawClient` (its `From<StreamOwned<…>>` impl).
fn connect_electrum_ssl(
    addr: &str,
) -> Result<electrum_client::raw_client::RawClient<electrum_client::raw_client::ElectrumSslStream>>
{
    let (host, _port) = addr
        .rsplit_once(':')
        .with_context(|| format!("Electrum ssl URL needs host:port, got {addr:?}"))?;
    // Bounded connect + per-request I/O: a stalled REMOTE server must FAIL,
    // not hang — pactd serializes all RPCs on one lock, so an unbounded read
    // here froze the whole app (rc10 field report).
    let sock = std::net::ToSocketAddrs::to_socket_addrs(addr)
        .with_context(|| format!("resolving Electrum server {addr}"))?
        .next()
        .with_context(|| format!("Electrum server {addr} resolved to no address"))?;
    let tcp = std::net::TcpStream::connect_timeout(&sock, ELECTRUM_CONNECT_TIMEOUT)
        .with_context(|| format!("connecting to Electrum server {addr}"))?;
    tcp.set_read_timeout(Some(ELECTRUM_IO_TIMEOUT))
        .context("electrum read timeout")?;
    tcp.set_write_timeout(Some(ELECTRUM_IO_TIMEOUT))
        .context("electrum write timeout")?;
    // pactd installs aws-lc-rs as the process default in main() (the wss
    // fix); standalone users of libswap (tests, examples) fall back here.
    let provider = rustls::crypto::CryptoProvider::get_default()
        .cloned()
        .unwrap_or_else(|| std::sync::Arc::new(rustls::crypto::aws_lc_rs::default_provider()));
    let schemes = provider
        .signature_verification_algorithms
        .supported_schemes();
    let config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .context("rustls protocol versions")?
        .dangerous()
        .with_custom_certificate_verifier(std::sync::Arc::new(AcceptAnyServerCert { schemes }))
        .with_no_client_auth();
    let name = rustls::pki_types::ServerName::try_from(host.to_string())
        .with_context(|| format!("invalid Electrum server name {host:?}"))?;
    let conn = rustls::ClientConnection::new(std::sync::Arc::new(config), name)
        .context("rustls client connection")?;
    Ok(rustls::StreamOwned::new(conn, tcp).into())
}

pub struct ElectrumBackend {
    params: &'static ChainParams,
    client: ElectrumConn,
}

impl ElectrumBackend {
    /// `url`: `tcp://host:port` or `ssl://host:port`.
    pub fn new(params: &'static ChainParams, url: &str) -> Result<Self> {
        let client = if let Some(addr) = url.strip_prefix("ssl://") {
            ElectrumConn::Ssl(connect_electrum_ssl(addr)?)
        } else {
            let addr = url.strip_prefix("tcp://").unwrap_or(url);
            ElectrumConn::Tcp(
                electrum_client::raw_client::RawClient::new(addr, Some(ELECTRUM_IO_TIMEOUT))
                    .with_context(|| format!("connecting to Electrum server {url}"))?,
            )
        };
        Ok(Self { params, client })
    }

    fn raw(&self, method: &str, params: Vec<electrum_client::Param>) -> Result<Value> {
        self.client
            .raw_call(method, params)
            .with_context(|| format!("electrum {method}"))
    }

    /// Electrum addresses outputs by the SHA256 of the scriptPubKey,
    /// reversed (display order).
    pub(crate) fn scripthash(spk: &ScriptBuf) -> String {
        use bitcoin::hashes::{sha256, Hash};
        let mut digest = sha256::Hash::hash(spk.as_bytes()).to_byte_array();
        digest.reverse();
        hex::encode(digest)
    }

    /// (height, raw tip header) from headers.subscribe.
    pub(crate) fn tip(&self) -> Result<(u64, Vec<u8>)> {
        let tip = self.raw("blockchain.headers.subscribe", vec![])?;
        let height = tip["height"]
            .as_u64()
            .context("headers.subscribe: no height")?;
        let raw = hex::decode(tip["hex"].as_str().context("headers.subscribe: no hex")?)?;
        Ok((height, raw))
    }

    fn confirmations(&self, entry_height: i64, tip_height: u64) -> u64 {
        if entry_height > 0 {
            tip_height.saturating_sub(entry_height as u64) + 1
        } else {
            0 // mempool (0) or mempool-with-unconfirmed-parents (-1)
        }
    }

    pub(crate) fn get_raw_tx(&self, txid: &str) -> Result<Transaction> {
        let hex_tx = self.raw(
            "blockchain.transaction.get",
            vec![electrum_client::Param::String(txid.into())],
        )?;
        let bytes = hex::decode(hex_tx.as_str().context("transaction.get: non-string")?)?;
        bitcoin::consensus::encode::deserialize(&bytes).context("transaction.get: bad tx")
    }

    /// (block hash hex, header timestamp) at `height` — raw header bytes
    /// hashed via [`ChainParams::header_hash`] (PoCX 286-byte headers safe).
    /// The nodeless wallet's chain source uses this for bdk anchors and
    /// checkpoints.
    pub(crate) fn header_at(&self, height: u64) -> Result<(String, u32)> {
        let raw = self.raw(
            "blockchain.block.header",
            vec![electrum_client::Param::Usize(height as usize)],
        )?;
        let raw = hex::decode(raw.as_str().context("block.header: non-string")?)?;
        Ok((
            self.params.header_hash(&raw)?,
            self.params.header_time(&raw)?,
        ))
    }

    pub(crate) fn history(&self, spk: &ScriptBuf) -> Result<Vec<(String, i64)>> {
        let entries = self.raw(
            "blockchain.scripthash.get_history",
            vec![electrum_client::Param::String(Self::scripthash(spk))],
        )?;
        Ok(entries
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|e| {
                Some((
                    e["tx_hash"].as_str()?.to_string(),
                    e["height"].as_i64().unwrap_or(0),
                ))
            })
            .collect())
    }
}

impl ChainBackend for ElectrumBackend {
    fn params(&self) -> &ChainParams {
        self.params
    }

    fn verify_chain(&self) -> Result<()> {
        // Capability handshake first. Everything we call afterwards is
        // MANDATORY protocol-1.4 surface (scripthash history/listunspent,
        // headers, transaction get/broadcast, estimatefee), so there is no
        // per-method probing — the three real risks are an old protocol, a
        // PRUNED server (a restored seed's full scan would silently miss
        // history), and the wrong chain. `server.version` also matters for
        // politeness: some public servers drop clients that skip negotiation.
        let ver = self.raw(
            "server.version",
            vec![
                electrum_client::Param::String("satchel".into()),
                electrum_client::Param::String("1.4".into()),
            ],
        )?;
        let proto = ver
            .as_array()
            .and_then(|a| a.get(1))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        anyhow::ensure!(
            proto.parse::<f32>().map(|p| p >= 1.4).unwrap_or(false),
            "Electrum server negotiated protocol {proto:?} — need 1.4+ \
             (server: {})",
            ver.as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
                .unwrap_or("?")
        );
        // features: strict where advertised, lenient where absent.
        if let Ok(features) = self.raw("server.features", vec![]) {
            if let Some(genesis) = features["genesis_hash"].as_str() {
                anyhow::ensure!(
                    genesis == self.params.genesis_hash,
                    "Electrum server advertises the wrong chain: genesis \
                     {genesis}, expected {} ({} {:?})",
                    self.params.genesis_hash,
                    self.params.coin_id,
                    self.params.network
                );
            }
            let pruning = &features["pruning"];
            anyhow::ensure!(
                pruning.is_null() || pruning.as_u64() == Some(0),
                "Electrum server is PRUNED (keeps {} blocks) — a pruned server \
                 cannot serve full wallet history; use an unpruned one",
                pruning
            );
        }
        // Deep genesis check: fetch header 0 and hash it OURSELVES — validates
        // both the chain and our (PoCX-aware) header parsing on this server.
        let raw = self.raw(
            "blockchain.block.header",
            vec![electrum_client::Param::Usize(0)],
        )?;
        let raw = hex::decode(raw.as_str().context("block.header: non-string")?)?;
        let genesis = self.params.header_hash(&raw)?;
        anyhow::ensure!(
            genesis == self.params.genesis_hash,
            "Electrum server serves the wrong chain: genesis {genesis}, expected {} ({} {:?})",
            self.params.genesis_hash,
            self.params.coin_id,
            self.params.network
        );
        Ok(())
    }

    fn broadcast(&self, tx: &Transaction) -> Result<Txid> {
        let hex_tx = bitcoin::consensus::encode::serialize_hex(tx);
        match self.raw(
            "blockchain.transaction.broadcast",
            vec![electrum_client::Param::String(hex_tx)],
        ) {
            Ok(txid) => Ok(Txid::from_str(
                txid.as_str().context("broadcast: non-string")?,
            )?),
            // Already mined / in the mempool: a no-op success, not an error.
            Err(e) if is_already_broadcast(&e) => Ok(tx.compute_txid()),
            Err(e) => Err(e),
        }
    }

    fn get_txout(
        &self,
        outpoint: &OutPoint,
        expected_spk: &ScriptBuf,
    ) -> Result<Option<TxOutInfo>> {
        let utxos = self.raw(
            "blockchain.scripthash.listunspent",
            vec![electrum_client::Param::String(Self::scripthash(
                expected_spk,
            ))],
        )?;
        let (tip_height, _) = self.tip()?;
        for utxo in utxos.as_array().cloned().unwrap_or_default() {
            if utxo["tx_hash"].as_str() == Some(outpoint.txid.to_string().as_str())
                && utxo["tx_pos"].as_u64() == Some(u64::from(outpoint.vout))
            {
                return Ok(Some(TxOutInfo {
                    value_sat: utxo["value"].as_u64().context("listunspent: no value")?,
                    // Queried *by* script, so the binding is structural.
                    script_pubkey_hex: hex::encode(expected_spk.as_bytes()),
                    confirmations: self
                        .confirmations(utxo["height"].as_i64().unwrap_or(0), tip_height),
                }));
            }
        }
        Ok(None)
    }

    fn find_funding(&self, spk: &ScriptBuf) -> Result<Option<(OutPoint, TxOutInfo)>> {
        let utxos = self.raw(
            "blockchain.scripthash.listunspent",
            vec![electrum_client::Param::String(Self::scripthash(spk))],
        )?;
        let (tip_height, _) = self.tip()?;
        let Some(utxo) = utxos
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .next()
        else {
            return Ok(None);
        };
        let txid = Txid::from_str(
            utxo["tx_hash"]
                .as_str()
                .context("listunspent: no tx_hash")?,
        )?;
        let vout = utxo["tx_pos"].as_u64().context("listunspent: no tx_pos")? as u32;
        Ok(Some((
            OutPoint { txid, vout },
            TxOutInfo {
                value_sat: utxo["value"].as_u64().context("listunspent: no value")?,
                script_pubkey_hex: hex::encode(spk.as_bytes()),
                confirmations: self.confirmations(utxo["height"].as_i64().unwrap_or(0), tip_height),
            },
        )))
    }

    fn find_vout(&self, txid: &str, script_pubkey_hex: &str) -> Result<u32> {
        let tx = self.get_raw_tx(txid)?;
        let wanted = hex::decode(script_pubkey_hex)?;
        tx.output
            .iter()
            .position(|out| out.script_pubkey.as_bytes() == wanted.as_slice())
            .map(|pos| pos as u32)
            .with_context(|| format!("transaction {txid} has no output paying the expected script"))
    }

    fn find_spend_witness(
        &self,
        outpoint: &OutPoint,
        watch_spk: &ScriptBuf,
        _from_height: u64,
    ) -> Result<Option<Vec<Vec<u8>>>> {
        // The HTLC scripthash history contains both the funding tx and any
        // spend of it — no block scanning needed.
        for (tx_hash, _height) in self.history(watch_spk)? {
            if tx_hash == outpoint.txid.to_string() {
                continue; // the funding tx itself
            }
            let tx = self.get_raw_tx(&tx_hash)?;
            for input in &tx.input {
                if input.previous_output == *outpoint {
                    return Ok(Some(
                        input.witness.iter().map(|item| item.to_vec()).collect(),
                    ));
                }
            }
        }
        Ok(None)
    }

    fn tip_height(&self) -> Result<u64> {
        Ok(self.tip()?.0)
    }

    fn tip_median_time(&self) -> Result<u64> {
        // Median of the last (up to) 11 header timestamps, like
        // CBlockIndex::GetMedianTimePast.
        let (tip_height, _) = self.tip()?;
        let span = tip_height.min(10);
        let start = tip_height - span;
        let headers = self.raw(
            "blockchain.block.headers",
            vec![
                electrum_client::Param::Usize(start as usize),
                electrum_client::Param::Usize((span + 1) as usize),
            ],
        )?;
        let raw = hex::decode(headers["hex"].as_str().context("block.headers: no hex")?)?;
        let header_len = self.params.header_len();
        anyhow::ensure!(
            raw.len() % header_len == 0 && !raw.is_empty(),
            "block.headers returned {} bytes, not a multiple of {header_len}",
            raw.len()
        );
        let mut times: Vec<u64> = raw
            .chunks(header_len)
            .map(|hdr| self.params.header_time(hdr).map(u64::from))
            .collect::<Result<_>>()?;
        times.sort_unstable();
        Ok(times[times.len() / 2])
    }

    fn tx_confirmations(&self, txid: &str, spk_hint: Option<&ScriptBuf>) -> Result<u64> {
        let spk = spk_hint.context(
            "Electrum backend can only locate transactions by script — spk hint required",
        )?;
        let (tip_height, _) = self.tip()?;
        for (tx_hash, height) in self.history(spk)? {
            if tx_hash == txid {
                return Ok(self.confirmations(height, tip_height));
            }
        }
        Ok(0)
    }

    fn fee_rate_for(&self, conf_target: u16, _conservative: bool) -> Result<u64> {
        // No estimate (empty/low-traffic mempool, or the node can't estimate) →
        // the fee market is effectively empty, so the relay minimum suffices.
        // The bump nurse covers the rare case where this later under-prices.
        // Electrum's estimatefee takes only a block target (no economical/
        // conservative distinction), so `_conservative` is honored via the
        // tighter `conf_target` alone.
        Ok(self
            .fee_estimate(conf_target)?
            .unwrap_or(self.params.min_feerate_sat_vb.max(1)))
    }

    fn fee_estimate(&self, conf_target: u16) -> Result<Option<u64>> {
        Ok(self
            .raw(
                "blockchain.estimatefee",
                vec![electrum_client::Param::Usize(conf_target as usize)],
            )
            .ok()
            .and_then(|v| v.as_f64())
            .filter(|btc_kb| *btc_kb > 0.0) // -1 = no estimate available
            .map(btc_kvb_to_sat_vb)
            .map(|est| {
                est.clamp(1, SANITY_MAX_SAT_PER_VB)
                    .max(self.params.min_feerate_sat_vb)
            }))
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

    /// The *least*-advanced MTP across backends — the conservative clock for
    /// deciding our own CLTV refund is spendable. The trait [`tip_median_time`]
    /// takes the max (refuse deadline-sensitive actions earliest, the safe
    /// direction for "stop acting in time"); for refund *readiness* the safe
    /// direction is the opposite: only believe a refund is final once even the
    /// laggiest backend's MTP has reached the locktime, so the broadcast can't
    /// hit `non-final` on the node that will actually mine it. Single-backend
    /// setups collapse to the same value.
    ///
    /// [`tip_median_time`]: ChainBackend::tip_median_time
    pub fn tip_median_time_min(&self) -> Result<u64> {
        let mut min: Option<u64> = None;
        for backend in &self.backends {
            let mtp = backend.tip_median_time()?;
            min = Some(min.map_or(mtp, |m: u64| m.min(mtp)));
        }
        min.context("no backends")
    }
}

impl ChainBackend for MultiBackend {
    fn params(&self) -> &ChainParams {
        self.primary().params()
    }

    fn verify_chain(&self) -> Result<()> {
        for backend in &self.backends {
            backend.verify_chain()?;
        }
        Ok(())
    }

    fn broadcast(&self, tx: &Transaction) -> Result<Txid> {
        // Best-effort to all; success if any accepts.
        let mut last_err = None;
        let mut accepted = None;
        for backend in &self.backends {
            match backend.broadcast(tx) {
                Ok(txid) => accepted = Some(txid),
                Err(err) => last_err = Some(err),
            }
        }
        accepted.ok_or_else(|| last_err.expect("at least one backend"))
    }

    fn get_txout(
        &self,
        outpoint: &OutPoint,
        expected_spk: &ScriptBuf,
    ) -> Result<Option<TxOutInfo>> {
        // Verification read: all responding backends must agree on the
        // output's script and value; confirmations take the minimum.
        let mut agreed: Option<TxOutInfo> = None;
        for backend in &self.backends {
            match backend.get_txout(outpoint, expected_spk)? {
                None => return Ok(None), // any view of "spent/missing" wins (conservative)
                Some(info) => match &mut agreed {
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
                },
            }
        }
        Ok(agreed)
    }

    fn find_funding(&self, spk: &ScriptBuf) -> Result<Option<(OutPoint, TxOutInfo)>> {
        // Discovery only — first backend that sees a paying output wins. The
        // caller re-verifies the located outpoint via `get_txout` (which demands
        // backend agreement), so one lying server can't substitute a funding.
        let mut last_err = None;
        for backend in &self.backends {
            match backend.find_funding(spk) {
                Ok(Some(found)) => return Ok(Some(found)),
                Ok(None) => {}
                Err(err) => last_err = Some(err),
            }
        }
        match last_err {
            Some(err) if self.backends.len() == 1 => Err(err),
            _ => Ok(None),
        }
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
        // Withholding-resistant: first positive answer wins. The witness
        // is self-verifying (preimage hashes to H), so a lying server
        // cannot fabricate one.
        let mut last_err = None;
        for backend in &self.backends {
            match backend.find_spend_witness(outpoint, watch_spk, from_height) {
                Ok(Some(witness)) => return Ok(Some(witness)),
                Ok(None) => {}
                Err(err) => last_err = Some(err),
            }
        }
        match last_err {
            Some(err) if self.backends.len() == 1 => Err(err),
            _ => Ok(None),
        }
    }

    fn tip_height(&self) -> Result<u64> {
        let mut best = 0;
        for backend in &self.backends {
            best = best.max(backend.tip_height()?);
        }
        Ok(best)
    }

    fn tip_median_time(&self) -> Result<u64> {
        // Most advanced clock: refuses deadline-sensitive actions earliest.
        let mut best = 0;
        for backend in &self.backends {
            best = best.max(backend.tip_median_time()?);
        }
        Ok(best)
    }

    fn tx_confirmations(&self, txid: &str, spk_hint: Option<&ScriptBuf>) -> Result<u64> {
        let mut best = 0;
        for backend in &self.backends {
            best = best.max(backend.tx_confirmations(txid, spk_hint)?);
        }
        Ok(best)
    }

    fn fee_rate_for(&self, conf_target: u16, conservative: bool) -> Result<u64> {
        let mut best = 1;
        for backend in &self.backends {
            best = best.max(backend.fee_rate_for(conf_target, conservative)?);
        }
        Ok(best)
    }

    fn fee_estimate(&self, conf_target: u16) -> Result<Option<u64>> {
        // Most conservative live view wins, like fee_rate_for; "no estimate"
        // only when NO backend has one (so the send form's fallback kicks in).
        let mut best = None;
        for backend in &self.backends {
            best = best.max(backend.fee_estimate(conf_target)?);
        }
        Ok(best)
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

    fn wallet_bumpfee(&self, txid: &str, feerate_sat_vb: u64) -> Result<String> {
        self.primary().wallet_bumpfee(txid, feerate_sat_vb)
    }
}

#[cfg(test)]
mod funding_conf_target_tests {
    use super::{btc_kvb_to_sat_vb, funding_conf_target_for};

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

    #[test]
    fn estimator_conversion_rounds_like_phoenix() {
        // 0.00001012 BTC/kvB = 1.012 sat/vB — the real bottom-of-market shape
        // that `ceil` used to double to 2 (rc10 field report).
        assert_eq!(btc_kvb_to_sat_vb(0.00001012), 1);
        assert_eq!(btc_kvb_to_sat_vb(0.00001000), 1);
        // ≥ .5 rounds up, matching phoenix's Math.round.
        assert_eq!(btc_kvb_to_sat_vb(0.00001500), 2);
        assert_eq!(btc_kvb_to_sat_vb(0.00009873), 10);
        // Sub-relay dust rounds to 0 — callers clamp to ≥ 1 / the coin floor.
        assert_eq!(btc_kvb_to_sat_vb(0.00000040), 0);
    }
}
