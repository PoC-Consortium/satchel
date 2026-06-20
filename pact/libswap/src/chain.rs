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

use crate::params::ChainParams;
use crate::rpc::{RpcClient, RpcError};

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

/// What `gettxout` tells us about an unspent output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxOutInfo {
    pub value_sat: u64,
    pub script_pubkey_hex: String,
    pub confirmations: u64,
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

    /// Feerate in sat/vB from the node's estimator, with a conservative
    /// fallback when the estimator has no data (fresh chains, regtest).
    fn fee_rate_sat_per_vb(&self) -> Result<u64>;

    /// Fresh receive address from the user's core wallet (sweep target).
    fn wallet_new_address(&self) -> Result<String>;

    /// Confirmed core-wallet balance in base units.
    fn wallet_balance(&self) -> Result<u64>;

    /// Fund `address` with exactly `amount_sat` via the core wallet
    /// (HTLC funding is a normal send, spec §6.1).
    fn wallet_send(&self, address: &str, amount_sat: u64) -> Result<String>;

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
        _watch_spk: &ScriptBuf,
        from_height: u64,
    ) -> Result<Option<Vec<Vec<u8>>>> {
        // Mempool first: the spend may not be mined yet.
        let mempool = self.rpc.call("getrawmempool", &[])?;
        for txid in mempool.as_array().cloned().unwrap_or_default() {
            let tx = self
                .rpc
                .call("getrawtransaction", &[txid.clone(), json!(true)])?;
            for vin in tx["vin"].as_array().cloned().unwrap_or_default() {
                if Self::vin_matches(&vin, outpoint) {
                    return Ok(Some(Self::witness_of(&vin)?));
                }
            }
        }
        // Then blocks from the HTLC's funding height to the tip.
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

    fn fee_rate_sat_per_vb(&self) -> Result<u64> {
        const FALLBACK_SAT_PER_VB: u64 = 10;
        const MAX_SAT_PER_VB: u64 = 500; // sanity cap against estimator glitches
        let rate = self
            .rpc
            .call("estimatesmartfee", &[json!(6)])
            .ok()
            .and_then(|r| r["feerate"].as_f64()) // BTC per kvB
            .map(|btc_kvb| ((btc_kvb * 1e8) / 1000.0).ceil() as u64)
            .unwrap_or(FALLBACK_SAT_PER_VB);
        Ok(rate.clamp(1, MAX_SAT_PER_VB))
    }

    fn wallet_new_address(&self) -> Result<String> {
        Ok(self
            .rpc
            .call("getnewaddress", &[])?
            .as_str()
            .context("getnewaddress: non-string")?
            .to_string())
    }

    fn wallet_send(&self, address: &str, amount_sat: u64) -> Result<String> {
        // Amount as a decimal string: exact, no float in our code path.
        let amount = format!(
            "{}.{:08}",
            amount_sat / 100_000_000,
            amount_sat % 100_000_000
        );
        let txid = self
            .rpc
            .call("sendtoaddress", &[json!(address), json!(amount)])?;
        Ok(txid
            .as_str()
            .context("sendtoaddress: non-string")?
            .to_string())
    }

    fn wallet_balance(&self) -> Result<u64> {
        let balance = self.rpc.call("getbalance", &[])?;
        let coins = balance.as_f64().context("getbalance: non-numeric")?;
        Ok((coins * 1e8).round() as u64)
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
}

/// Chain-data backend speaking the Electrum protocol — the same client
/// for BTC (any public Electrum server) and PoCX (`electrs-pocx`, which
/// serves Electrum RPC alongside the Esplora REST API used by the
/// explorer).
///
/// Chain data only: it has no wallet, so it cannot be the primary backend
/// (funding and sweep addresses come from a Core-RPC wallet URL).
///
/// PoCX caveat baked in: PoCX block headers are 286 bytes with extra
/// consensus fields and a generator signature that is *excluded* from the
/// block hash, so all header handling goes through
/// [`ChainParams::header_hash`]/[`ChainParams::header_time`] on raw bytes
/// — never through `electrum-client`'s Bitcoin-typed header API.
pub struct ElectrumBackend {
    params: &'static ChainParams,
    client: electrum_client::Client,
}

impl ElectrumBackend {
    /// `url`: `tcp://host:port` or `ssl://host:port`.
    pub fn new(params: &'static ChainParams, url: &str) -> Result<Self> {
        let client = electrum_client::Client::new(url)
            .with_context(|| format!("connecting to Electrum server {url}"))?;
        Ok(Self { params, client })
    }

    fn raw(&self, method: &str, params: Vec<electrum_client::Param>) -> Result<Value> {
        use electrum_client::ElectrumApi;
        self.client
            .raw_call(method, params)
            .with_context(|| format!("electrum {method}"))
    }

    /// Electrum addresses outputs by the SHA256 of the scriptPubKey,
    /// reversed (display order).
    fn scripthash(spk: &ScriptBuf) -> String {
        use bitcoin::hashes::{sha256, Hash};
        let mut digest = sha256::Hash::hash(spk.as_bytes()).to_byte_array();
        digest.reverse();
        hex::encode(digest)
    }

    /// (height, raw tip header) from headers.subscribe.
    fn tip(&self) -> Result<(u64, Vec<u8>)> {
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

    fn get_raw_tx(&self, txid: &str) -> Result<Transaction> {
        let hex_tx = self.raw(
            "blockchain.transaction.get",
            vec![electrum_client::Param::String(txid.into())],
        )?;
        let bytes = hex::decode(hex_tx.as_str().context("transaction.get: non-string")?)?;
        bitcoin::consensus::encode::deserialize(&bytes).context("transaction.get: bad tx")
    }

    fn history(&self, spk: &ScriptBuf) -> Result<Vec<(String, i64)>> {
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

    fn fee_rate_sat_per_vb(&self) -> Result<u64> {
        const FALLBACK_SAT_PER_VB: u64 = 10;
        const MAX_SAT_PER_VB: u64 = 500;
        let rate = self
            .raw(
                "blockchain.estimatefee",
                vec![electrum_client::Param::Usize(6)],
            )
            .ok()
            .and_then(|v| v.as_f64())
            .filter(|btc_kb| *btc_kb > 0.0) // -1 = no estimate available
            .map(|btc_kb| ((btc_kb * 1e8) / 1000.0).ceil() as u64)
            .unwrap_or(FALLBACK_SAT_PER_VB);
        Ok(rate.clamp(1, MAX_SAT_PER_VB))
    }

    fn wallet_new_address(&self) -> Result<String> {
        anyhow::bail!(
            "the Electrum backend is chain-data only — the primary backend must be a \
             Core-RPC wallet URL (http://...)"
        )
    }

    fn wallet_send(&self, _address: &str, _amount_sat: u64) -> Result<String> {
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

    fn fee_rate_sat_per_vb(&self) -> Result<u64> {
        let mut best = 1;
        for backend in &self.backends {
            best = best.max(backend.fee_rate_sat_per_vb()?);
        }
        Ok(best)
    }

    fn wallet_new_address(&self) -> Result<String> {
        self.primary().wallet_new_address()
    }

    fn wallet_send(&self, address: &str, amount_sat: u64) -> Result<String> {
        self.primary().wallet_send(address, amount_sat)
    }

    fn wallet_balance(&self) -> Result<u64> {
        self.primary().wallet_balance()
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
}
