//! Generates the deterministic test vectors committed at
//! `spec/vectors/htlc_v1.json` (spec §13).
//!
//! Run: `cargo run -p libswap --example gen-vectors`
//!
//! Alice's seed is the standard BIP39 test mnemonic; Bob's differs in the
//! last word. Both are PUBLIC test seeds — never fund them outside regtest.

use libswap::htlc::Htlc;
use libswap::keys::{hash_preimage, swap_id, PactSeed, COIN_BTC, COIN_BTCX};
use libswap::params::{BTCX_REGTEST, BTC_REGTEST};
use serde_json::json;

const ALICE_MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const BOB_MNEMONIC: &str = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong";
const SWAP_INDEX: u32 = 0;
const T1: u32 = 1_790_000_000;
const T2: u32 = 1_789_978_400;

fn main() -> anyhow::Result<()> {
    let alice = PactSeed::from_mnemonic(ALICE_MNEMONIC, "")?;
    let bob = PactSeed::from_mnemonic(BOB_MNEMONIC, "")?;

    let s = alice.preimage(SWAP_INDEX)?;
    let h = hash_preimage(&s);

    let alice_refund_a = alice.swap_pubkey(COIN_BTCX, SWAP_INDEX)?;
    let alice_redeem_b = alice.swap_pubkey(COIN_BTC, SWAP_INDEX)?;
    let bob_redeem_a = bob.swap_pubkey(COIN_BTCX, SWAP_INDEX)?;
    let bob_refund_b = bob.swap_pubkey(COIN_BTC, SWAP_INDEX)?;

    let htlc_a = Htlc::new(h, bob_redeem_a, alice_refund_a, T1)?;
    let htlc_b = Htlc::new(h, alice_redeem_b, bob_refund_b, T2)?;

    let vectors = json!({
        "_comment": "Deterministic pact-htlc-v1 test vectors (spec §13). Public test seeds; regtest only. Regenerate with: cargo run -p libswap --example gen-vectors",
        "protocol": libswap::PROTOCOL_VERSION,
        "alice_mnemonic": ALICE_MNEMONIC,
        "bob_mnemonic": BOB_MNEMONIC,
        "swap_index": SWAP_INDEX,
        "derivation": {
            "purpose": 7228,
            "coin_btc": COIN_BTC,
            "coin_pocx": COIN_BTCX,
            "alice_identity_xonly": alice.identity_pubkey()?.to_string(),
            "bob_identity_xonly": bob.identity_pubkey()?.to_string(),
            "alice_refund_pubkey_a": alice_refund_a.to_string(),
            "alice_redeem_pubkey_b": alice_redeem_b.to_string(),
            "bob_redeem_pubkey_a": bob_redeem_a.to_string(),
            "bob_refund_pubkey_b": bob_refund_b.to_string(),
        },
        "secret": {
            "preimage_s": hex::encode(s),
            "hash_h": hex::encode(h),
            "swap_id": swap_id(&h),
        },
        "timelocks": { "t1": T1, "t2": T2 },
        "htlc_a_pocx_regtest": {
            "witness_script": hex::encode(htlc_a.witness_script().as_bytes()),
            "script_pubkey": hex::encode(htlc_a.script_pubkey().as_bytes()),
            "address": htlc_a.address(&BTCX_REGTEST)?,
        },
        "htlc_b_btc_regtest": {
            "witness_script": hex::encode(htlc_b.witness_script().as_bytes()),
            "script_pubkey": hex::encode(htlc_b.script_pubkey().as_bytes()),
            "address": htlc_b.address(&BTC_REGTEST)?,
        },
    });

    println!("{}", serde_json::to_string_pretty(&vectors)?);
    Ok(())
}
