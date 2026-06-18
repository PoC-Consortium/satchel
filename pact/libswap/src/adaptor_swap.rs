//! v2 swap orchestration (pact-htlc-v2, spec v2 §4–§9): the MuSig2 adaptor
//! sessions that bind the two legs, on top of [`crate::taproot`] outputs and
//! [`crate::musig`] glue.
//!
//! Roles (as v1): **Alice funds leg A** (refund `T1`), **Bob funds leg B**
//! (refund `T2 < T1`), Alice holds the adaptor secret `t`. Both redeem
//! signatures are MuSig2 *adaptor* signatures under the same point `T = t·G`;
//! Alice claiming B reveals `t`, which lets Bob claim A.
//!
//! This module covers the engine-independent crypto + transaction flow. The
//! daemon message routing / chain monitoring that drives it lives in
//! `engine.rs` (it reuses v1's scheduler); the protocol is proven end to end
//! by `adaptor_swap_end_to_end` below — both parties, real Taproot outputs,
//! real BIP341 sighashes, real adaptor reveal.

use anyhow::{ensure, Context, Result};
use bitcoin::hashes::Hash;
use bitcoin::secp256k1::{PublicKey, Secp256k1, Verification};
use bitcoin::XOnlyPublicKey;
use musig2::{BinaryEncoding, KeyAggContext, LiftedSignature};
use serde::{Deserialize, Serialize};

use crate::musig;
use crate::taproot::TaprootLeg;

/// Protocol string negotiated in `init` for v2 (spec v2 §10). A party that
/// doesn't recognise it MUST `abort`.
pub const PROTOCOL_V2: &str = "pact-htlc-v2";

/// One party's view of a v2 swap lifecycle (spec v2 §9). Refund states are
/// reachable from any funded state via the clock, not via messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdaptorState {
    Created,
    Accepted,
    /// Both MuSig2 nonce sets exchanged (both redeem sessions).
    NoncesExchanged,
    /// Both adaptor signatures aggregated and verified against `T`.
    Signed,
    FundedA,
    FundedB,
    /// Alice broadcast the adapted leg-B redeem; `t` is now public.
    RedeemedB,
    Completed,
    Refunded,
    Aborted,
}

/// Everything both parties know after `accept` — enough to reconstruct both
/// Taproot legs and run both adaptor sessions deterministically (spec v2 §7).
/// All pubkeys are x-only (BIP340); `adaptor_point` is a full point.
#[derive(Debug, Clone)]
pub struct AdaptorSwapParams {
    pub amount_a: u64,
    pub amount_b: u64,
    /// Absolute Unix-time refund locktimes; `t2 < t1` (spec v2 §6).
    pub t1: u32,
    pub t2: u32,
    // MuSig2 signer keys are full points (with parity); only the *aggregate*
    // is x-only-ified for Taproot. (The refund keys below are x-only — they
    // sign their tapleaf as plain BIP340 single-sig.)
    pub alice_swap_a: PublicKey,
    pub alice_swap_b: PublicKey,
    pub bob_swap_a: PublicKey,
    pub bob_swap_b: PublicKey,
    /// Funders' refund keys (single-key CLTV leaves).
    pub alice_refund_a: XOnlyPublicKey,
    pub bob_refund_b: XOnlyPublicKey,
    /// The adaptor point `T = t·G` (Alice's secret), shared in `init`.
    pub adaptor_point: PublicKey,
}

impl AdaptorSwapParams {
    /// Structural timelock rule (spec v2 §6, inherited from v1 §7.1).
    pub fn validate_structure(&self) -> Result<()> {
        ensure!(self.t2 < self.t1, "spec v2 §6: T2 must be < T1");
        ensure!(
            self.amount_a > 0 && self.amount_b > 0,
            "amounts must be positive"
        );
        Ok(())
    }

    /// Leg A — Alice funds, Bob redeems, Alice refunds at `T1`. Internal key
    /// aggregates `[alice_swap_a, bob_swap_a]` (funder first, spec v2 §4).
    pub fn leg_a<C: Verification>(&self, secp: &Secp256k1<C>) -> Result<TaprootLeg> {
        let internal = aggregate_xonly(secp, &self.alice_swap_a, &self.bob_swap_a)?;
        TaprootLeg::new(internal, self.alice_refund_a, self.t1)
    }

    /// Leg B — Bob funds, Alice redeems, Bob refunds at `T2`. Internal key
    /// aggregates `[bob_swap_b, alice_swap_b]` (funder first).
    pub fn leg_b<C: Verification>(&self, secp: &Secp256k1<C>) -> Result<TaprootLeg> {
        let internal = aggregate_xonly(secp, &self.bob_swap_b, &self.alice_swap_b)?;
        TaprootLeg::new(internal, self.bob_refund_b, self.t2)
    }
}

