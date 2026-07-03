//! v2 daemon swap driver (pact-htlc-v2): the reusable steps a pactd engine
//! runs to drive an adaptor swap, wired to the same abstractions the daemon
//! uses — the [`ChainBackend`](crate::chain::ChainBackend) trait, the
//! use-once nonce [`Store`], the functional MuSig2 adaptor API, and the
//! [`crate::taproot`] tx builders.
//!
//! Proven in-process, end to end, by `adaptor_swap_over_chain_backend` (a
//! mock backend stands in for live nodes): fund both legs, generate +
//! persist use-once nonces, exchange, partial-adaptor-sign, aggregate +
//! verify, Alice broadcasts the adapted leg-B redeem, Bob reads the on-chain
//! signature and recovers `t`, Bob redeems leg A; plus the refund path.
//!
//! These steps are now driven in production: [`crate::engine`] wraps them in
//! signed envelopes (`messages.rs`) with pactd RPC routing, and the live
//! two-node regtest harness exercises the full lifecycle
//! (`harness/test_adaptor_swap.py`). v2+ is enabled on every network
//! ([`crate::registry::ADAPTOR_MAINNET_ENABLED`]).

use anyhow::{Context, Result};
use musig2::secp::{MaybeScalar, Point, Scalar};
use musig2::{
    adaptor, AdaptorSignature, AggNonce, BinaryEncoding, CompactSignature, KeyAggContext,
    PartialSignature, PubNonce, SecNonce,
};

use crate::store::Store;

/// Load-or-create this party's **use-once** MuSig2 secret nonce for a signing
/// session (spec v2 §3.2). The secret nonce is persisted write-ahead, before
/// its public nonce is released; on resume the persisted nonce is reused, so
/// a replay can never produce a second signature under a fresh nonce.
pub fn session_nonce(
    store: &Store,
    swap_id: &str,
    leg: &str,
    nonce_seed: [u8; 32],
    my_point: Point,
    agg_point: Point,
    msg: &[u8; 32],
) -> Result<(SecNonce, PubNonce)> {
    if let Some(sess) = store.nonce_session(swap_id, leg)? {
        let secnonce = SecNonce::from_bytes(&sess.secnonce)
            .map_err(|_| anyhow::anyhow!("corrupt persisted secnonce for {swap_id}/{leg}"))?;
        let pubnonce = secnonce.public_nonce();
        return Ok((secnonce, pubnonce));
    }
    let secnonce = SecNonce::build_with_pubkey(nonce_seed, my_point)
        .with_aggregated_pubkey(agg_point)
        .with_message(msg)
        .build();
    store.nonce_commit(swap_id, leg, &secnonce.to_bytes())?; // write-ahead
    let pubnonce = secnonce.public_nonce();
    store.nonce_reveal(swap_id, leg)?;
    Ok((secnonce, pubnonce))
}

/// Produce this party's partial adaptor signature and record it consumed (so
/// a resume re-sends the stored signature rather than signing again).
pub fn session_partial(
    store: &Store,
    swap_id: &str,
    leg: &str,
    ctx: &KeyAggContext,
    my_scalar: Scalar,
    secnonce: SecNonce,
    aggnonce: &AggNonce,
    adaptor_point: Point,
    msg: &[u8; 32],
) -> Result<PartialSignature> {
    let partial: PartialSignature =
        adaptor::sign_partial(ctx, my_scalar, secnonce, aggnonce, adaptor_point, msg)
            .map_err(|e| anyhow::anyhow!("partial adaptor sign: {e}"))?;
    store.nonce_consume(swap_id, leg, &partial.serialize())?;
    Ok(partial)
}

/// Aggregate both partial adaptor signatures into the leg's `AdaptorSignature`.
pub fn aggregate_adaptor(
    ctx: &KeyAggContext,
    aggnonce: &AggNonce,
    adaptor_point: Point,
    partials: [PartialSignature; 2],
    msg: &[u8; 32],
) -> Result<AdaptorSignature> {
    adaptor::aggregate_partial_signatures(ctx, aggnonce, adaptor_point, partials, msg)
        .map_err(|e| anyhow::anyhow!("aggregate adaptor: {e}"))
}

