//! Relay blob sealing — spec §10. End-to-end encryption of coordination
//! envelopes so a board operator sees only ciphertext addressed to a
//! pubkey: ephemeral-key ECDH against the recipient identity, keyed into
//! ChaCha20-Poly1305.

use anyhow::{bail, Context, Result};
use bitcoin::secp256k1::{ecdh, Keypair, Parity, PublicKey, Secp256k1, SecretKey, XOnlyPublicKey};
use chacha20poly1305::aead::Aead;
use chacha20poly1305::{ChaCha20Poly1305, KeyInit};
use std::str::FromStr;

use crate::crypto::tagged_hash;
use crate::envelope::Envelope;

const SEAL_MAGIC: &str = "PACTSEALED1";

/// Seal an envelope to a recipient identity (x-only pubkey, hex):
/// ephemeral-key ECDH against the even-Y lift of the identity point,
/// key = TaggedHash("pact/relay/ecdh/v1", shared), ChaCha20-Poly1305.
/// The board (and its operator) sees only the ephemeral pubkey and
/// ciphertext — sender identity, message type and contents are hidden.
pub fn seal_envelope(recipient_xonly_hex: &str, envelope: &Envelope) -> Result<String> {
    let secp = Secp256k1::new();
    let xonly = XOnlyPublicKey::from_str(recipient_xonly_hex).context("bad recipient identity")?;
    let recipient = PublicKey::from_x_only_public_key(xonly, Parity::Even);

    let mut rng = bitcoin::secp256k1::rand::thread_rng();
    let ephemeral_sk = SecretKey::new(&mut rng);
    let ephemeral_pk = ephemeral_sk.public_key(&secp);
    let shared = ecdh::SharedSecret::new(&recipient, &ephemeral_sk);
    let key = tagged_hash("pact/relay/ecdh/v1", &shared.secret_bytes());

    use bitcoin::secp256k1::rand::RngCore;
    let mut nonce = [0u8; 12];
    rng.fill_bytes(&mut nonce);
    let cipher = ChaCha20Poly1305::new((&key).into());
    let ciphertext = cipher
        .encrypt((&nonce).into(), serde_json::to_string(envelope)?.as_bytes())
        .map_err(|_| anyhow::anyhow!("relay encryption failed"))?;
    Ok(format!(
        "{SEAL_MAGIC}:{}:{}:{}",
        hex::encode(ephemeral_pk.serialize()),
        hex::encode(nonce),
        hex::encode(ciphertext)
    ))
}

/// Open a relay blob with our identity key. Only sealed blobs are
/// accepted — an unencrypted envelope is rejected, not parsed, so the
/// board cannot inject plaintext and there is no downgrade path.
pub fn open_envelope(identity: &Keypair, blob: &str) -> Result<Envelope> {
    if !blob.starts_with(SEAL_MAGIC) {
        bail!("relay blob is not sealed — refusing (no plaintext relay path)");
    }
    let mut parts = blob.split(':');
    let (_, epk, nonce, ciphertext) = (
        parts.next(),
        parts.next().context("malformed sealed blob")?,
        parts.next().context("malformed sealed blob")?,
        parts.next().context("malformed sealed blob")?,
    );
    let ephemeral_pk = PublicKey::from_slice(&hex::decode(epk)?)?;
    // BIP340 identities are x-only; ECDH needs the secret for the even-Y
    // point, so negate if our full key has odd parity.
    let (_, parity) = identity.x_only_public_key();
    let mut secret = SecretKey::from_keypair(identity);
    if parity == Parity::Odd {
        secret = secret.negate();
    }
    let shared = ecdh::SharedSecret::new(&ephemeral_pk, &secret);
    let key = tagged_hash("pact/relay/ecdh/v1", &shared.secret_bytes());
    let cipher = ChaCha20Poly1305::new((&key).into());
    let nonce_bytes = hex::decode(nonce)?;
    // Fixed-size field: the `&[u8] -> &Nonce` conversion below is a
    // `GenericArray::from_slice`, which PANICS on a length mismatch. A relay
    // (or anyone who can post to it) controls this string, so a malformed
    // length must be a recoverable parse error, never an unwind.
    if nonce_bytes.len() != 12 {
        bail!(
            "malformed sealed blob: nonce is {} bytes, expected 12",
            nonce_bytes.len()
        );
    }
    let plaintext = cipher
        .decrypt(
            nonce_bytes.as_slice().into(),
            hex::decode(ciphertext)?.as_slice(),
        )
        .map_err(|_| anyhow::anyhow!("relay decryption failed — not addressed to us?"))?;
    Ok(serde_json::from_str(&String::from_utf8(plaintext)?)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1::Secp256k1;

    fn identity(byte: u8) -> Keypair {
        Keypair::from_secret_key(
            &Secp256k1::new(),
            &SecretKey::from_slice(&[byte; 32]).unwrap(),
        )
    }

    fn xonly_hex(kp: &Keypair) -> String {
        kp.x_only_public_key().0.to_string()
    }

    #[test]
    fn seal_open_roundtrip_and_wrong_recipient() {
        let alice = identity(7);
        let bob = identity(8);
        let envelope = Envelope {
            v: 1,
            msg_type: "abort".into(),
            swap_id: "0011223344556677".into(),
            from: xonly_hex(&alice),
            body: serde_json::json!({ "reason": "test" }),
            sig: String::new(),
        };

        let blob = seal_envelope(&xonly_hex(&bob), &envelope).unwrap();
        assert!(blob.starts_with(SEAL_MAGIC));
        assert!(
            !blob.contains("abort"),
            "plaintext leaked into the sealed blob"
        );

        assert_eq!(open_envelope(&bob, &blob).unwrap(), envelope);
        // Not addressed to Alice — she must not be able to read it.
        assert!(open_envelope(&alice, &blob).is_err());
        // Plaintext blobs are refused outright (no downgrade path).
        let plain = serde_json::to_string(&envelope).unwrap();
        assert!(open_envelope(&bob, &plain).is_err());
    }

    /// Security review 2026-09-09 #5: a nonce of any length other than 12
    /// bytes used to reach `GenericArray::from_slice` and panic — under the
    /// daemon's registry lock, poisoning it. Every malformed length must now
    /// be an ordinary `Err`.
    #[test]
    fn malformed_nonce_length_is_an_error_not_a_panic() {
        let bob = identity(8);
        let envelope = Envelope {
            v: 1,
            msg_type: "abort".into(),
            swap_id: "0011223344556677".into(),
            from: xonly_hex(&bob),
            body: serde_json::json!({}),
            sig: String::new(),
        };
        let blob = seal_envelope(&xonly_hex(&bob), &envelope).unwrap();
        let mut parts: Vec<&str> = blob.split(':').collect();
        assert_eq!(parts.len(), 4);
        for bad in [
            "",
            "00",
            "0011223344556677889900",
            "00112233445566778899aabb",
            "001122334455667788990011aa",
        ] {
            parts[2] = bad;
            let tampered = parts.join(":");
            let res = std::panic::catch_unwind(|| open_envelope(&bob, &tampered));
            let res = res.expect("open_envelope must not panic on a malformed nonce");
            assert!(res.is_err(), "nonce {bad:?} must be rejected");
        }
        // Odd-length hex is a decode error, also non-panicking.
        parts[2] = "abc";
        assert!(open_envelope(&bob, &parts.join(":")).is_err());
    }
}