/// The untweaked 2-of-2 MuSig2 aggregate of two (full) swap keys, as the
/// Taproot internal key. Key order is significant and fixed (funder first).
pub fn aggregate_xonly<C: Verification>(
    _secp: &Secp256k1<C>,
    funder: &PublicKey,
    counterparty: &PublicKey,
) -> Result<XOnlyPublicKey> {
    let ctx = key_agg_ctx(funder, counterparty)?;
    let agg: musig2::secp::Point = ctx.aggregated_pubkey();
    musig::point_to_xonly(&agg)
}

/// The MuSig2 [`KeyAggContext`] for a leg, key order `[funder, counterparty]`.
/// Aggregates the full signer keys (parity included), per BIP327.
pub fn key_agg_ctx(funder: &PublicKey, counterparty: &PublicKey) -> Result<KeyAggContext> {
    let f = musig::pubkey_to_point(funder)?;
    let c = musig::pubkey_to_point(counterparty)?;
    KeyAggContext::new([f, c]).map_err(|e| anyhow::anyhow!("key aggregation: {e}"))
}

/// Convert a finalized musig2 [`LiftedSignature`] into a `rust-bitcoin`
/// Schnorr signature (64 bytes), crossing the secp256k1 version boundary.
pub fn lifted_to_bitcoin(sig: &LiftedSignature) -> Result<bitcoin::secp256k1::schnorr::Signature> {
    let bytes: [u8; 64] = sig.to_bytes();
    bitcoin::secp256k1::schnorr::Signature::from_slice(&bytes).context("lifted sig -> bitcoin")
}