/// Recover the adaptor secret `t` from an adaptor signature plus the final
/// on-chain (64-byte BIP340) signature that was broadcast — the cross-leg
/// link (spec v2 §6): once Alice's leg-B redeem is on-chain, Bob extracts `t`.
pub fn reveal_from_onchain(
    adaptor_sig: &AdaptorSignature,
    final_sig_64: &[u8],
) -> Result<MaybeScalar> {
    let compact = CompactSignature::from_bytes(final_sig_64)
        .map_err(|_| anyhow::anyhow!("bad on-chain signature bytes"))?;
    let lifted = compact
        .lift_nonce()
        .map_err(|_| anyhow::anyhow!("cannot lift nonce"))?;
    adaptor_sig
        .reveal_secret::<MaybeScalar>(&lifted)
        .context("final sig unrelated to adaptor sig")
}

// ---- hex (de)serialization of handshake material, for the swap record ----

pub fn pubnonce_hex(n: &PubNonce) -> String {
    hex::encode(n.to_bytes())
}
pub fn pubnonce_from_hex(s: &str) -> Result<PubNonce> {
    PubNonce::from_bytes(&hex::decode(s).context("pubnonce hex")?)
        .map_err(|_| anyhow::anyhow!("bad pubnonce"))
}
pub fn partial_hex(p: &PartialSignature) -> String {
    hex::encode(p.serialize())
}
pub fn partial_from_hex(s: &str) -> Result<PartialSignature> {
    MaybeScalar::from_slice(&hex::decode(s).context("partial hex")?)
        .map_err(|_| anyhow::anyhow!("bad partial signature"))
}
pub fn adaptor_sig_hex(a: &AdaptorSignature) -> String {
    hex::encode(a.to_bytes())
}
pub fn adaptor_sig_from_hex(s: &str) -> Result<AdaptorSignature> {
    AdaptorSignature::from_bytes(&hex::decode(s).context("adaptor sig hex")?)
        .map_err(|_| anyhow::anyhow!("bad adaptor signature"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adaptor_swap::{tweaked_ctx_for_leg, AdaptorSwapParams};
    use crate::chain::{ChainBackend, TxOutInfo};
    use crate::keys::{PactSeed, COIN_BTC, COIN_POCX};
    use crate::musig;
    use crate::params::{ChainParams, BTC_REGTEST, POCX_REGTEST};
    use crate::taproot::{attach_keypath_signature, build_keypath_redeem, build_refund_tx};
    use bitcoin::secp256k1::{All, Secp256k1};
    use bitcoin::{Amount, OutPoint, ScriptBuf, Transaction, TxOut, Txid};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::str::FromStr;

    const ALICE_M: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    const BOB_M: &str =
        "legal winner thank year wave sausage worth useful legal winner thank yellow";
    const T1: u32 = 1_790_000_000;
    const T2: u32 = 1_789_978_400;

    /// Minimal in-memory [`ChainBackend`] — enough to fund outputs, broadcast
    /// spends, and read back the witness of a spend (for secret reveal).
    struct MockBackend {
        params: &'static ChainParams,
        utxos: RefCell<HashMap<OutPoint, TxOut>>,
        txs: RefCell<Vec<Transaction>>,
    }
    impl MockBackend {
        fn new(params: &'static ChainParams) -> Self {
            Self {
                params,
                utxos: RefCell::new(HashMap::new()),
                txs: RefCell::new(Vec::new()),
            }
        }
        /// Fund an outpoint paying `spk` (stands in for the funder's wallet send).
        fn fund(&self, outpoint: OutPoint, value: u64, spk: ScriptBuf) {
            self.utxos.borrow_mut().insert(
                outpoint,
                TxOut {
                    value: Amount::from_sat(value),
                    script_pubkey: spk,
                },
            );
        }
    }
    impl ChainBackend for MockBackend {
        fn params(&self) -> &ChainParams {
            self.params
        }
        fn verify_chain(&self) -> Result<()> {
            Ok(())
        }
        fn broadcast(&self, tx: &Transaction) -> Result<Txid> {
            let txid = tx.compute_txid();
            self.txs.borrow_mut().push(tx.clone());
            Ok(txid)
        }
        fn get_txout(
            &self,
            outpoint: &OutPoint,
            expected_spk: &ScriptBuf,
        ) -> Result<Option<TxOutInfo>> {
            Ok(self
                .utxos
                .borrow()
                .get(outpoint)
                .filter(|o| &o.script_pubkey == expected_spk)
                .map(|o| TxOutInfo {
                    value_sat: o.value.to_sat(),
                    script_pubkey_hex: hex::encode(o.script_pubkey.as_bytes()),
                    confirmations: 1,
                }))
        }
        fn find_funding(&self, spk: &ScriptBuf) -> Result<Option<(OutPoint, TxOutInfo)>> {
            Ok(self
                .utxos
                .borrow()
                .iter()
                .find(|(_, o)| &o.script_pubkey == spk)
                .map(|(op, o)| {
                    (
                        *op,
                        TxOutInfo {
                            value_sat: o.value.to_sat(),
                            script_pubkey_hex: hex::encode(o.script_pubkey.as_bytes()),
                            confirmations: 1,
                        },
                    )
                }))
        }
        fn find_vout(&self, _txid: &str, _spk_hex: &str) -> Result<u32> {
            Ok(0)
        }
        fn find_spend_witness(
            &self,
            outpoint: &OutPoint,
            _watch: &ScriptBuf,
            _from: u64,
        ) -> Result<Option<Vec<Vec<u8>>>> {
            for tx in self.txs.borrow().iter() {
                if tx.input.iter().any(|i| &i.previous_output == outpoint) {
                    return Ok(Some(tx.input[0].witness.to_vec()));
                }
            }
            Ok(None)
        }
        fn tip_height(&self) -> Result<u64> {
            Ok(100)
        }
        fn tip_median_time(&self) -> Result<u64> {
            Ok(1_700_000_000)
        }
        fn tx_confirmations(&self, _txid: &str, _spk: Option<&ScriptBuf>) -> Result<u64> {
            Ok(1)
        }
        fn fee_rate_for(&self, _conf_target: u16, _conservative: bool) -> Result<u64> {
            Ok(1)
        }
        fn wallet_new_address(&self) -> Result<String> {
            unimplemented!()
        }
        fn wallet_balance(&self) -> Result<u64> {
            Ok(0)
        }
        fn wallet_send(&self, _address: &str, _amount_sat: u64, _conf_target: u16) -> Result<String> {
            unimplemented!()
        }
    }

    fn temp_store(tag: &str) -> (Store, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("libswap-ae-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        (Store::init(&dir, None).unwrap(), dir)
    }
    fn op(tag: u8) -> OutPoint {
        OutPoint {
            txid: Txid::from_str(&format!("{:02x}", tag).repeat(32)).unwrap(),
            vout: 0,
        }
    }
    fn dest() -> ScriptBuf {
        ScriptBuf::new_p2wsh(&ScriptBuf::from(vec![0x51u8]).wscript_hash())
    }

    /// Run one party's nonce+partial for a leg session via the store-backed
    /// driver, returning (pubnonce, then-callable partial closure inputs).
    #[allow(clippy::too_many_arguments)]
    fn full_swap() {
        let secp: Secp256k1<All> = Secp256k1::new();
        let alice = PactSeed::from_mnemonic(ALICE_M, "").unwrap();
        let bob = PactSeed::from_mnemonic(BOB_M, "").unwrap();
        let (a_store, a_dir) = temp_store("alice");
        let (b_store, b_dir) = temp_store("bob");
        let i = 0u32;
        let swap_id = "deadbeefdeadbeef";

        let params = AdaptorSwapParams {
            amount_a: 50_000_000,
            amount_b: 100_000,
            t1: T1,
            t2: T2,
            alice_swap_a: alice.swap_pubkey(COIN_POCX, i).unwrap(),
            alice_swap_b: alice.swap_pubkey(COIN_BTC, i).unwrap(),
            bob_swap_a: bob.swap_pubkey(COIN_POCX, i).unwrap(),
            bob_swap_b: bob.swap_pubkey(COIN_BTC, i).unwrap(),
            alice_refund_a: alice.refund_xonly_pubkey(COIN_POCX, i).unwrap(),
            bob_refund_b: bob.refund_xonly_pubkey(COIN_BTC, i).unwrap(),
            adaptor_point: alice.adaptor_point(i).unwrap(),
        };
        params.validate_structure().unwrap();
        let leg_a = params.leg_a(&secp).unwrap();
        let leg_b = params.leg_b(&secp).unwrap();

        // ---- Funding (both legs) registered on each chain's backend ----
        let pocx = MockBackend::new(&POCX_REGTEST);
        let btc = MockBackend::new(&BTC_REGTEST);
        pocx.fund(
            op(0xaa),
            params.amount_a,
            leg_a.script_pubkey(&secp).unwrap(),
        );
        btc.fund(
            op(0xbb),
            params.amount_b,
            leg_b.script_pubkey(&secp).unwrap(),
        );
        // The funder confirms its output exists before proceeding.
        assert!(pocx
            .get_txout(&op(0xaa), &leg_a.script_pubkey(&secp).unwrap())
            .unwrap()
            .is_some());

        let t_scalar = musig::seckey_to_scalar(&alice.adaptor_secret(i).unwrap()).unwrap();
        let t_point = musig::pubkey_to_point(&params.adaptor_point).unwrap();

        // ---- Leg B redeem session (Alice claims B; funder=Bob idx0) ----
        let (redeem_b, sighash_b) =
            build_keypath_redeem(&secp, &leg_b, op(0xbb), params.amount_b, dest(), 1_000).unwrap();
        let ctx_b =
            tweaked_ctx_for_leg(&secp, &leg_b, &params.bob_swap_b, &params.alice_swap_b).unwrap();
        let agg_b: Point = ctx_b.aggregated_pubkey();
        let b_pt_b = musig::pubkey_to_point(&params.bob_swap_b).unwrap();
        let a_pt_b = musig::pubkey_to_point(&params.alice_swap_b).unwrap();
        // Bob's and Alice's use-once nonces (each persisted in their own store).
        let (b_sn, b_pn) = session_nonce(
            &b_store, swap_id, "redeem_b", [0x01; 32], b_pt_b, agg_b, &sighash_b,
        )
        .unwrap();
        let (a_sn, a_pn) = session_nonce(
            &a_store, swap_id, "redeem_b", [0x02; 32], a_pt_b, agg_b, &sighash_b,
        )
        .unwrap();
        // Resume must hand back the SAME nonce (use-once), not a fresh one.
        let (_b_sn2, b_pn2) = session_nonce(
            &b_store, swap_id, "redeem_b", [0xff; 32], b_pt_b, agg_b, &sighash_b,
        )
        .unwrap();
        assert_eq!(
            b_pn.serialize(),
            b_pn2.serialize(),
            "resume reused the persisted nonce"
        );
        let aggnonce_b = AggNonce::sum([b_pn.clone(), a_pn.clone()]);
        let b_part = session_partial(
            &b_store,
            swap_id,
            "redeem_b",
            &ctx_b,
            musig::seckey_to_scalar(&bob.swap_secret_key(COIN_BTC, i).unwrap()).unwrap(),
            b_sn,
            &aggnonce_b,
            t_point,
            &sighash_b,
        )
        .unwrap();
        let a_part = session_partial(
            &a_store,
            swap_id,
            "redeem_b",
            &ctx_b,
            musig::seckey_to_scalar(&alice.swap_secret_key(COIN_BTC, i).unwrap()).unwrap(),
            a_sn,
            &aggnonce_b,
            t_point,
            &sighash_b,
        )
        .unwrap();
        let sig_b =
            aggregate_adaptor(&ctx_b, &aggnonce_b, t_point, [b_part, a_part], &sighash_b).unwrap();
        adaptor::verify_single(agg_b, &sig_b, sighash_b, t_point).unwrap();

        // ---- Leg A redeem session (Bob claims A; funder=Alice idx0) ----
        let (redeem_a, sighash_a) =
            build_keypath_redeem(&secp, &leg_a, op(0xaa), params.amount_a, dest(), 1_000).unwrap();
        let ctx_a =
            tweaked_ctx_for_leg(&secp, &leg_a, &params.alice_swap_a, &params.bob_swap_a).unwrap();
        let agg_a: Point = ctx_a.aggregated_pubkey();
        let a_pt_a = musig::pubkey_to_point(&params.alice_swap_a).unwrap();
        let b_pt_a = musig::pubkey_to_point(&params.bob_swap_a).unwrap();
        let (a_sn_a, a_pn_a) = session_nonce(
            &a_store, swap_id, "redeem_a", [0x03; 32], a_pt_a, agg_a, &sighash_a,
        )
        .unwrap();
        let (b_sn_a, b_pn_a) = session_nonce(
            &b_store, swap_id, "redeem_a", [0x04; 32], b_pt_a, agg_a, &sighash_a,
        )
        .unwrap();
        let aggnonce_a = AggNonce::sum([a_pn_a.clone(), b_pn_a.clone()]);
        let a_part_a = session_partial(
            &a_store,
            swap_id,
            "redeem_a",
            &ctx_a,
            musig::seckey_to_scalar(&alice.swap_secret_key(COIN_POCX, i).unwrap()).unwrap(),
            a_sn_a,
            &aggnonce_a,
            t_point,
            &sighash_a,
        )
        .unwrap();
        let b_part_a = session_partial(
            &b_store,
            swap_id,
            "redeem_a",
            &ctx_a,
            musig::seckey_to_scalar(&bob.swap_secret_key(COIN_POCX, i).unwrap()).unwrap(),
            b_sn_a,
            &aggnonce_a,
            t_point,
            &sighash_a,
        )
        .unwrap();
        let sig_a = aggregate_adaptor(
            &ctx_a,
            &aggnonce_a,
            t_point,
            [a_part_a, b_part_a],
            &sighash_a,
        )
        .unwrap();

        // ---- Alice adapts + broadcasts leg-B redeem (reveals t on-chain) ----
        let final_b = sig_b.adapt::<musig2::LiftedSignature>(t_scalar).unwrap();
        let mut redeem_b = redeem_b;
        attach_keypath_signature(
            &mut redeem_b,
            crate::adaptor_swap::lifted_to_bitcoin(&final_b).unwrap(),
        );
        btc.broadcast(&redeem_b).unwrap();

        // ---- Bob reads the on-chain witness and recovers t ----
        let witness = btc
            .find_spend_witness(&op(0xbb), &leg_b.script_pubkey(&secp).unwrap(), 0)
            .unwrap()
            .unwrap();
        let revealed = reveal_from_onchain(&sig_b, &witness[0]).unwrap();
        assert_eq!(
            revealed,
            MaybeScalar::Valid(t_scalar),
            "Bob recovers Alice's secret from the chain"
        );

        // ---- Bob adapts leg A with the recovered secret + redeems ----
        let final_a = sig_a.adapt::<musig2::LiftedSignature>(revealed).unwrap();
        let mut redeem_a = redeem_a;
        attach_keypath_signature(
            &mut redeem_a,
            crate::adaptor_swap::lifted_to_bitcoin(&final_a).unwrap(),
        );
        let txid_a = pocx.broadcast(&redeem_a).unwrap();
        assert_eq!(txid_a, redeem_a.compute_txid());

        // ---- Refund path is independently broadcastable (single-key) ----
        let refund_kp = alice
            .refund_secret_key(COIN_POCX, i)
            .unwrap()
            .keypair(&secp);
        let refund = build_refund_tx(
            &secp,
            &leg_a,
            op(0xaa),
            params.amount_a,
            dest(),
            1_000,
            &refund_kp,
        )
        .unwrap();
        pocx.broadcast(&refund).unwrap();

        std::fs::remove_dir_all(&a_dir).ok();
        std::fs::remove_dir_all(&b_dir).ok();
    }

    #[test]
    fn adaptor_swap_over_chain_backend() {
        full_swap();
    }
}
