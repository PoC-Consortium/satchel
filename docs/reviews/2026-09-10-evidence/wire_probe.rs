#[cfg(test)]
mod t {
    use bitcoin::secp256k1::{Keypair, Secp256k1, SecretKey};
    use pact_proto::envelope::{canonical_json, sign, signing_digest, verify, Envelope};
    use pact_proto::seal::{open_envelope, seal_envelope};
    use serde_json::Value;

    fn kp(b: u8) -> Keypair {
        Keypair::from_secret_key(&Secp256k1::new(), &SecretKey::from_slice(&[b; 32]).unwrap())
    }
    fn xonly(k: &Keypair) -> String { k.x_only_public_key().0.to_string() }
    fn env(body: Value) -> Envelope {
        Envelope { v: 1, msg_type: "abort".into(), swap_id: "0011223344556677".into(), from: String::new(), body, sig: String::new() }
    }

    #[test]
    fn nonce_length_panics_at_head() {
        let alice = kp(7); let bob = kp(8);
        let mut e = env(serde_json::json!({"reason":"x"}));
        sign(&mut e, &alice).unwrap();
        let blob = seal_envelope(&xonly(&bob), &e).unwrap();
        let parts: Vec<&str> = blob.split(':').collect();
        for len in [0usize, 1, 11, 13, 24] {
            let nonce = "ab".repeat(len);
            let bad = format!("{}:{}:{}:{}", parts[0], parts[1], nonce, parts[3]);
            let r = std::panic::catch_unwind(|| open_envelope(&bob, &bad).is_err());
            println!("nonce len {len:>2} bytes -> {}", if r.is_err() { "PANIC" } else { "Err (recoverable)" });
        }
        // 12-byte wrong nonce is a clean error
        let bad = format!("{}:{}:{}:{}", parts[0], parts[1], "00".repeat(12), parts[3]);
        assert!(open_envelope(&bob, &bad).is_err());
        // odd hex / bad epk are clean errors
        assert!(open_envelope(&bob, "PACTSEALED1:zz:00:00").is_err());
        assert!(open_envelope(&bob, "PACTSEALED1:00:00:00").is_err());
        assert!(open_envelope(&bob, "PACTSEALED1").is_err());
    }

    #[test]
    fn canonical_json_edges() {
        // duplicate keys: serde_json last-wins, silently
        let v: Value = serde_json::from_str(r#"{"a":1,"a":2}"#).unwrap();
        println!("dup keys -> {}", canonical_json(&v).unwrap());
        // > u64 max -> float -> rejected
        let v: Value = serde_json::from_str(r#"{"a":18446744073709551616}"#).unwrap();
        println!("u64 overflow -> {:?}", canonical_json(&v).map_err(|e| e.to_string()));
        let v: Value = serde_json::from_str(r#"{"a":-0}"#).unwrap();
        println!("-0 -> {:?}", canonical_json(&v).map_err(|e| e.to_string()));
        let v: Value = serde_json::from_str(r#"{"a":1.0}"#).unwrap();
        println!("1.0 -> {:?}", canonical_json(&v).map_err(|e| e.to_string()));
        let v: Value = serde_json::from_str(r#"{"\u00e9":"\u2028\u0000\/x","B":"é","a":"\ud83d\ude00"}"#).unwrap();
        println!("unicode -> {}", canonical_json(&v).unwrap());
        // envelope with float body: verify errors (no panic)
        let mut e = env(serde_json::json!({"x": 1.5}));
        assert!(sign(&mut e, &kp(1)).is_err());
        // v / type / swap_id are all under the signature
        let mut e = env(serde_json::json!({"r":"x"})); sign(&mut e, &kp(1)).unwrap();
        let d0 = signing_digest(&e).unwrap();
        let mut e2 = e.clone(); e2.v = 2; assert_ne!(signing_digest(&e2).unwrap(), d0);
        let mut e3 = e.clone(); e3.msg_type = "take".into(); assert_ne!(signing_digest(&e3).unwrap(), d0);
        assert!(verify(&e2).is_err() && verify(&e3).is_err());
        // empty sig / short sig: clean errors
        let mut e4 = e.clone(); e4.sig = String::new(); assert!(verify(&e4).is_err());
        let mut e5 = e.clone(); e5.from = "00".repeat(32); assert!(verify(&e5).is_err());
        // Envelope with duplicate top-level field: serde derive rejects
        let r: Result<Envelope, _> = serde_json::from_str(r#"{"v":1,"v":2,"type":"a","swap_id":"s","from":"f","body":{}}"#);
        println!("dup top-level field -> {:?}", r.map(|_| ()).map_err(|e| e.to_string()));
    }

    #[test]
    fn snapshot_dtag_is_linkable_from_public_swap_id() {
        let public_swap_id = "00aa11bb22cc33dd"; // visible in the kind-31510 `d` tag
        let d = pact_nostr::snapshot_dtag(public_swap_id);
        println!("anyone can compute snapshot d-tag for offer {public_swap_id}: {d}");
        assert_eq!(d, pact_nostr::snapshot_dtag(public_swap_id));
    }

    #[test]
    fn junk_blob_open_cost() {
        let alice = kp(7); let bob = kp(8); let carol = kp(9);
        let mut e = env(serde_json::json!({"reason":"x"}));
        sign(&mut e, &alice).unwrap();
        let blob = seal_envelope(&xonly(&bob), &e).unwrap(); // not for carol
        let n = 2000;
        let t = std::time::Instant::now();
        for _ in 0..n { let _ = open_envelope(&carol, &blob); }
        println!("open_envelope (not addressed to us): {:.1} us/blob", t.elapsed().as_secs_f64()*1e6/n as f64);
        let t = std::time::Instant::now();
        for _ in 0..n { verify(&e).unwrap(); }
        println!("verify envelope: {:.1} us", t.elapsed().as_secs_f64()*1e6/n as f64);
    }
}
