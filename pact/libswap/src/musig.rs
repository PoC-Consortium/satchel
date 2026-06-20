//! M1 crypto spike for `pact-htlc-v2` (Route A — Taproot/MuSig2 adaptor
//! swaps). See `spec/protocol-v2.md`.
//!
//! This module proves, in isolation, the three crypto unknowns the v2
//! protocol rests on — **before** any protocol, state machine, or engine
//! wiring depends on them:
//!
//! 1. **2-of-2 MuSig2** key aggregation + a full signing session producing a
//!    single BIP340 signature that verifies under the aggregated x-only key
//!    (the cooperative Taproot *key-path* spend).
//! 2. **Schnorr adaptor signatures** — build a pre-signature under an adaptor
//!    point `T = t·G`, `adapt` it with `t` into a valid signature, and (the
//!    swap-critical step) **extract `t`** back out of the completed signature
//!    via `reveal_secret`. This is what links the two swap legs.
//! 3. **Boundary glue** — `musig2` carries its own `secp256k1 0.31`, separate
//!    from this crate's `bitcoin 0.32 → secp256k1 0.29`. The two type
//!    universes must never mix; we cross the boundary by bytes only
//!    (33-byte compressed pubkey / 32-byte x-only / 32-byte scalar).
//!
//! These primitives are now wired into the engine: [`ADAPTOR_BUILT`] is `true`
//! and [`crate::registry::select_protocol`] returns `Adaptor` for taproot pairs
//! (v2+ is enabled on every network, [`crate::registry::ADAPTOR_MAINNET_ENABLED`]).
//! The nonce seeds in *this module's tests* are fixed for determinism; the
//! production driver in [`crate::adaptor_engine`] uses fresh-random,
//! use-once-persisted nonces.
//!
//! [`ADAPTOR_BUILT`]: crate::registry::ADAPTOR_BUILT

use anyhow::{anyhow, Result};
use bitcoin::secp256k1::{PublicKey, SecretKey};
use bitcoin::XOnlyPublicKey;
use musig2::secp::{Point, Scalar};
use musig2::KeyAggContext;

/// Cross-version glue: this crate's secp256k1-0.29 `PublicKey` → `musig2`'s
/// secp256k1-0.31 / `secp` `Point`, by 33-byte compressed serialization.
pub fn pubkey_to_point(pk: &PublicKey) -> Result<Point> {
    Point::from_slice(&pk.serialize()).map_err(|e| anyhow!("pubkey→point: {e}"))
}

/// Cross-version glue: secp256k1-0.29 `SecretKey` → `secp` `Scalar`.
pub fn seckey_to_scalar(sk: &SecretKey) -> Result<Scalar> {
    Scalar::from_slice(&sk.secret_bytes()).map_err(|e| anyhow!("seckey→scalar: {e}"))
}

/// Cross-version glue: a `secp` `Point` (e.g. the aggregated key) → this
/// crate's `bitcoin::XOnlyPublicKey` (a Taproot internal/output key), by its
/// 32-byte x-only serialization.
pub fn point_to_xonly(p: &Point) -> Result<XOnlyPublicKey> {
    XOnlyPublicKey::from_slice(&p.serialize_xonly()).map_err(|e| anyhow!("point→xonly: {e}"))
}

