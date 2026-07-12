//! Swap-leg state reconstruction from chain ground truth
//! (docs/STATE_RECONSTRUCTION.md).
//!
//! Every classification here is a PURE, idempotent function of the chain:
//! it answers "what happened to this leg" identically whether the swap
//! completed a minute or a month ago, so a follower/rescuer that missed
//! events (dormant observer, restart, DB restore) converges on the next
//! evaluation — there is no history-catchup vs live-monitor mode switch to
//! desync. Live triggers (ticks, subscriptions) only decide WHEN to
//! re-evaluate, never what is true.
//!
//! The classifier needs a script-history-capable backend ([`ChainBackend::
//! spk_history`], Electrum) — the live-UTXO reads (`find_funding`/
//! `get_txout`) structurally cannot see an output that is already spent,
//! which is every completed swap. History-less backends (Core RPC, tier L)
//! return `Ok(None)` and callers degrade to live reads + the timelock
//! age-out.
//!
//! All backend data is untrusted (spec §10): funding outputs are matched
//! byte-for-byte against the locally derived scriptPubKey AND the agreed
//! amount; spend classification rests on witness content that cannot be
//! fabricated meaningfully (a v1 redeem must carry a preimage hashing to
//! `H`; a v2 refund must reveal the exact tapleaf we can rebuild; a v2
//! key-path spend can only exist co-signed by the MuSig2 aggregate). A
//! garbage witness lands in [`SpendKind::Unknown`], which never drives a
//! terminal decision.

use anyhow::{bail, Context, Result};
use bitcoin::{OutPoint, Script, ScriptBuf};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::chain::ChainBackend;
use crate::htlc::extract_preimage;
use crate::params::Network;

/// How a swap-leg funding output was spent, judged from the spending
/// input's witness alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpendKind {
    /// The claim path: v1 hash-branch (preimage verified against `H`),
    /// v2 key-path (single Schnorr sig — only the MuSig2 aggregate can).
    Redeem,
    /// The timeout path: v1 CLTV branch, v2 refund tapleaf (byte-equal to
    /// the locally rebuilt leaf script).
    Refund,
    /// Neither shape — anomalous. Never treated as a terminal signal.
    Unknown,
}

/// One leg's spent-state, with everything a caller needs to persist
/// pointers, judge finality, or fast-forward a record.
#[derive(Debug, Clone)]
pub struct SpentLeg {
    pub outpoint: OutPoint,
    /// Funding block height (0 = still unconfirmed — possible when the
    /// spend is a same-mempool chain).
    pub funding_height: u64,
    pub spend_txid: String,
    /// Spend block height (0 = unconfirmed).
    pub spend_height: u64,
    pub spend_confs: u64,
    pub kind: SpendKind,
    /// Full hex of the spending tx — lets a takeover adopt the spend as its
    /// own `final_tx` so the existing confirmation nurses converge on it.
    pub spend_tx_hex: String,
}

/// The complete classification of one swap leg, front-to-back.
#[derive(Debug, Clone)]
pub enum LegClass {
    /// No output paying `(spk, amount)` has ever appeared.
    Unfunded,
    /// The funding output exists and is unspent.
    Funded {
        outpoint: OutPoint,
        /// Funding block height (0 = mempool).
        height: u64,
        confs: u64,
    },
    /// The funding output existed and is spent — the historical fact no
    /// live-UTXO read can see.
    Spent(SpentLeg),
    /// The funding is PROVEN (wallet evidence, #171) and the output is gone
    /// from the UTXO set — an unambiguous spend whose spending tx we cannot
    /// retrieve (tier L, spent by the counterparty). Enough to refuse a
    /// re-fund and to resolve a follower via the tip-drift buffer; never
    /// enough for a depth-verified terminal.
    Vanished {
        outpoint: OutPoint,
        /// Funding block height (0 = unknown/unconfirmed at recording).
        funding_height: u64,
    },
}

