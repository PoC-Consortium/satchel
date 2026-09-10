use bitcoin::secp256k1::{Keypair, Secp256k1, SecretKey};
fn main() {
 let kp = Keypair::from_secret_key(&Secp256k1::new(), &SecretKey::from_slice(&[7;32]).unwrap());
 let epk = kp.public_key().to_string();
 for len in [0usize, 1, 11, 12, 13] {
  let blob = format!("PACTSEALED1:{}:{}:00", epk, "00".repeat(len));
  let result = std::panic::catch_unwind(|| pact_proto::seal::open_envelope(&kp, &blob));
  println!("nonce_length={len} panic={}", result.is_err());
 }
}