/// The taproot-tweaked [`KeyAggContext`] whose aggregated key is the leg's
/// P2TR **output** key — what the MuSig2 session must sign for so the
/// key-path signature is valid on-chain (spec v2 §4; BIP341 tweak over the
/// refund-leaf merkle root).
pub fn tweaked_ctx_for_leg<C: Verification>(
    secp: &Secp256k1<C>,
    leg: &TaprootLeg,
    funder: &PublicKey,
    counterparty: &PublicKey,
) -> Result<KeyAggContext> {
    let merkle_root = leg
        .spend_info(secp)?
        .merkle_root()
        .context("leg must have a refund tapleaf")?
        .to_byte_array();
    key_agg_ctx(funder, counterparty)?
        .with_taproot_tweak(&merkle_root)
        .map_err(|e| anyhow::anyhow!("taproot tweak: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::{PactSeed, COIN_BTC, COIN_POCX};
    use crate::params::{BTC_REGTEST, POCX_REGTEST};
    use crate::taproot::{attach_keypath_signature, build_keypath_redeem, build_refund_tx};
    use bitcoin::secp256k1::{All, Keypair, Message};
    use bitcoin::sighash::{Prevouts, SighashCache};
    use bitcoin::taproot::{LeafVersion, TapLeafHash};
    use bitcoin::{OutPoint, ScriptBuf, TapSighashType};
    use musig2::{AdaptorSignature, FirstRound, PartialSignature, SecNonceSpices, SecondRound};
    use std::str::FromStr;

    const ALICE_M: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    const BOB_M: &str =
        "legal winner thank year wave sausage worth useful legal winner thank yellow";
    const T1: u32 = 1_780_050_000;
    const T2: u32 = 1_780_020_000; // < T1

    fn outpoint(tag: u8) -> OutPoint {
        OutPoint {
            txid: bitcoin::Txid::from_str(&format!("{:02x}", tag).repeat(32)).unwrap(),
            vout: 0,
        }
    }
    fn dest() -> ScriptBuf {
        ScriptBuf::new_p2wsh(&ScriptBuf::from(vec![0x51u8]).wscript_hash())
    }

    /// Run a complete 2-of-2 MuSig2 *adaptor* session over `msg`, under
    /// `adaptor_point`, returning the aggregate `AdaptorSignature`. Drives
    /// both signers in-process (the test stands in for the wire handshake).
    #[allow(clippy::too_many_arguments)]
    fn run_adaptor_session(
        ctx: &KeyAggContext,
        msg: [u8; 32],
        adaptor_point: musig2::secp::Point,
        sk0: musig2::secp::Scalar,
        sk1: musig2::secp::Scalar,
    ) -> AdaptorSignature {
        let mut r0 = FirstRound::new(
            ctx.clone(),
            [0xa1; 32],
            0,
            SecNonceSpices::new().with_seckey(sk0).with_message(&msg),
        )
        .unwrap();
        let mut r1 = FirstRound::new(
            ctx.clone(),
            [0xb2; 32],
            1,
            SecNonceSpices::new().with_seckey(sk1).with_message(&msg),
        )
        .unwrap();
        let (n0, n1) = (r0.our_public_nonce(), r1.our_public_nonce());
        r0.receive_nonce(1, n1).unwrap();
        r1.receive_nonce(0, n0).unwrap();
        let mut s0: SecondRound<[u8; 32]> = r0.finalize_adaptor(sk0, adaptor_point, msg).unwrap();
        let mut s1: SecondRound<[u8; 32]> = r1.finalize_adaptor(sk1, adaptor_point, msg).unwrap();
        let (p0, p1): (PartialSignature, PartialSignature) =
            (s0.our_signature(), s1.our_signature());
        s0.receive_signature(1, p1).unwrap();
        s1.receive_signature(0, p0).unwrap();
        s0.finalize_adaptor::<AdaptorSignature>().unwrap()
    }

    /// The tweaked aggregate (what the MuSig2 session signs for) must equal
    /// rust-bitcoin's computed Taproot output key — the x-only parity risk
    /// flagged in V2_ADAPTOR_SWAPS.md, pinned by a test.
    #[test]
    fn musig_tweak_matches_bitcoin_output_key() {
        let secp = Secp256k1::new();
        let alice = PactSeed::from_mnemonic(ALICE_M, "").unwrap();
        let bob = PactSeed::from_mnemonic(BOB_M, "").unwrap();
        let (fa, ca) = (
            alice.swap_pubkey(COIN_BTC, 0).unwrap(),
            bob.swap_pubkey(COIN_BTC, 0).unwrap(),
        );
        let leg = TaprootLeg::new(
            aggregate_xonly(&secp, &fa, &ca).unwrap(),
            bob.refund_xonly_pubkey(COIN_BTC, 0).unwrap(),
            T2,
        )
        .unwrap();
        let ctx = tweaked_ctx_for_leg(&secp, &leg, &fa, &ca).unwrap();
        let agg_tweaked: musig2::secp::Point = ctx.aggregated_pubkey();
        let output_key = leg
            .spend_info(&secp)
            .unwrap()
            .output_key()
            .to_x_only_public_key();
        assert_eq!(agg_tweaked.serialize_xonly(), output_key.serialize());
    }

    /// Full v2 swap, end to end, in-process: build both legs, run both adaptor
    /// sessions under one `T`, Alice claims B (reveals `t`), Bob extracts `t`
    /// and claims A. Plus: the refund path is independently spendable.
    #[test]
    fn adaptor_swap_end_to_end() {
        let secp: Secp256k1<All> = Secp256k1::new();
        let alice = PactSeed::from_mnemonic(ALICE_M, "").unwrap();
        let bob = PactSeed::from_mnemonic(BOB_M, "").unwrap();
        let i = 0u32;

        // Keys: leg A on PoCX, leg B on BTC.
        let alice_swap_a = alice.swap_pubkey(COIN_POCX, i).unwrap();
        let alice_swap_b = alice.swap_pubkey(COIN_BTC, i).unwrap();
        let bob_swap_a = bob.swap_pubkey(COIN_POCX, i).unwrap();
        let bob_swap_b = bob.swap_pubkey(COIN_BTC, i).unwrap();

        let params = AdaptorSwapParams {
            amount_a: 50_000_000,
            amount_b: 100_000,
            t1: T1,
            t2: T2,
            alice_swap_a,
            alice_swap_b,
            bob_swap_a,
            bob_swap_b,
            alice_refund_a: alice.refund_xonly_pubkey(COIN_POCX, i).unwrap(),
            bob_refund_b: bob.refund_xonly_pubkey(COIN_BTC, i).unwrap(),
            adaptor_point: alice.adaptor_point(i).unwrap(),
        };
        params.validate_structure().unwrap();

        let leg_a = params.leg_a(&secp).unwrap();
        let leg_b = params.leg_b(&secp).unwrap();
        // Distinct legs with valid P2TR addresses on each chain.
        assert!(leg_a
            .address(&secp, &POCX_REGTEST)
            .unwrap()
            .starts_with("rpocx1p"));
        assert!(leg_b
            .address(&secp, &BTC_REGTEST)
            .unwrap()
            .starts_with("bcrt1p"));

        // Adaptor scalars/points (Alice's secret t and its point T).
        let t_scalar = musig::seckey_to_scalar(&alice.adaptor_secret(i).unwrap()).unwrap();
        let t_point = musig::pubkey_to_point(&params.adaptor_point).unwrap();

        // Signer scalars per leg (funder index 0, counterparty index 1).
        let a_sk_b = musig::seckey_to_scalar(&alice.swap_secret_key(COIN_BTC, i).unwrap()).unwrap();
        let b_sk_b = musig::seckey_to_scalar(&bob.swap_secret_key(COIN_BTC, i).unwrap()).unwrap();
        let a_sk_a =
            musig::seckey_to_scalar(&alice.swap_secret_key(COIN_POCX, i).unwrap()).unwrap();
        let b_sk_a = musig::seckey_to_scalar(&bob.swap_secret_key(COIN_POCX, i).unwrap()).unwrap();

        // ---- Leg B redeem: Alice claims B (Bob funder idx0, Alice idx1) ----
        let (mut redeem_b, sighash_b) = build_keypath_redeem(
            &secp,
            &leg_b,
            outpoint(0xbb),
            params.amount_b,
            dest(),
            1_000,
        )
        .unwrap();
        let ctx_b =
            tweaked_ctx_for_leg(&secp, &leg_b, &params.bob_swap_b, &params.alice_swap_b).unwrap();
        let sig_b: AdaptorSignature =
            run_adaptor_session(&ctx_b, sighash_b, t_point, b_sk_b, a_sk_b);
        // Pre-signature verifies against T but is not yet usable.
        musig2::adaptor::verify_single(
            ctx_b.aggregated_pubkey::<musig2::secp::Point>(),
            &sig_b,
            sighash_b,
            t_point,
        )
        .unwrap();
        // Alice, holding t, completes and "broadcasts".
        let final_b: LiftedSignature = sig_b.adapt(t_scalar).expect("adapt B");
        attach_keypath_signature(&mut redeem_b, lifted_to_bitcoin(&final_b).unwrap());
        verify_keypath(&secp, &redeem_b, &leg_b, params.amount_b);

        // ---- Leg A redeem: Bob claims A (Alice funder idx0, Bob idx1) ----
        let (mut redeem_a, sighash_a) = build_keypath_redeem(
            &secp,
            &leg_a,
            outpoint(0xaa),
            params.amount_a,
            dest(),
            1_000,
        )
        .unwrap();
        let ctx_a =
            tweaked_ctx_for_leg(&secp, &leg_a, &params.alice_swap_a, &params.bob_swap_a).unwrap();
        let sig_a: AdaptorSignature =
            run_adaptor_session(&ctx_a, sighash_a, t_point, a_sk_a, b_sk_a);

        // Bob never knew t — he extracts it from Alice's on-chain leg-B sig.
        let revealed: musig2::secp::MaybeScalar =
            sig_b.reveal_secret(&final_b).expect("reveal t from B");
        // Bob adapts leg A with the recovered secret and claims A.
        let final_a: LiftedSignature = sig_a.adapt(revealed).expect("adapt A with revealed t");
        attach_keypath_signature(&mut redeem_a, lifted_to_bitcoin(&final_a).unwrap());
        verify_keypath(&secp, &redeem_a, &leg_a, params.amount_a);

        // ---- Refund path (independently spendable, single-key) ----
        let alice_refund_kp: Keypair = alice
            .refund_secret_key(COIN_POCX, i)
            .unwrap()
            .keypair(&secp);
        let refund_a = build_refund_tx(
            &secp,
            &leg_a,
            outpoint(0xaa),
            params.amount_a,
            dest(),
            1_000,
            &alice_refund_kp,
        )
        .unwrap();
        assert_eq!(refund_a.lock_time.to_consensus_u32(), T1);
        assert_eq!(refund_a.input[0].witness.len(), 3);
    }

    /// Assert a key-path redeem tx's signature verifies under the leg's
    /// Taproot output key (i.e. it would be accepted on-chain).
    fn verify_keypath(
        secp: &Secp256k1<All>,
        tx: &bitcoin::Transaction,
        leg: &TaprootLeg,
        value: u64,
    ) {
        let prevout = leg.funding_txout(secp, value).unwrap();
        let sighash = SighashCache::new(tx)
            .taproot_key_spend_signature_hash(
                0,
                &Prevouts::All(&[prevout]),
                TapSighashType::Default,
            )
            .unwrap();
        let w = tx.input[0].witness.to_vec();
        assert_eq!(w[0].len(), 64);
        let sig = bitcoin::secp256k1::schnorr::Signature::from_slice(&w[0]).unwrap();
        let output_key = leg
            .spend_info(secp)
            .unwrap()
            .output_key()
            .to_x_only_public_key();
        secp.verify_schnorr(
            &sig,
            &Message::from_digest(sighash.to_byte_array()),
            &output_key,
        )
        .expect("key-path signature must verify under the output key");
        let _ = (TapLeafHash::from_script(
            &leg.refund_script(),
            LeafVersion::TapScript,
        ),);
    }
}