/// Classify a v1 (P2WSH HTLC) spend from its witness. The spk match already
/// proves the spend runs OUR witness script (consensus checks the last item
/// against the P2WSH hash), so only the branch needs judging:
/// redeem = `[sig, pubkey, s, 0x01, script]` (a 32-byte item hashing to `H`
/// — position-independent and hash-verified, `crate::htlc::extract_preimage`);
/// refund = `[sig, pubkey, <>, script]` (empty OP_ELSE selector).
pub fn classify_v1_spend(witness: &[Vec<u8>], hash_h: &[u8; 32]) -> SpendKind {
    if extract_preimage(witness, hash_h).is_some() {
        return SpendKind::Redeem;
    }
    if witness.len() == 4 && witness[2].is_empty() {
        return SpendKind::Refund;
    }
    SpendKind::Unknown
}

/// Classify a v2 (Taproot) spend from its witness. The leg has exactly ONE
/// tapleaf (`crate::taproot::TaprootLeg`), so consensus admits two shapes:
/// a key-path spend (single 64/65-byte Schnorr sig — only the 2-of-2 MuSig2
/// aggregate can produce it, i.e. the cooperative redeem) or our CLTV
/// refund leaf (`[sig, leaf_script, control_block]`, leaf byte-equal to the
/// locally rebuilt `refund_script()`). Anything else — including a
/// fabricated "signature" a lying server could invent — is `Unknown`.
pub fn classify_v2_spend(witness: &[Vec<u8>], refund_script: &Script) -> SpendKind {
    match witness {
        [sig] if sig.len() == 64 || sig.len() == 65 => SpendKind::Redeem,
        [_sig, script, _ctrl] if script.as_slice() == refund_script.as_bytes() => SpendKind::Refund,
        _ => SpendKind::Unknown,
    }
}

/// Defensive cap on how many history entries a leg classification will
/// fetch. A swap leg's script is unique to the swap, so its real history is
/// a handful of transactions; anything larger is address spam and reads as
/// inconclusive rather than an unbounded fetch loop.
const MAX_HISTORY_TXS: usize = 24;

/// Reconstruct one swap leg's state from the chain, front-to-back.
///
/// `Ok(None)` = the backend has no script history (tier L) — the caller
/// falls back to live reads + the timelock age-out. Any other
/// inconclusiveness (unfetchable history tx, oversized history) is an
/// `Err`, which callers treat as "leave the record untouched, retry later".
///
/// Cost: one `spk_history` round-trip, plus one `fetch_tx` per history
/// entry — a handful, and only paid when the caller has no cached
/// classification (see the follow evaluator's spend cache).
pub fn classify_leg(
    backend: &dyn ChainBackend,
    spk: &ScriptBuf,
    amount_sat: u64,
    classify_spend: &dyn Fn(&[Vec<u8>]) -> SpendKind,
) -> Result<Option<LegClass>> {
    let Some(entries) = backend.spk_history(spk)? else {
        return Ok(None); // tier L — no script index on any view
    };
    if entries.is_empty() {
        return Ok(Some(LegClass::Unfunded));
    }
    if entries.len() > MAX_HISTORY_TXS {
        bail!(
            "script history has {} entries — not a plausible swap leg, refusing to classify",
            entries.len()
        );
    }
    let tip = backend.tip_height()?;
    let confs_of = |height: u64| -> u64 {
        if height > 0 && tip >= height {
            tip - height + 1
        } else {
            0
        }
    };

    // Fetch every history tx once. A missing tx is inconclusive — the
    // entry came from the same backend set, so absence is a transient gap,
    // not evidence.
    let mut txs = Vec::with_capacity(entries.len());
    for (txid, height) in &entries {
        let tx = backend
            .fetch_tx(txid)?
            .with_context(|| format!("history tx {txid} not retrievable — inconclusive"))?;
        // Electrum reports mempool entries as height 0 / -1.
        let height = u64::try_from(*height).unwrap_or(0);
        txs.push((txid.clone(), height, tx));
    }

    // Funding candidates: outputs byte-matching the derived spk AND the
    // agreed amount (a wrong-amount payment to the same script is a
    // mis-funding, ignored exactly like the live `find_funding` path).
    let mut candidates: Vec<(OutPoint, u64)> = Vec::new(); // (outpoint, funding height)
    for (txid, height, tx) in &txs {
        for (vout, out) in tx.output.iter().enumerate() {
            if out.script_pubkey == *spk && out.value.to_sat() == amount_sat {
                candidates.push((
                    OutPoint {
                        txid: bitcoin::Txid::from_str(txid)?,
                        vout: vout as u32,
                    },
                    *height,
                ));
            }
        }
    }
    if candidates.is_empty() {
        return Ok(Some(LegClass::Unfunded));
    }

    // Spend lookup per candidate; prefer a SPENT candidate (a completed
    // swap must classify terminal even if a stray duplicate funding
    // lingers unspent).
    let find_spend =
        |op: &OutPoint| -> Option<(&String, u64, &bitcoin::Transaction, Vec<Vec<u8>>)> {
            for (txid, height, tx) in &txs {
                for input in &tx.input {
                    if input.previous_output == *op {
                        let witness: Vec<Vec<u8>> =
                            input.witness.iter().map(|item| item.to_vec()).collect();
                        return Some((txid, *height, tx, witness));
                    }
                }
            }
            None
        };
    for (op, funding_height) in &candidates {
        if let Some((spend_txid, spend_height, spend_tx, witness)) = find_spend(op) {
            return Ok(Some(LegClass::Spent(SpentLeg {
                outpoint: *op,
                funding_height: *funding_height,
                spend_txid: spend_txid.clone(),
                spend_height,
                spend_confs: confs_of(spend_height),
                kind: classify_spend(&witness),
                spend_tx_hex: bitcoin::consensus::encode::serialize_hex(spend_tx),
            })));
        }
    }
    let (op, height) = candidates[0];
    Ok(Some(LegClass::Funded {
        outpoint: op,
        height,
        confs: confs_of(height),
    }))
}

