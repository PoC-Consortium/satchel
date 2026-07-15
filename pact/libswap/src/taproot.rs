//! v2 Taproot output + spend transactions (pact-htlc-v2, spec v2 §4–§5).
//!
//! Each swap leg is one P2TR output whose
//! - **key-path** is the 2-of-2 MuSig2 aggregate (the cooperative redeem —
//!   built and signed by [`crate::musig`] / the swap state machine in M5),
//! - **script-path** is a single tapleaf, a single-key CLTV refund:
//!   `<T> OP_CLTV OP_DROP <refund_xonly> OP_CHECKSIG`.
//!
//! This module builds the output (address/scriptPubKey), the refund tx
//! (fully signed here — single-key Schnorr, no MuSig2), and the key-path
//! redeem skeleton + its BIP341 sighash (the message the MuSig2 session
//! signs); the aggregate signature is attached with
//! [`attach_keypath_signature`].

use anyhow::{ensure, Context, Result};
use bitcoin::absolute::LockTime;
use bitcoin::hashes::Hash;
use bitcoin::opcodes::all::{OP_CHECKSIG, OP_CLTV, OP_DROP};
use bitcoin::script::Builder;
use bitcoin::secp256k1::schnorr;
use bitcoin::secp256k1::{Keypair, Message, Secp256k1, Signing, Verification};
use bitcoin::sighash::{Prevouts, SighashCache};
use bitcoin::taproot::{
    LeafVersion, Signature as TaprootSignature, TapLeafHash, TaprootBuilder, TaprootSpendInfo,
};
use bitcoin::transaction::Version;
use bitcoin::{
    Amount, OutPoint, ScriptBuf, Sequence, TapSighashType, Transaction, TxIn, TxOut, Witness,
    XOnlyPublicKey,
};

use crate::htlc::MIN_TIME_LOCKTIME;
use crate::params::ChainParams;
use crate::swap::{dust_threshold, HTLC_SPEND_SEQUENCE};

/// Worst-case vsizes of the v2 1-in/1-out spends (P2TR input, one P2WSH-sized
/// output), used to turn a feerate into an absolute fee before the witness
/// exists. The key-path spend carries one 64-byte Schnorr signature; the
/// script-path refund additionally reveals the leaf script + control block.
pub const KEYPATH_REDEEM_VSIZE: u64 = 111;
pub const SCRIPTPATH_REFUND_VSIZE: u64 = 140;

/// One v2 Taproot swap-leg output (spec v2 §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaprootLeg {
    /// MuSig2 aggregate of both parties' swap keys — the untweaked internal
    /// key, i.e. the cooperative key-path spender.
    pub internal_key: XOnlyPublicKey,
    /// The funder's refund key (branch `3'`), sole signer of the CLTV leaf.
    pub refund_key: XOnlyPublicKey,
    /// Absolute Unix-time locktime `T` for the refund leaf (CLTV, MTP-based).
    pub locktime: u32,
}

impl TaprootLeg {
    pub fn new(
        internal_key: XOnlyPublicKey,
        refund_key: XOnlyPublicKey,
        locktime: u32,
    ) -> Result<Self> {
        ensure!(
            locktime >= MIN_TIME_LOCKTIME,
            "locktime {locktime} is a block height; v2 requires Unix-time locktimes (spec v2 §4)"
        );
        Ok(Self {
            internal_key,
            refund_key,
            locktime,
        })
    }

    /// The refund tapleaf script: `<T> OP_CLTV OP_DROP <refund_xonly> OP_CHECKSIG`.
    pub fn refund_script(&self) -> ScriptBuf {
        Builder::new()
            .push_int(i64::from(self.locktime))
            .push_opcode(OP_CLTV)
            .push_opcode(OP_DROP)
            .push_x_only_key(&self.refund_key)
            .push_opcode(OP_CHECKSIG)
            .into_script()
    }

    /// The Taproot spend info (output key + merkle root + control blocks),
    /// internal key tweaked by the single refund tapleaf.
    pub fn spend_info<C: Verification>(&self, secp: &Secp256k1<C>) -> Result<TaprootSpendInfo> {
        TaprootBuilder::new()
            .add_leaf(0, self.refund_script())
            .context("add refund leaf")?
            .finalize(secp, self.internal_key)
            .map_err(|_| anyhow::anyhow!("taproot finalize failed"))
    }

