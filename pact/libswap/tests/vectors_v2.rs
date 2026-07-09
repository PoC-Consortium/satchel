//! Regression-pins libswap against the committed v2 spec vectors
//! (spec v2 Â§11). If this fails, either the protocol bytes changed (requires
//! a protocol version bump) or the vectors are stale (regenerate with
//! `cargo run -p libswap --example gen-vectors-v2 > ../spec/vectors/htlc_v2.json`).

use bitcoin::secp256k1::Secp256k1;
use libswap::adaptor_swap::AdaptorSwapParams;
use libswap::keys::{DeriveScope, PactSeed, COIN_BTC, COIN_BTCX};
use libswap::params::{BTCX_REGTEST, BTC_REGTEST};
use serde_json::Value;

#[test]
fn committed_v2_vectors_reproduce() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../spec/vectors/htlc_v2.json"
    );
    let text = std::fs::read_to_string(path).expect("spec/vectors/htlc_v2.json must exist");
    let v: Value = serde_json::from_str(&text).unwrap();

    let secp = Secp256k1::new();
    let alice = PactSeed::from_mnemonic(v["alice_mnemonic"].as_str().unwrap(), "").unwrap();
    let bob = PactSeed::from_mnemonic(v["bob_mnemonic"].as_str().unwrap(), "").unwrap();
    let i = v["swap_index"].as_u64().unwrap() as u32;
    let t1 = v["timelocks"]["t1"].as_u64().unwrap() as u32;
    let t2 = v["timelocks"]["t2"].as_u64().unwrap() as u32;

    // Adaptor secret/point.
    assert_eq!(
        hex::encode(
            alice
                .adaptor_secret(DeriveScope::LEGACY, i)
                .unwrap()
                .secret_bytes()
        ),
        v["adaptor"]["secret_t"].as_str().unwrap()
    );
    assert_eq!(
        alice
            .adaptor_point(DeriveScope::LEGACY, i)
            .unwrap()
            .to_string(),
        v["adaptor"]["point_T"].as_str().unwrap()
    );

    let params = AdaptorSwapParams {
        amount_a: v["amounts"]["amount_a"].as_u64().unwrap(),
        amount_b: v["amounts"]["amount_b"].as_u64().unwrap(),
        t1,
        t2,
        alice_swap_a: alice
            .swap_pubkey(COIN_BTCX, DeriveScope::LEGACY, i)
            .unwrap(),
        alice_swap_b: alice.swap_pubkey(COIN_BTC, DeriveScope::LEGACY, i).unwrap(),
        bob_swap_a: bob.swap_pubkey(COIN_BTCX, DeriveScope::LEGACY, i).unwrap(),
        bob_swap_b: bob.swap_pubkey(COIN_BTC, DeriveScope::LEGACY, i).unwrap(),
        alice_refund_a: alice
            .refund_xonly_pubkey(COIN_BTCX, DeriveScope::LEGACY, i)
            .unwrap(),
        bob_refund_b: bob
            .refund_xonly_pubkey(COIN_BTC, DeriveScope::LEGACY, i)
            .unwrap(),
        adaptor_point: alice.adaptor_point(DeriveScope::LEGACY, i).unwrap(),
    };
    params.validate_structure().unwrap();

    // Derivation table.
    let d = &v["derivation"];
    assert_eq!(
        params.alice_swap_a.to_string(),
        d["alice_swap_a"].as_str().unwrap()
    );
    assert_eq!(
        params.bob_swap_b.to_string(),
        d["bob_swap_b"].as_str().unwrap()
    );
    assert_eq!(
        params.alice_refund_a.to_string(),
        d["alice_refund_a"].as_str().unwrap()
    );
    assert_eq!(
        params.bob_refund_b.to_string(),
        d["bob_refund_b"].as_str().unwrap()
    );

    // Legs: internal key, output key, refund script, address.
    let leg_a = params.leg_a(&secp).unwrap();
    let leg_b = params.leg_b(&secp).unwrap();
    let out_a = leg_a
        .spend_info(&secp)
        .unwrap()
        .output_key()
        .to_x_only_public_key();
    let out_b = leg_b
        .spend_info(&secp)
        .unwrap()
        .output_key()
        .to_x_only_public_key();

    let a = &v["leg_a_pocx_regtest"];
    assert_eq!(
        leg_a.internal_key.to_string(),
        a["internal_key"].as_str().unwrap()
    );
    assert_eq!(out_a.to_string(), a["output_key"].as_str().unwrap());
    assert_eq!(
        hex::encode(leg_a.refund_script().as_bytes()),
        a["refund_script"].as_str().unwrap()
    );
    assert_eq!(
        leg_a.address(&secp, &BTCX_REGTEST).unwrap(),
        a["address"].as_str().unwrap()
    );

    let b = &v["leg_b_btc_regtest"];
    assert_eq!(
        leg_b.internal_key.to_string(),
        b["internal_key"].as_str().unwrap()
    );
    assert_eq!(out_b.to_string(), b["output_key"].as_str().unwrap());
    assert_eq!(
        hex::encode(leg_b.refund_script().as_bytes()),
        b["refund_script"].as_str().unwrap()
    );
    assert_eq!(
        leg_b.address(&secp, &BTC_REGTEST).unwrap(),
        b["address"].as_str().unwrap()
    );
}
