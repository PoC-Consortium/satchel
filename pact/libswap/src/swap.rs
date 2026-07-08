//! Swap parameters and HTLC spend transactions — spec §6, §7, §9.

use anyhow::{ensure, Context, Result};
use bitcoin::absolute::LockTime;
use bitcoin::hashes::Hash;
use bitcoin::secp256k1::{Message, Secp256k1, SecretKey};
use bitcoin::sighash::{EcdsaSighashType, SighashCache};
use bitcoin::transaction::Version;
use bitcoin::{Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness};
use serde::{Deserialize, Serialize};

use crate::htlc::Htlc;
use crate::params::ChainParams;

/// Both redeem and refund signal RBF and keep locktime enforceable
/// (spec §6.2/§6.3).
pub const HTLC_SPEND_SEQUENCE: u32 = 0xFFFF_FFFD;

/// Conservative dust bound for the swept output (spec §6.4).
pub const DUST_LIMIT_SAT: u64 = 546;

/// Worst-case vsizes of the 1-in/1-out HTLC spends (P2WSH input with the
/// §6.2/§6.3 witnesses, one P2WSH-sized output) — used to turn a feerate
/// into an absolute fee before the witness exists.
pub const REDEEM_TX_VSIZE: u64 = 155;
pub const REFUND_TX_VSIZE: u64 = 146;

/// Estimated vsize of the HTLC *funding* tx (spec §6.1) — a normal wallet
/// send building the P2WSH output. Unlike the spends above this isn't a tx we
/// construct (the user's core wallet does, coin-selection and all), so it's an
/// estimate for the fee *preview* only: one P2WPKH input + the P2WSH HTLC
/// output + a P2WPKH change output ≈ 1-in/2-out segwit ≈ 150–170 vB. 160 is a
/// sensible mid-point; real wallet selection (more inputs) may differ.
pub const FUND_TX_VSIZE: u64 = 160;

/// Default for the deprecated [`crate::fee_policy::FeeBumpPolicy::min_fee_sat`]
/// field — retained only so previously-persisted policies still deserialize. It
/// is **not** a fee floor: every spend/bump is market-derived (`spend_fee_sat` /
/// [`crate::fee_policy::FeeBumpPolicy::target_feerate`]).
pub const MIN_SPEND_FEE_SAT: u64 = 1000;

/// Absolute fee (sat) for an HTLC spend at the given feerate. The feerate is
/// already market-derived and clamped to ≥ 1 sat/vB upstream (`target_feerate`
/// and the estimator), 1 sat/vB being the relay minimum — so this is just
/// `rate × vsize` with a defensive min-relay guard, **not** an arbitrary
/// absolute floor (the old 1000-sat floor was removed: it overrode the market
/// price on quiet mempools).
pub fn spend_fee_sat(rate_sat_per_vb: u64, tx_vsize: u64) -> u64 {
    rate_sat_per_vb.max(1).saturating_mul(tx_vsize)
}

/// Legacy alias used by tests; production paths compute via
/// [`spend_fee_sat`].
pub const FLAT_FEE_SAT: u64 = MIN_SPEND_FEE_SAT;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Alice — holds the preimage, locks chain A, refund at T1.
    Initiator,
    /// Bob — locks chain B, refund at T2 < T1.
    Participant,
}

/// Spec §9 lifecycle (one party's view). Refund states are reachable from
/// any funded state via the clock, not via messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Created,
    Accepted,
    FundedA,
    FundedB,
    RedeemedB,
    Completed,
    Refunded,
    Aborted,
}

/// Everything both parties know after `accept` — sufficient to reconstruct
/// both HTLCs deterministically (spec §8.4).
#[derive(Debug, Clone)]
pub struct SwapParams {
    pub chain_a: &'static ChainParams,
    pub chain_b: &'static ChainParams,
    pub amount_a: u64,
    pub amount_b: u64,
    pub hash_h: [u8; 32],
    pub t1: u32,
    pub t2: u32,
    pub n_a: u32,
    pub n_b: u32,
    pub alice_refund_pubkey_a: bitcoin::secp256k1::PublicKey,
    pub alice_redeem_pubkey_b: bitcoin::secp256k1::PublicKey,
    pub bob_redeem_pubkey_a: bitcoin::secp256k1::PublicKey,
    pub bob_refund_pubkey_b: bitcoin::secp256k1::PublicKey,
}