/// Aggregate two Pact pubkeys into a 2-of-2 MuSig2 context and return the
/// aggregated key as a Taproot x-only internal key. Both parties MUST pass
/// the keys in the same order; v2 fixes the order in the spec (M2).
///
/// NOTE: the returned key is the *untweaked* internal key. The on-chain
/// Taproot output key applies the BIP341 tweak over the CLTV-refund tapleaf
/// merkle root — that lands in M4 via `KeyAggContext::with_taproot_tweak`.
pub fn aggregate_2of2(a: &PublicKey, b: &PublicKey) -> Result<(KeyAggContext, XOnlyPublicKey)> {
    let ctx = KeyAggContext::new([pubkey_to_point(a)?, pubkey_to_point(b)?])
        .map_err(|e| anyhow!("key aggregation: {e}"))?;
    let agg: Point = ctx.aggregated_pubkey();
    let xonly = point_to_xonly(&agg)?;
    Ok((ctx, xonly))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1::Secp256k1;
    use musig2::secp::MaybeScalar;
    use musig2::{
        AdaptorSignature, FirstRound, LiftedSignature, PartialSignature, SecNonceSpices,
        SecondRound,
    };

    /// Two fixed Pact keypairs (Alice = signer 0, Bob = signer 1).
    fn alice_bob() -> (SecretKey, PublicKey, SecretKey, PublicKey) {
        let secp = Secp256k1::new();
        let sk_a = SecretKey::from_slice(&[0x11; 32]).unwrap();
        let sk_b = SecretKey::from_slice(&[0x22; 32]).unwrap();
        let pk_a = sk_a.public_key(&secp);
        let pk_b = sk_b.public_key(&secp);
        (sk_a, pk_a, sk_b, pk_b)
    }

    /// The boundary glue round-trips: a Pact pubkey → `Point` → x-only bytes
    /// match the pubkey's own x-only bytes; a seckey → scalar regenerates the
    /// same point.
    #[test]
    fn boundary_glue_roundtrip() {
        let (sk_a, pk_a, ..) = alice_bob();
        let point = pubkey_to_point(&pk_a).unwrap();
        // x-only of the converted point == x-only of the original pubkey.
        assert_eq!(
            point.serialize_xonly(),
            pk_a.x_only_public_key().0.serialize()
        );
        // seckey → scalar → base point == the converted pubkey point.
        let scalar = seckey_to_scalar(&sk_a).unwrap();
        assert_eq!(scalar.base_point_mul().serialize(), point.serialize());
    }

    /// Cooperative path: a 2-of-2 MuSig2 session yields one BIP340 signature
    /// valid under the aggregated x-only key — the Taproot key-path spend.
    #[test]
    fn musig_2of2_keyspend() {
        let (sk_a, pk_a, sk_b, pk_b) = alice_bob();
        let (ctx, _xonly) = aggregate_2of2(&pk_a, &pk_b).unwrap();
        let agg: Point = ctx.aggregated_pubkey();
        let msg = [0x9a; 32]; // stand-in for a BIP341 sighash

        let (sa, sb) = (
            seckey_to_scalar(&sk_a).unwrap(),
            seckey_to_scalar(&sk_b).unwrap(),
        );
        let mut ra = FirstRound::new(
            ctx.clone(),
            [0xa1; 32],
            0,
            SecNonceSpices::new().with_seckey(sa).with_message(&msg),
        )
        .unwrap();
        let mut rb = FirstRound::new(
            ctx.clone(),
            [0xb2; 32],
            1,
            SecNonceSpices::new().with_seckey(sb).with_message(&msg),
        )
        .unwrap();
        let (na, nb) = (ra.our_public_nonce(), rb.our_public_nonce());
        ra.receive_nonce(1, nb).unwrap();
        rb.receive_nonce(0, na).unwrap();

        let mut sra: SecondRound<[u8; 32]> = ra.finalize(sa, msg).unwrap();
        let mut srb: SecondRound<[u8; 32]> = rb.finalize(sb, msg).unwrap();
        let (pa, pb): (PartialSignature, PartialSignature) =
            (sra.our_signature(), srb.our_signature());
        sra.receive_signature(1, pb).unwrap();
        srb.receive_signature(0, pa).unwrap();

        let sig: LiftedSignature = sra.finalize().unwrap();
        musig2::verify_single(agg, sig, msg).expect("aggregate keyspend sig must verify");
    }

    /// Adaptor path: the session produces an `AdaptorSignature` under point
    /// `T = t·G`; adapting with `t` yields a valid signature, and the secret
    /// `t` can be recovered from it — the mechanism that links the two legs.
    #[test]
    fn musig_2of2_adaptor_reveals_secret() {
        let (sk_a, pk_a, sk_b, pk_b) = alice_bob();
        let (ctx, _xonly) = aggregate_2of2(&pk_a, &pk_b).unwrap();
        let agg: Point = ctx.aggregated_pubkey();
        let msg = [0x9a; 32];

        // The adaptor secret t and its point T (the value that crosses chains).
        let t = Scalar::from_slice(&[0x42; 32]).unwrap();
        let adaptor_point = t.base_point_mul();

        let (sa, sb) = (
            seckey_to_scalar(&sk_a).unwrap(),
            seckey_to_scalar(&sk_b).unwrap(),
        );
        let mut ra = FirstRound::new(
            ctx.clone(),
            [0xa1; 32],
            0,
            SecNonceSpices::new().with_seckey(sa).with_message(&msg),
        )
        .unwrap();
        let mut rb = FirstRound::new(
            ctx.clone(),
            [0xb2; 32],
            1,
            SecNonceSpices::new().with_seckey(sb).with_message(&msg),
        )
        .unwrap();
        let (na, nb) = (ra.our_public_nonce(), rb.our_public_nonce());
        ra.receive_nonce(1, nb).unwrap();
        rb.receive_nonce(0, na).unwrap();

        let mut sra: SecondRound<[u8; 32]> = ra.finalize_adaptor(sa, adaptor_point, msg).unwrap();
        let mut srb: SecondRound<[u8; 32]> = rb.finalize_adaptor(sb, adaptor_point, msg).unwrap();
        let (pa, pb): (PartialSignature, PartialSignature) =
            (sra.our_signature(), srb.our_signature());
        sra.receive_signature(1, pb).unwrap();
        srb.receive_signature(0, pa).unwrap();

        let adaptor_sig: AdaptorSignature = sra.finalize_adaptor::<AdaptorSignature>().unwrap();
        // The pre-signature verifies against the adaptor point but is not yet
        // a usable BIP340 signature.
        musig2::adaptor::verify_single(agg, &adaptor_sig, msg, adaptor_point)
            .expect("adaptor pre-signature must verify");

        // Holder of t completes it; result is a valid plain signature.
        let final_sig: LiftedSignature = adaptor_sig.adapt(t).expect("adapt");
        musig2::verify_single(agg, final_sig, msg).expect("adapted sig must verify");

        // Counterparty observing final_sig recovers t — links the legs.
        let revealed: MaybeScalar = adaptor_sig.reveal_secret(&final_sig).expect("reveal");
        assert_eq!(revealed, MaybeScalar::Valid(t));
    }
}