/// Safety margin past a swap's LAST timelock before a followed record with
/// no visible funds may be aged out (docs/STATE_RECONSTRUCTION.md §4.2):
/// generous enough that no rational continuation exists, and at least the
/// finality budget so a reorg cannot un-pass it. `0` on regtest, matching
/// the `action_margins` house style (tests jump clocks).
pub fn age_out_margin_secs(network: Network, needed_confs: u32, target_spacing_secs: u32) -> u64 {
    if network == Network::Regtest {
        return 0;
    }
    86_400u64.max(6 * u64::from(needed_confs.max(1)) * u64::from(target_spacing_secs))
}

/// What the NODE WALLET's own history can prove about a swap leg (#171).
/// POSITIVE-ONLY: the backup-session contract shares the wallet across a
/// merchant's machines, so this sees every transaction the merchant SIDE
/// made — but never the counterparty's. Absence proves nothing.
#[derive(Debug, Clone)]
pub enum WalletEvidence {
    /// The leg's funding output was spent, and the wallet holds the spending
    /// tx (our claim/refund — it pays the wallet), fully classified.
    Spent(SpentLeg),
    /// The wallet FUNDED this leg (our own send) — the pointer survives the
    /// output being spent, unlike any live-UTXO read. Liveness is NOT
    /// implied: the caller must ask the chain (`get_txout`) whether the
    /// output still exists; a vanished pointer is an unambiguous spend
    /// (depth unknowable without the spending tx).
    FundingPointer { outpoint: OutPoint, height: u64 },
}