impl SwapParams {
    /// Structural timelock rules that hold on every network (spec §7.1).
    /// Network-profile duration minimums (§7.3) are policy, checked by the
    /// caller against its own clock and profile.
    pub fn validate_structure(&self) -> Result<()> {
        ensure!(self.t2 < self.t1, "spec §7.1: T2 must be < T1");
        ensure!(
            self.amount_a > 0 && self.amount_b > 0,
            "amounts must be positive"
        );
        Ok(())
    }

    /// The chain-A HTLC: Bob redeems with `s`, Alice refunds at T1.
    pub fn htlc_a(&self) -> Result<Htlc> {
        Htlc::new(
            self.hash_h,
            self.bob_redeem_pubkey_a,
            self.alice_refund_pubkey_a,
            self.t1,
        )
    }

    /// The chain-B HTLC: Alice redeems with `s`, Bob refunds at T2.
    pub fn htlc_b(&self) -> Result<Htlc> {
        Htlc::new(
            self.hash_h,
            self.alice_redeem_pubkey_b,
            self.bob_refund_pubkey_b,
            self.t2,
        )
    }
}

/// Shared skeleton + BIP143 signature for both HTLC spend paths.
fn signed_htlc_spend(
    htlc: &Htlc,
    outpoint: OutPoint,
    htlc_value_sat: u64,
    destination: ScriptBuf,
    fee_sat: u64,
    lock_time: LockTime,
    key: &SecretKey,
    build_witness: impl FnOnce(Vec<u8>, Vec<u8>, &ScriptBuf) -> Witness,
) -> Result<Transaction> {
    ensure!(
        htlc_value_sat > fee_sat + DUST_LIMIT_SAT,
        "HTLC value {htlc_value_sat} cannot cover fee {fee_sat} plus dust (spec §6.4)"
    );
    let witness_script = htlc.witness_script();
    let mut tx = Transaction {
        version: Version::TWO,
        lock_time,
        input: vec![TxIn {
            previous_output: outpoint,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::from_consensus(HTLC_SPEND_SEQUENCE),
            witness: Witness::default(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(htlc_value_sat - fee_sat),
            script_pubkey: destination,
        }],
    };

    let sighash = SighashCache::new(&tx)
        .p2wsh_signature_hash(
            0,
            &witness_script,
            Amount::from_sat(htlc_value_sat),
            EcdsaSighashType::All,
        )
        .context("sighash computation")?;
    let secp = Secp256k1::new();
    let signature = secp.sign_ecdsa(&Message::from_digest(sighash.to_byte_array()), key);
    let mut sig_with_hashtype = signature.serialize_der().to_vec();
    sig_with_hashtype.push(EcdsaSighashType::All as u8);
    let pubkey = key.public_key(&secp).serialize().to_vec();

    tx.input[0].witness = build_witness(sig_with_hashtype, pubkey, &witness_script);
    Ok(tx)
}

/// Redeem transaction (spec §6.2): hash branch, witness
/// `[sig, pubkey, s, 0x01, witness_script]`, nLockTime 0.
pub fn build_redeem_tx(
    htlc: &Htlc,
    outpoint: OutPoint,
    htlc_value_sat: u64,
    destination: ScriptBuf,
    fee_sat: u64,
    preimage: &[u8; 32],
    key: &SecretKey,
) -> Result<Transaction> {
    signed_htlc_spend(
        htlc,
        outpoint,
        htlc_value_sat,
        destination,
        fee_sat,
        LockTime::ZERO,
        key,
        |sig, pubkey, witness_script| {
            let mut witness = Witness::new();
            witness.push(sig);
            witness.push(pubkey);
            witness.push(preimage);
            witness.push([0x01]); // select the OP_IF branch
            witness.push(witness_script.as_bytes());
            witness
        },
    )
}

/// Refund transaction (spec §6.3): timeout branch, witness
/// `[sig, pubkey, <>, witness_script]`, nLockTime = T. Valid only once the
/// chain's MTP reaches T; broadcasting earlier is rejected, not fatal.
pub fn build_refund_tx(
    htlc: &Htlc,
    outpoint: OutPoint,
    htlc_value_sat: u64,
    destination: ScriptBuf,
    fee_sat: u64,
    key: &SecretKey,
) -> Result<Transaction> {
    signed_htlc_spend(
        htlc,
        outpoint,
        htlc_value_sat,
        destination,
        fee_sat,
        LockTime::from_consensus(htlc.locktime),
        key,
        |sig, pubkey, witness_script| {
            let mut witness = Witness::new();
            witness.push(sig);
            witness.push(pubkey);
            witness.push([] as [u8; 0]); // empty item selects the OP_ELSE branch
            witness.push(witness_script.as_bytes());
            witness
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::{DeriveScope, PactSeed, COIN_BTC, COIN_BTCX};
    use crate::params::{BTCX_REGTEST, BTC_REGTEST};
    use std::str::FromStr;

    fn test_params() -> (SwapParams, PactSeed, PactSeed, [u8; 32]) {
        let alice = PactSeed::from_seed(&[1u8; 64]).unwrap();
        let bob = PactSeed::from_seed(&[2u8; 64]).unwrap();
        let s = alice.preimage(DeriveScope::LEGACY, 0).unwrap();
        let params = SwapParams {
            chain_a: &BTCX_REGTEST,
            chain_b: &BTC_REGTEST,
            amount_a: 50_0000_0000,
            amount_b: 10_0000,
            hash_h: crate::keys::hash_preimage(&s),
            t1: 1_780_043_200,
            t2: 1_780_021_600,
            n_a: 1,
            n_b: 1,
            alice_refund_pubkey_a: alice
                .swap_pubkey(COIN_BTCX, DeriveScope::LEGACY, 0)
                .unwrap(),
            alice_redeem_pubkey_b: alice.swap_pubkey(COIN_BTC, DeriveScope::LEGACY, 0).unwrap(),
            bob_redeem_pubkey_a: bob.swap_pubkey(COIN_BTCX, DeriveScope::LEGACY, 0).unwrap(),
            bob_refund_pubkey_b: bob.swap_pubkey(COIN_BTC, DeriveScope::LEGACY, 0).unwrap(),
        };
        (params, alice, bob, s)
    }

    #[test]
    fn htlc_composition_uses_right_keys_and_locktimes() {
        let (params, ..) = test_params();
        params.validate_structure().unwrap();
        let a = params.htlc_a().unwrap();
        let b = params.htlc_b().unwrap();
        assert_eq!(a.locktime, params.t1);
        assert_eq!(b.locktime, params.t2);
        assert_eq!(a.redeem_pubkey, params.bob_redeem_pubkey_a);
        assert_eq!(b.redeem_pubkey, params.alice_redeem_pubkey_b);
        assert_ne!(a.witness_script(), b.witness_script());

        let mut bad = params.clone();
        bad.t2 = bad.t1;
        assert!(bad.validate_structure().is_err());
    }

    #[test]
    fn redeem_and_refund_tx_shape() {
        let (params, alice, bob, s) = test_params();
        let htlc_b = params.htlc_b().unwrap();
        let outpoint = OutPoint {
            txid: bitcoin::Txid::from_str(
                "1111111111111111111111111111111111111111111111111111111111111111",
            )
            .unwrap(),
            vout: 0,
        };
        let dest = ScriptBuf::new_p2wsh(&ScriptBuf::from(vec![0x51u8]).wscript_hash());

        let redeem = build_redeem_tx(
            &htlc_b,
            outpoint,
            params.amount_b,
            dest.clone(),
            FLAT_FEE_SAT,
            &s,
            &alice
                .swap_secret_key(COIN_BTC, DeriveScope::LEGACY, 0)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(redeem.lock_time, LockTime::ZERO);
        assert_eq!(redeem.input[0].sequence.0, HTLC_SPEND_SEQUENCE);
        assert_eq!(
            redeem.output[0].value.to_sat(),
            params.amount_b - FLAT_FEE_SAT
        );
        let witness: Vec<_> = redeem.input[0].witness.iter().map(|i| i.to_vec()).collect();
        assert_eq!(witness.len(), 5);
        assert_eq!(witness[2], s.to_vec());
        assert_eq!(witness[3], vec![0x01]);
        assert_eq!(witness[4], htlc_b.witness_script().as_bytes().to_vec());
        assert_eq!(
            crate::htlc::extract_preimage(&witness, &params.hash_h),
            Some(s)
        );

        let refund = build_refund_tx(
            &htlc_b,
            outpoint,
            params.amount_b,
            dest,
            FLAT_FEE_SAT,
            &bob.swap_secret_key(COIN_BTC, DeriveScope::LEGACY, 0)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(refund.lock_time.to_consensus_u32(), params.t2);
        let witness: Vec<_> = refund.input[0].witness.iter().map(|i| i.to_vec()).collect();
        assert_eq!(witness.len(), 4);
        assert!(witness[2].is_empty());

        // Value must cover fee + dust.
        let too_small = build_redeem_tx(
            &htlc_b,
            outpoint,
            FLAT_FEE_SAT + 100,
            ScriptBuf::new(),
            FLAT_FEE_SAT,
            &s,
            &alice
                .swap_secret_key(COIN_BTC, DeriveScope::LEGACY, 0)
                .unwrap(),
        );
        assert!(too_small.is_err());
    }
}
