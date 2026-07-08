//! Regression-pins libswap against the committed spec test vectors
//! (spec Â§13). If this test fails, either the protocol bytes changed
//! (requires a protocol version bump) or the vectors file is stale
//! (regenerate with `cargo run -p libswap --example gen-vectors`).

use libswap::htlc::Htlc;
use libswap::keys::{hash_preimage, swap_id, DeriveScope, PactSeed, COIN_BTC, COIN_BTCX};
use libswap::params::{BTCX_REGTEST, BTC_REGTEST};
use serde_json::Value;

#[test]
fn committed_vectors_reproduce() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../spec/vectors/htlc_v1.json"
    );
    let text = std::fs::read_to_string(path).expect("spec/vectors/htlc_v1.json must exist");
    let v: Value = serde_json::from_str(&text).unwrap();

    let alice = PactSeed::from_mnemonic(v["alice_mnemonic"].as_str().unwrap(), "").unwrap();
    let bob = PactSeed::from_mnemonic(v["bob_mnemonic"].as_str().unwrap(), "").unwrap();
    let index = v["swap_index"].as_u64().unwrap() as u32;
    let t1 = v["timelocks"]["t1"].as_u64().unwrap() as u32;
    let t2 = v["timelocks"]["t2"].as_u64().unwrap() as u32;

    let s = alice.preimage(DeriveScope::LEGACY, index).unwrap();
    let h = hash_preimage(&s);
    assert_eq!(hex::encode(s), v["secret"]["preimage_s"].as_str().unwrap());
    assert_eq!(hex::encode(h), v["secret"]["hash_h"].as_str().unwrap());
    assert_eq!(swap_id(&h), v["secret"]["swap_id"].as_str().unwrap());

    assert_eq!(
        alice.identity_pubkey().unwrap().to_string(),
        v["derivation"]["alice_identity_xonly"].as_str().unwrap()
    );

    let htlc_a = Htlc::new(
        h,
        bob.swap_pubkey(COIN_BTCX, DeriveScope::LEGACY, index)
            .unwrap(),
        alice
            .swap_pubkey(COIN_BTCX, DeriveScope::LEGACY, index)
            .unwrap(),
        t1,
    )
    .unwrap();
    let htlc_b = Htlc::new(
        h,
        alice
            .swap_pubkey(COIN_BTC, DeriveScope::LEGACY, index)
            .unwrap(),
        bob.swap_pubkey(COIN_BTC, DeriveScope::LEGACY, index)
            .unwrap(),
        t2,
    )
    .unwrap();

    for (htlc, chain, key) in [
        (&htlc_a, &BTCX_REGTEST, "htlc_a_pocx_regtest"),
        (&htlc_b, &BTC_REGTEST, "htlc_b_btc_regtest"),
    ] {
        assert_eq!(
            hex::encode(htlc.witness_script().as_bytes()),
            v[key]["witness_script"].as_str().unwrap(),
            "{key} witness_script"
        );
        assert_eq!(
            hex::encode(htlc.script_pubkey().as_bytes()),
            v[key]["script_pubkey"].as_str().unwrap(),
            "{key} script_pubkey"
        );
        assert_eq!(
            htlc.address(chain).unwrap(),
            v[key]["address"].as_str().unwrap(),
            "{key} address"
        );
    }
}
