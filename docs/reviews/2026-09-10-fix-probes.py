"""Isolated regression probes; production source is read, never edited."""
import pathlib, tempfile, subprocess, os

repo = pathlib.Path(__file__).resolve().parents[2]
scratch = pathlib.Path(tempfile.mkdtemp(prefix='satchel-fix-verification-'))
(scratch/'src').mkdir()
(scratch/'Cargo.toml').write_text('''[package]
name = "satchel-fix-probes"
version = "0.0.0"
edition = "2021"
[dependencies]
libswap = { path = "REPO/pact/libswap" }
bitcoin = { version = "0.32", features = ["serde", "rand-std"] }
anyhow = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
'''.replace('REPO', repo.as_posix()))
(scratch/'Cargo.lock').write_bytes((repo/'pact/Cargo.lock').read_bytes())
source = (repo/'pact/libswap/src/reconstruct.rs').read_text(encoding='utf-8')
extra = r'''
    #[test]
    fn review_wrong_branch_key_still_passes() {
        let (fx, secp) = v1_fixture();
        let spk = fx.htlc.script_pubkey();
        let value = 100_000;
        let f = funding_tx(&spk, value);
        // Redeem key signs the REFUND branch. The real HTLC requires the
        // other key there, but the added validator searches both branches.
        let invalid = sign_v1(&secp, &fx, &fx.redeem, value, &[vec![]], spend_tx(&f, &[]));
        let w: Vec<Vec<u8>> = invalid.input[0].witness.iter().map(<[u8]>::to_vec).collect();
        assert!(witness_authentic(&spk, value, &invalid, 0));
        assert_eq!(classify_v1_spend(&w, &fx.htlc.hash_h), SpendKind::Refund);
        println!("CONFIRMED: refund branch signed by redeem key accepted as authentic Refund");
    }
    #[test]
    fn review_claimed_height_changes_finality_without_new_signature() {
        let (fx, secp) = v1_fixture();
        let spk = fx.htlc.script_pubkey();
        let value = 100_000;
        let f = funding_tx(&spk, value);
        let spend = sign_v1(&secp, &fx, &fx.redeem, value,
            &[fx.preimage.to_vec(), vec![1]], spend_tx(&f, &[]));
        for height in [0, 95] {
            let backend = MockBackend { history: Some(vec![(f.compute_txid().to_string(), 90),
                (spend.compute_txid().to_string(), height)]),
                txs: vec![f.clone(), spend.clone()], tip: 100, spend: None };
            let class = classify_leg(&backend, &spk, value,
                &|w| classify_v1_spend(w, &fx.htlc.hash_h)).unwrap().unwrap();
            let LegClass::Spent(s) = class else { panic!("expected spend") };
            assert_eq!(s.spend_confs, if height == 0 {0} else {6});
            println!("same signed transaction, provider height={height}, accepted confirmations={}", s.spend_confs);
        }
    }
'''
pos = source.rfind('}')
(scratch/'src/reconstruct.rs').write_text(source[:pos]+extra+source[pos:], encoding='utf-8')
desktop = (repo/'satchel/src/main.rs').read_text(encoding='utf-8')
predicate = desktop.split('fn swap_needs_coin(')[1].split('\n///')[0]
predicate = 'fn swap_needs_coin(' + predicate
(scratch/'src/lib.rs').write_text('pub use libswap::{chain, htlc, params, taproot};\nmod reconstruct;\n'+predicate+r'''
#[test]
fn review_coin_guard_on_actual_wire_shape() {
    let chain = libswap::messages::ChainRef { coin_id: "btc".into(), network: params::Network::Regtest };
    let record = serde_json::json!({"swap_id":"review", "state":"signed", "settled":false,
        "source":"local", "chain_a":chain, "chain_b":chain});
    println!("actual serialized record: {record}");
    assert!(!swap_needs_coin(&record, "btc"));
    println!("CONFIRMED: coin removal guard misses live swap with actual ChainRef serialization");
}
''', encoding='utf-8')
env = dict(os.environ, CARGO_TARGET_DIR=str(repo/'pact/target'))
print('Scratch:', scratch, flush=True)
raise SystemExit(subprocess.run(['cargo','test','--offline','review_','--','--nocapture'], cwd=scratch, env=env).returncode)