    /// The P2TR scriptPubKey (`OP_1 <32-byte output key>`).
    pub fn script_pubkey<C: Verification>(&self, secp: &Secp256k1<C>) -> Result<ScriptBuf> {
        Ok(ScriptBuf::new_p2tr_tweaked(
            self.spend_info(secp)?.output_key(),
        ))
    }

    /// The funding output (prevout) for this leg at a given value.
    pub fn funding_txout<C: Verification>(
        &self,
        secp: &Secp256k1<C>,
        value_sat: u64,
    ) -> Result<TxOut> {
        Ok(TxOut {
            value: Amount::from_sat(value_sat),
            script_pubkey: self.script_pubkey(secp)?,
        })
    }

    /// bech32m address under the given chain's HRP (spec v2 §4).
    pub fn address<C: Verification>(
        &self,
        secp: &Secp256k1<C>,
        chain: &ChainParams,
    ) -> Result<String> {
        chain.p2tr_address(&self.spend_info(secp)?.output_key().to_x_only_public_key())
    }
}

/// 1-in/1-out tx skeleton spending `funding`, sweeping value−fee to `dest`.
///
/// `sequence` is part of the sighash, so for the CO-SIGNED redeem both
/// parties must pass the same value — see the two callers for the choice.
fn spend_skeleton(
    funding: OutPoint,
    value_sat: u64,
    fee_sat: u64,
    dest: ScriptBuf,
    lock_time: LockTime,
    sequence: Sequence,
) -> Result<Transaction> {
    let dust = dust_threshold(&dest);
    ensure!(
        value_sat > fee_sat + dust,
        "leg value {value_sat} cannot cover fee {fee_sat} plus dust {dust} (spec v2 §5)"
    );
    Ok(Transaction {
        version: Version::TWO,
        lock_time,
        input: vec![TxIn {
            previous_output: funding,
            script_sig: ScriptBuf::new(),
            sequence,
            witness: Witness::default(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(value_sat - fee_sat),
            script_pubkey: dest,
        }],
    })
}

/// Build the cooperative **key-path redeem** tx (unsigned) and return it with
/// the BIP341 key-path sighash that the MuSig2 adaptor session must sign. The
/// aggregate signature is later attached with [`attach_keypath_signature`].
/// nLockTime 0 (no timelock on the happy path).
pub fn build_keypath_redeem<C: Verification>(
    secp: &Secp256k1<C>,
    leg: &TaprootLeg,
    funding: OutPoint,
    value_sat: u64,
    dest: ScriptBuf,
    fee_sat: u64,
) -> Result<(Transaction, [u8; 32])> {
    // NON-replaceable (no BIP125 signal, rc10): the redeem's fee is committed
    // into the adaptor signature — nothing can ever RBF it (a stuck redeem is
    // CPFP'd), so signal that honestly. The sequence is in the MuSig2 sighash,
    // so both parties must build it identically: rc9 peers built 0xFFFFFFFD →
    // rc9↔rc10 v2 swaps fail partial-sig verification at the handshake (a
    // clean pre-funding abort; alpha, no compat shim on purpose).
    let tx = spend_skeleton(
        funding,
        value_sat,
        fee_sat,
        dest,
        LockTime::ZERO,
        Sequence::ENABLE_LOCKTIME_NO_RBF,
    )?;
    let prevout = leg.funding_txout(secp, value_sat)?;
    let sighash = SighashCache::new(&tx)
        .taproot_key_spend_signature_hash(0, &Prevouts::All(&[prevout]), TapSighashType::Default)
        .context("key-path sighash")?;
    Ok((tx, sighash.to_byte_array()))
}

/// Attach a final (adapted) aggregate Schnorr signature to a key-path redeem
/// tx built by [`build_keypath_redeem`]. Witness is a single 64-byte sig
/// (SIGHASH_DEFAULT).
pub fn attach_keypath_signature(tx: &mut Transaction, sig: schnorr::Signature) {
    let ts = TaprootSignature {
        signature: sig,
        sighash_type: TapSighashType::Default,
    };
    tx.input[0].witness = Witness::from_slice(&[ts.to_vec()]);
}

/// Build and fully sign the **script-path refund** tx — single-key Schnorr
/// over the CLTV leaf; NO MuSig2, NO interactive nonce (the unattended
/// auto-refund path, spec v2 §5). nLockTime = `T`; valid only once MTP ≥ T.
pub fn build_refund_tx<C: Signing + Verification>(
    secp: &Secp256k1<C>,
    leg: &TaprootLeg,
    funding: OutPoint,
    value_sat: u64,
    dest: ScriptBuf,
    fee_sat: u64,
    refund_keypair: &Keypair,
) -> Result<Transaction> {
    ensure!(
        refund_keypair.x_only_public_key().0 == leg.refund_key,
        "refund keypair does not match the leg's refund key"
    );
    // The refund is SINGLE-signer (the funder's CLTV leaf), so its sequence
    // is a local choice — keep the RBF signal: the funder can always rebuild
    // a stuck refund at a higher fee. (CLTV needs a non-final sequence, which
    // 0xFFFFFFFD satisfies.)
    let mut tx = spend_skeleton(
        funding,
        value_sat,
        fee_sat,
        dest,
        LockTime::from_consensus(leg.locktime),
        Sequence::from_consensus(HTLC_SPEND_SEQUENCE),
    )?;
    let script = leg.refund_script();
    let spend_info = leg.spend_info(secp)?;
    let control_block = spend_info
        .control_block(&(script.clone(), LeafVersion::TapScript))
        .context("control block for refund leaf")?;
    let prevout = leg.funding_txout(secp, value_sat)?;
    let leaf_hash = TapLeafHash::from_script(&script, LeafVersion::TapScript);
    let sighash = SighashCache::new(&tx)
        .taproot_script_spend_signature_hash(
            0,
            &Prevouts::All(&[prevout]),
            leaf_hash,
            TapSighashType::Default,
        )
        .context("script-path sighash")?;
    let sig = secp.sign_schnorr(
        &Message::from_digest(sighash.to_byte_array()),
        refund_keypair,
    );
    let ts = TaprootSignature {
        signature: sig,
        sighash_type: TapSighashType::Default,
    };

    let mut witness = Witness::new();
    witness.push(ts.to_vec()); // signature
    witness.push(script.as_bytes()); // the tapleaf script
    witness.push(control_block.serialize()); // control block
    tx.input[0].witness = witness;
    Ok(tx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::{DeriveScope, PactSeed, COIN_BTC};
    use crate::musig;
    use crate::params::BTC_REGTEST;
    use bitcoin::key::TapTweak;
    use bitcoin::secp256k1::PublicKey;
    use std::str::FromStr;

    const MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    const T_LEG: u32 = 1_780_000_000;

    /// Build a leg whose internal key is the 2-of-2 MuSig2 aggregate of two
    /// real Pact swap keys, with a Pact refund key.
    fn sample_leg() -> (TaprootLeg, Secp256k1<bitcoin::secp256k1::All>) {
        let secp = Secp256k1::new();
        let alice = PactSeed::from_mnemonic(MNEMONIC, "").unwrap();
        let bob = PactSeed::from_mnemonic(MNEMONIC, "deviation").unwrap();
        let pk_a: PublicKey = alice.swap_pubkey(COIN_BTC, DeriveScope::LEGACY, 0).unwrap();
        let pk_b: PublicKey = bob.swap_pubkey(COIN_BTC, DeriveScope::LEGACY, 0).unwrap();
        let (_, agg_xonly) = musig::aggregate_2of2(&pk_a, &pk_b).unwrap();
        let refund = alice
            .refund_xonly_pubkey(COIN_BTC, DeriveScope::LEGACY, 0)
            .unwrap();
        (TaprootLeg::new(agg_xonly, refund, T_LEG).unwrap(), secp)
    }

    fn dummy_outpoint() -> OutPoint {
        OutPoint {
            txid: bitcoin::Txid::from_str(&"11".repeat(32)).unwrap(),
            vout: 0,
        }
    }

    fn dummy_dest() -> ScriptBuf {
        ScriptBuf::new_p2wsh(&ScriptBuf::from(vec![0x51u8]).wscript_hash())
    }

    #[test]
    fn output_is_p2tr_with_valid_address() {
        let (leg, secp) = sample_leg();
        assert!(leg.script_pubkey(&secp).unwrap().is_p2tr());
        let addr = leg.address(&secp, &BTC_REGTEST).unwrap();
        assert!(addr.starts_with("bcrt1p"), "p2tr address: {addr}");
        // The refund leaf has a usable control block.
        let info = leg.spend_info(&secp).unwrap();
        assert!(info
            .control_block(&(leg.refund_script(), LeafVersion::TapScript))
            .is_some());
        assert!(info.merkle_root().is_some());
    }

    #[test]
    fn height_locktime_rejected() {
        let (leg, _) = sample_leg();
        assert!(TaprootLeg::new(leg.internal_key, leg.refund_key, 800_000).is_err());
    }

    #[test]
    fn refund_tx_is_signed_and_verifies() {
        let (leg, secp) = sample_leg();
        let alice = PactSeed::from_mnemonic(MNEMONIC, "").unwrap();
        let refund_kp = alice
            .refund_secret_key(COIN_BTC, DeriveScope::LEGACY, 0)
            .unwrap()
            .keypair(&secp);
        let value = 100_000u64;
        let fee = 1_000u64;

        let tx = build_refund_tx(
            &secp,
            &leg,
            dummy_outpoint(),
            value,
            dummy_dest(),
            fee,
            &refund_kp,
        )
        .unwrap();
        // nLockTime = T; RBF-signalling sequence; value net of fee.
        assert_eq!(tx.lock_time.to_consensus_u32(), T_LEG);
        assert_eq!(tx.input[0].sequence.0, HTLC_SPEND_SEQUENCE);
        assert_eq!(tx.output[0].value.to_sat(), value - fee);
        // Witness: [sig, script, control_block].
        let w: Vec<_> = tx.input[0].witness.iter().map(|i| i.to_vec()).collect();
        assert_eq!(w.len(), 3);
        assert_eq!(w[1], leg.refund_script().as_bytes());

        // The Schnorr signature verifies under the refund key over the
        // script-path sighash — i.e. the refund path is actually spendable.
        let prevout = leg.funding_txout(&secp, value).unwrap();
        let leaf_hash = TapLeafHash::from_script(&leg.refund_script(), LeafVersion::TapScript);
        let sighash = SighashCache::new(&tx)
            .taproot_script_spend_signature_hash(
                0,
                &Prevouts::All(&[prevout]),
                leaf_hash,
                TapSighashType::Default,
            )
            .unwrap();
        let sig = schnorr::Signature::from_slice(&w[0][..64]).unwrap();
        secp.verify_schnorr(
            &sig,
            &Message::from_digest(sighash.to_byte_array()),
            &leg.refund_key,
        )
        .expect("refund signature must verify under the refund key");
    }

    #[test]
    fn keypath_redeem_sighash_and_taproot_tweak_plumbing() {
        // The MuSig2 aggregate isn't available here, so we stand in a single
        // internal keypair, apply the BIP341 taproot tweak (as the MuSig2
        // KeyAggContext::with_taproot_tweak will at M5), sign the key-path
        // sighash, attach it, and verify under the OUTPUT key — proving the
        // key-path plumbing the aggregate signature will plug into.
        let secp = Secp256k1::new();
        let internal_kp = Keypair::from_seckey_slice(&secp, &[0x24; 32]).unwrap();
        let internal_xonly = internal_kp.x_only_public_key().0;
        let refund = PactSeed::from_mnemonic(MNEMONIC, "")
            .unwrap()
            .refund_xonly_pubkey(COIN_BTC, DeriveScope::LEGACY, 0)
            .unwrap();
        let leg = TaprootLeg::new(internal_xonly, refund, T_LEG).unwrap();
        let value = 100_000u64;

        let (mut tx, sighash) =
            build_keypath_redeem(&secp, &leg, dummy_outpoint(), value, dummy_dest(), 1_000)
                .unwrap();
        assert_eq!(tx.lock_time, LockTime::ZERO);
        // Co-signed redeem is NON-replaceable (rc10 flag-day) — the sequence
        // is in the shared MuSig2 sighash, so this value is protocol, not
        // policy: changing it breaks cross-version v2 swaps.
        assert_eq!(tx.input[0].sequence, Sequence::ENABLE_LOCKTIME_NO_RBF);

        let info = leg.spend_info(&secp).unwrap();
        let tweaked = internal_kp
            .tap_tweak(&secp, info.merkle_root())
            .to_keypair();
        let sig = secp.sign_schnorr(&Message::from_digest(sighash), &tweaked);
        attach_keypath_signature(&mut tx, sig);

        // Witness is a single 64-byte signature.
        let w: Vec<_> = tx.input[0].witness.iter().map(|i| i.to_vec()).collect();
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].len(), 64);
        // It verifies under the tweaked OUTPUT key.
        let output_key = info.output_key().to_x_only_public_key();
        secp.verify_schnorr(&sig, &Message::from_digest(sighash), &output_key)
            .expect("key-path signature must verify under the output key");
    }
}
