//! Generates the deterministic test vectors committed at
//! `spec/vectors/htlc_v2.json` (spec v2 §11).
//!
//! Run: `cargo run -p libswap --example gen-vectors-v2 > ../spec/vectors/htlc_v2.json`
//!
//! Public test seeds — never fund them outside regtest.

use bitcoin::secp256k1::Secp256k1;
use libswap::adaptor_swap::AdaptorSwapParams;
use libswap::keys::{PactSeed, COIN_BTC, COIN_POCX};
use libswap::params::{BTC_REGTEST, POCX_REGTEST};
use serde_json::json;

const ALICE_MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const BOB_MNEMONIC: &str =
    "legal winner thank year wave sausage worth useful legal winner thank yellow";
const SWAP_INDEX: u32 = 0;
const T1: u32 = 1_790_000_000;
const T2: u32 = 1_789_978_400;
const AMOUNT_A: u64 = 50_000_000;
const AMOUNT_B: u64 = 100_000;

fn main() -> anyhow::Result<()> {
    let secp = Secp256k1::new();
    let alice = PactSeed::from_mnemonic(ALICE_MNEMONIC, "")?;
    let bob = PactSeed::from_mnemonic(BOB_MNEMONIC, "")?;
    let i = SWAP_INDEX;

    let params = AdaptorSwapParams {
        amount_a: AMOUNT_A,
        amount_b: AMOUNT_B,
        t1: T1,
        t2: T2,
        alice_swap_a: alice.swap_pubkey(COIN_POCX, i)?,
        alice_swap_b: alice.swap_pubkey(COIN_BTC, i)?,
        bob_swap_a: bob.swap_pubkey(COIN_POCX, i)?,
        bob_swap_b: bob.swap_pubkey(COIN_BTC, i)?,
        alice_refund_a: alice.refund_xonly_pubkey(COIN_POCX, i)?,
        bob_refund_b: bob.refund_xonly_pubkey(COIN_BTC, i)?,
        adaptor_point: alice.adaptor_point(i)?,
    };

    let leg_a = params.leg_a(&secp)?;
    let leg_b = params.leg_b(&secp)?;
    let out_a = leg_a.spend_info(&secp)?.output_key().to_x_only_public_key();
    let out_b = leg_b.spend_info(&secp)?.output_key().to_x_only_public_key();

    let vectors = json!({
        "_comment": "Deterministic pact-htlc-v2 test vectors (spec v2 §11). Public test seeds; regtest only. Regenerate with: cargo run -p libswap --example gen-vectors-v2",
        "protocol": "pact-htlc-v2",
        "alice_mnemonic": ALICE_MNEMONIC,
        "bob_mnemonic": BOB_MNEMONIC,
        "swap_index": SWAP_INDEX,
        "amounts": { "amount_a": AMOUNT_A, "amount_b": AMOUNT_B },
        "timelocks": { "t1": T1, "t2": T2 },
        "adaptor": {
            "secret_t": hex::encode(alice.adaptor_secret(i)?.secret_bytes()),
            "point_T": params.adaptor_point.to_string(),
        },
        "derivation": {
            "alice_swap_a": params.alice_swap_a.to_string(),
            "alice_swap_b": params.alice_swap_b.to_string(),
            "bob_swap_a": params.bob_swap_a.to_string(),
            "bob_swap_b": params.bob_swap_b.to_string(),
            "alice_refund_a": params.alice_refund_a.to_string(),
            "bob_refund_b": params.bob_refund_b.to_string(),
        },
        "leg_a_pocx_regtest": {
            "internal_key": leg_a.internal_key.to_string(),
            "output_key": out_a.to_string(),
            "refund_script": hex::encode(leg_a.refund_script().as_bytes()),
            "address": leg_a.address(&secp, &POCX_REGTEST)?,
        },
        "leg_b_btc_regtest": {
            "internal_key": leg_b.internal_key.to_string(),
            "output_key": out_b.to_string(),
            "refund_script": hex::encode(leg_b.refund_script().as_bytes()),
            "address": leg_b.address(&secp, &BTC_REGTEST)?,
        },
    });

    println!("{}", serde_json::to_string_pretty(&vectors)?);
    Ok(())
}