/// Extract wallet evidence for one leg from the wallet's decoded
/// transactions ([`crate::chain::ChainBackend::wallet_txs_since`]).
///
/// Three positive shapes, strongest first:
/// - a wallet tx SPENDS a funding we can also see the wallet make → full
///   [`SpentLeg`] (kind from the witness, both heights known);
/// - a wallet tx is OUR CLAIM of a counterparty-funded leg (`claim_probe`
///   matches an input — v1: the revealed witness script byte-equals ours;
///   v2: the refund leaf byte-equals, or a key-path spend sweeping to the
///   record's negotiated sweep address) → [`SpentLeg`] with the funding
///   outpoint recovered from the claim's input (funding height unknown);
/// - a wallet tx FUNDS the leg (output pays `(spk, amount)`) with no
///   wallet-visible spend → [`WalletEvidence::FundingPointer`].
pub fn classify_leg_wallet(
    wallet_txs: &[(bitcoin::Transaction, u64)],
    tip: u64,
    spk: &ScriptBuf,
    amount_sat: u64,
    classify_spend: &dyn Fn(&[Vec<u8>]) -> SpendKind,
    claim_probe: &dyn Fn(&bitcoin::Transaction, usize) -> bool,
) -> Option<WalletEvidence> {
    let confs_of = |height: u64| -> u64 {
        if height > 0 && tip >= height {
            tip - height + 1
        } else {
            0
        }
    };
    // Fundings the wallet itself made.
    let mut fundings: Vec<(OutPoint, u64)> = Vec::new();
    for (tx, height) in wallet_txs {
        for (vout, out) in tx.output.iter().enumerate() {
            if out.script_pubkey == *spk && out.value.to_sat() == amount_sat {
                fundings.push((
                    OutPoint {
                        txid: tx.compute_txid(),
                        vout: vout as u32,
                    },
                    *height,
                ));
            }
        }
    }
    let spent_leg = |outpoint: OutPoint,
                     funding_height: u64,
                     tx: &bitcoin::Transaction,
                     height: u64,
                     witness: Vec<Vec<u8>>| {
        WalletEvidence::Spent(SpentLeg {
            outpoint,
            funding_height,
            spend_txid: tx.compute_txid().to_string(),
            spend_height: height,
            spend_confs: confs_of(height),
            kind: classify_spend(&witness),
            spend_tx_hex: bitcoin::consensus::encode::serialize_hex(tx),
        })
    };
    // Wallet-visible spends of those fundings (our own refunds, and claims
    // of legs we also funded — not a real v1/v2 shape, but cheap to cover).
    for (tx, height) in wallet_txs {
        for input in &tx.input {
            if let Some((op, fh)) = fundings.iter().find(|(op, _)| input.previous_output == *op) {
                let witness: Vec<Vec<u8>> = input.witness.iter().map(|i| i.to_vec()).collect();
                return Some(spent_leg(*op, *fh, tx, *height, witness));
            }
        }
    }
    // Our claim of a COUNTERPARTY-funded leg: the claim pays our wallet, so
    // it is a wallet tx; the probe identifies which input spends OUR leg,
    // and its prevout IS the funding outpoint we never saw live.
    for (tx, height) in wallet_txs {
        for (idx, input) in tx.input.iter().enumerate() {
            if claim_probe(tx, idx) {
                let witness: Vec<Vec<u8>> = input.witness.iter().map(|i| i.to_vec()).collect();
                return Some(spent_leg(input.previous_output, 0, tx, *height, witness));
            }
        }
    }
    fundings
        .into_iter()
        .next()
        .map(|(outpoint, height)| WalletEvidence::FundingPointer { outpoint, height })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{ChainParams, BTC_REGTEST};
    use crate::taproot::TaprootLeg;
    use anyhow::Result;
    use bitcoin::absolute::LockTime;
    use bitcoin::hashes::{sha256, Hash};
    use bitcoin::secp256k1::{Keypair, Secp256k1};
    use bitcoin::transaction::Version;
    use bitcoin::{Amount, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Txid, Witness};

    fn hash_of(preimage: &[u8; 32]) -> [u8; 32] {
        sha256::Hash::hash(preimage).to_byte_array()
    }

    #[test]
    fn v1_witness_classification() {
        let s = [7u8; 32];
        let h = hash_of(&s);
        let script = vec![0xAAu8; 40];
        // Redeem: [sig, pubkey, s, 0x01, script] — preimage verifies.
        let redeem = vec![
            vec![0x30; 71],
            vec![0x02; 33],
            s.to_vec(),
            vec![1],
            script.clone(),
        ];
        assert_eq!(classify_v1_spend(&redeem, &h), SpendKind::Redeem);
        // Refund: [sig, pubkey, <>, script] — empty OP_ELSE selector.
        let refund = vec![vec![0x30; 71], vec![0x02; 33], vec![], script.clone()];
        assert_eq!(classify_v1_spend(&refund, &h), SpendKind::Refund);
        // A 32-byte item that does NOT hash to H is not a redeem; the
        // 4-item shape with a non-empty selector is not a refund either.
        let bogus = vec![vec![0x30; 71], vec![0x02; 33], vec![0x55; 32], script];
        assert_eq!(classify_v1_spend(&bogus, &h), SpendKind::Unknown);
        assert_eq!(classify_v1_spend(&[vec![0u8; 64]], &h), SpendKind::Unknown);
    }

    fn sample_leg() -> (TaprootLeg, Secp256k1<bitcoin::secp256k1::All>) {
        let secp = Secp256k1::new();
        let internal = Keypair::from_seckey_slice(&secp, &[0x24; 32])
            .unwrap()
            .x_only_public_key()
            .0;
        let refund = Keypair::from_seckey_slice(&secp, &[0x42; 32])
            .unwrap()
            .x_only_public_key()
            .0;
        (
            TaprootLeg::new(internal, refund, 1_780_000_000).unwrap(),
            secp,
        )
    }

    #[test]
    fn v2_witness_classification() {
        let (leg, _secp) = sample_leg();
        let leaf = leg.refund_script();
        // Key-path spend: exactly one 64-byte sig (SIGHASH_DEFAULT)…
        assert_eq!(
            classify_v2_spend(&[vec![0u8; 64]], &leaf),
            SpendKind::Redeem
        );
        // …or 65 with an explicit sighash byte.
        assert_eq!(
            classify_v2_spend(&[vec![0u8; 65]], &leaf),
            SpendKind::Redeem
        );
        // Script-path refund: [sig, leaf, control] with OUR leaf bytes.
        let refund = vec![vec![0u8; 64], leaf.as_bytes().to_vec(), vec![0xC0; 33]];
        assert_eq!(classify_v2_spend(&refund, &leaf), SpendKind::Refund);
        // A different leaf script is not our refund.
        let alien = vec![vec![0u8; 64], vec![0x51], vec![0xC0; 33]];
        assert_eq!(classify_v2_spend(&alien, &leaf), SpendKind::Unknown);
        // A fabricated "sig" of the wrong size is nothing.
        assert_eq!(
            classify_v2_spend(&[vec![0u8; 63]], &leaf),
            SpendKind::Unknown
        );
    }

    // ---- classify_leg over a canned-history mock backend -------------------

    struct MockBackend {
        history: Option<Vec<(String, i64)>>,
        txs: Vec<Transaction>,
        tip: u64,
    }

    impl ChainBackend for MockBackend {
        fn params(&self) -> &ChainParams {
            &BTC_REGTEST
        }
        fn verify_chain(&self) -> Result<()> {
            Ok(())
        }
        fn broadcast(&self, _tx: &Transaction) -> Result<Txid> {
            anyhow::bail!("mock")
        }
        fn get_txout(
            &self,
            _outpoint: &bitcoin::OutPoint,
            _expected_spk: &ScriptBuf,
        ) -> Result<Option<crate::chain::TxOutInfo>> {
            anyhow::bail!("mock")
        }
        fn find_funding(
            &self,
            _spk: &ScriptBuf,
        ) -> Result<Option<(bitcoin::OutPoint, crate::chain::TxOutInfo)>> {
            anyhow::bail!("mock")
        }
        fn find_vout(&self, _txid: &str, _spk_hex: &str) -> Result<u32> {
            anyhow::bail!("mock")
        }
        fn find_spend_witness(
            &self,
            _outpoint: &bitcoin::OutPoint,
            _watch_spk: &ScriptBuf,
            _from_height: u64,
        ) -> Result<Option<Vec<Vec<u8>>>> {
            anyhow::bail!("mock")
        }
        fn spk_history(&self, _spk: &ScriptBuf) -> Result<Option<Vec<(String, i64)>>> {
            Ok(self.history.clone())
        }
        fn fetch_tx(&self, txid: &str) -> Result<Option<Transaction>> {
            let want = Txid::from_str(txid)?;
            Ok(self.txs.iter().find(|t| t.compute_txid() == want).cloned())
        }
        fn tip_height(&self) -> Result<u64> {
            Ok(self.tip)
        }
        fn tip_median_time(&self) -> Result<u64> {
            anyhow::bail!("mock")
        }
        fn tx_confirmations(&self, _txid: &str, _spk: Option<&ScriptBuf>) -> Result<u64> {
            anyhow::bail!("mock")
        }
        fn fee_rate_for(&self, _conf_target: u16, _conservative: bool) -> Result<u64> {
            anyhow::bail!("mock")
        }
        fn wallet_new_address(&self) -> Result<String> {
            anyhow::bail!("mock")
        }
        fn wallet_balance(&self) -> Result<u64> {
            anyhow::bail!("mock")
        }
        fn wallet_send(
            &self,
            _address: &str,
            _amount_sat: u64,
            _fee: crate::chain::SendFee,
        ) -> Result<String> {
            anyhow::bail!("mock")
        }
    }

    fn spk() -> ScriptBuf {
        ScriptBuf::new_p2wsh(&ScriptBuf::from(vec![0x51u8]).wscript_hash())
    }

    fn funding_tx(spk: &ScriptBuf, amount: u64) -> Transaction {
        Transaction {
            version: Version::TWO,
            lock_time: LockTime::ZERO,
            input: vec![TxIn {
                previous_output: bitcoin::OutPoint {
                    txid: Txid::from_str(&"33".repeat(32)).unwrap(),
                    vout: 0,
                },
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::default(),
            }],
            output: vec![TxOut {
                value: Amount::from_sat(amount),
                script_pubkey: spk.clone(),
            }],
        }
    }

    fn spend_tx(funding: &Transaction, witness_items: &[Vec<u8>]) -> Transaction {
        let mut w = Witness::new();
        for item in witness_items {
            w.push(item.clone());
        }
        Transaction {
            version: Version::TWO,
            lock_time: LockTime::ZERO,
            input: vec![TxIn {
                previous_output: bitcoin::OutPoint {
                    txid: funding.compute_txid(),
                    vout: 0,
                },
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: w,
            }],
            output: vec![TxOut {
                value: Amount::from_sat(90_000),
                script_pubkey: ScriptBuf::new(),
            }],
        }
    }

    fn kind_always(kind: SpendKind) -> impl Fn(&[Vec<u8>]) -> SpendKind {
        move |_| kind
    }

    #[test]
    fn leg_unfunded_and_tier_l() {
        let spk = spk();
        let none = MockBackend {
            history: None,
            txs: vec![],
            tip: 100,
        };
        assert!(
            classify_leg(&none, &spk, 100_000, &kind_always(SpendKind::Unknown))
                .unwrap()
                .is_none(),
            "history-less backend reads as tier L"
        );
        let empty = MockBackend {
            history: Some(vec![]),
            txs: vec![],
            tip: 100,
        };
        assert!(matches!(
            classify_leg(&empty, &spk, 100_000, &kind_always(SpendKind::Unknown))
                .unwrap()
                .unwrap(),
            LegClass::Unfunded
        ));
    }

    #[test]
    fn leg_funded_live() {
        let spk = spk();
        let f = funding_tx(&spk, 100_000);
        let backend = MockBackend {
            history: Some(vec![(f.compute_txid().to_string(), 90)]),
            txs: vec![f.clone()],
            tip: 100,
        };
        match classify_leg(&backend, &spk, 100_000, &kind_always(SpendKind::Unknown))
            .unwrap()
            .unwrap()
        {
            LegClass::Funded {
                outpoint,
                height,
                confs,
            } => {
                assert_eq!(outpoint.txid, f.compute_txid());
                assert_eq!(height, 90);
                assert_eq!(confs, 11);
            }
            other => panic!("expected Funded, got {other:?}"),
        }
    }

    #[test]
    fn leg_wrong_amount_is_unfunded() {
        let spk = spk();
        let f = funding_tx(&spk, 55_555); // pays the spk, but not the agreed amount
        let backend = MockBackend {
            history: Some(vec![(f.compute_txid().to_string(), 90)]),
            txs: vec![f],
            tip: 100,
        };
        assert!(matches!(
            classify_leg(&backend, &spk, 100_000, &kind_always(SpendKind::Unknown))
                .unwrap()
                .unwrap(),
            LegClass::Unfunded
        ));
    }

    #[test]
    fn leg_spent_classifies_front_to_back() {
        // The field bug's shape: funding AND spend are both history — a
        // live-UTXO read sees nothing, the classifier sees the whole story.
        let s = [9u8; 32];
        let h = hash_of(&s);
        let spk = spk();
        let f = funding_tx(&spk, 100_000);
        let redeem_witness = vec![
            vec![0x30; 71],
            vec![0x02; 33],
            s.to_vec(),
            vec![1],
            vec![0xAA; 40],
        ];
        let sp = spend_tx(&f, &redeem_witness);
        let backend = MockBackend {
            history: Some(vec![
                (f.compute_txid().to_string(), 90),
                (sp.compute_txid().to_string(), 95),
            ]),
            txs: vec![f.clone(), sp.clone()],
            tip: 100,
        };
        let classify = |w: &[Vec<u8>]| classify_v1_spend(w, &h);
        match classify_leg(&backend, &spk, 100_000, &classify)
            .unwrap()
            .unwrap()
        {
            LegClass::Spent(leg) => {
                assert_eq!(leg.outpoint.txid, f.compute_txid());
                assert_eq!(leg.spend_txid, sp.compute_txid().to_string());
                assert_eq!(leg.spend_height, 95);
                assert_eq!(leg.spend_confs, 6);
                assert_eq!(leg.kind, SpendKind::Redeem);
                assert_eq!(leg.funding_height, 90);
            }
            other => panic!("expected Spent, got {other:?}"),
        }
    }

    #[test]
    fn leg_mempool_spend_has_zero_confs() {
        let spk = spk();
        let f = funding_tx(&spk, 100_000);
        let sp = spend_tx(&f, &[vec![0u8; 64]]);
        let backend = MockBackend {
            history: Some(vec![
                (f.compute_txid().to_string(), 90),
                (sp.compute_txid().to_string(), -1), // unconfirmed-parents marker
            ]),
            txs: vec![f, sp],
            tip: 100,
        };
        match classify_leg(&backend, &spk, 100_000, &kind_always(SpendKind::Redeem))
            .unwrap()
            .unwrap()
        {
            LegClass::Spent(leg) => {
                assert_eq!(leg.spend_height, 0);
                assert_eq!(leg.spend_confs, 0);
            }
            other => panic!("expected Spent, got {other:?}"),
        }
    }

    #[test]
    fn missing_history_tx_is_an_error_not_evidence() {
        let spk = spk();
        let f = funding_tx(&spk, 100_000);
        let backend = MockBackend {
            history: Some(vec![(f.compute_txid().to_string(), 90)]),
            txs: vec![], // the referenced tx is not retrievable
            tip: 100,
        };
        assert!(classify_leg(&backend, &spk, 100_000, &kind_always(SpendKind::Unknown)).is_err());
    }

    #[test]
    fn age_out_margin_shape() {
        assert_eq!(age_out_margin_secs(Network::Regtest, 6, 600), 0);
        // Mainnet floor is a day…
        assert_eq!(age_out_margin_secs(Network::Mainnet, 1, 600), 86_400);
        // …and scales with the finality budget on slow/deep configs.
        assert_eq!(age_out_margin_secs(Network::Mainnet, 30, 600), 108_000);
    }

    // ---- wallet-assisted evidence (#171) -----------------------------------

    fn no_probe(_tx: &Transaction, _idx: usize) -> bool {
        false
    }

    #[test]
    fn wallet_funding_only_yields_pointer() {
        let spk = spk();
        let f = funding_tx(&spk, 100_000);
        let txs = vec![(f.clone(), 90u64)];
        match classify_leg_wallet(
            &txs,
            100,
            &spk,
            100_000,
            &kind_always(SpendKind::Unknown),
            &no_probe,
        ) {
            Some(WalletEvidence::FundingPointer { outpoint, height }) => {
                assert_eq!(outpoint.txid, f.compute_txid());
                assert_eq!(height, 90);
            }
            other => panic!("expected FundingPointer, got {other:?}"),
        }
        // Wrong amount → the wallet proves nothing about THIS leg.
        assert!(classify_leg_wallet(
            &txs,
            100,
            &spk,
            55_555,
            &kind_always(SpendKind::Unknown),
            &no_probe
        )
        .is_none());
    }

    #[test]
    fn wallet_funding_plus_spend_is_fully_classified() {
        let spk = spk();
        let f = funding_tx(&spk, 100_000);
        let sp = spend_tx(
            &f,
            &[vec![0x30; 71], vec![0x02; 33], vec![], vec![0xAA; 40]],
        );
        let txs = vec![(f.clone(), 90u64), (sp.clone(), 95u64)];
        match classify_leg_wallet(
            &txs,
            100,
            &spk,
            100_000,
            &kind_always(SpendKind::Refund),
            &no_probe,
        ) {
            Some(WalletEvidence::Spent(leg)) => {
                assert_eq!(leg.outpoint.txid, f.compute_txid());
                assert_eq!(leg.spend_txid, sp.compute_txid().to_string());
                assert_eq!(leg.funding_height, 90);
                assert_eq!(leg.spend_confs, 6);
                assert_eq!(leg.kind, SpendKind::Refund);
            }
            other => panic!("expected Spent, got {other:?}"),
        }
    }

    #[test]
    fn wallet_claim_of_counterparty_funding_recovers_the_outpoint() {
        // The user's ghost shape: the counterparty funded the leg (invisible
        // to the wallet), OUR claim swept it to the wallet — the claim's
        // input IS the funding outpoint we never saw.
        let spk = spk();
        let s = [9u8; 32];
        let h = hash_of(&s);
        let ws = vec![0xAB; 40]; // the leg's witness script bytes (probe target)
        let foreign_funding = funding_tx(&spk, 100_000); // NOT in the wallet set
        let claim = spend_tx(
            &foreign_funding,
            &[
                vec![0x30; 71],
                vec![0x02; 33],
                s.to_vec(),
                vec![1],
                ws.clone(),
            ],
        );
        let txs = vec![(claim.clone(), 95u64)];
        let classify = |w: &[Vec<u8>]| classify_v1_spend(w, &h);
        let probe = |tx: &Transaction, idx: usize| {
            tx.input[idx]
                .witness
                .last()
                .map(|w| w == ws.as_slice())
                .unwrap_or(false)
        };
        match classify_leg_wallet(&txs, 100, &spk, 100_000, &classify, &probe) {
            Some(WalletEvidence::Spent(leg)) => {
                assert_eq!(leg.outpoint.txid, foreign_funding.compute_txid());
                assert_eq!(leg.funding_height, 0, "funding height unknowable");
                assert_eq!(leg.kind, SpendKind::Redeem, "preimage verified");
                assert_eq!(leg.spend_confs, 6);
            }
            other => panic!("expected Spent via claim probe, got {other:?}"),
        }
        // Without the probe the wallet proves nothing (positive-only).
        assert!(classify_leg_wallet(&txs, 100, &spk, 100_000, &classify, &no_probe).is_none());
    }
}
