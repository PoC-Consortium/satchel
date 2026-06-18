//! Signed handshake envelopes + canonical JSON — spec §8.
//!
//! Envelopes are transport-agnostic JSON. Signatures are BIP340 Schnorr
//! by the sender's identity key over the tagged hash of the canonical
//! encoding without `sig`. The `body` is opaque JSON here — typed bodies
//! live in the engine, keeping this crate chain-agnostic.

use anyhow::{bail, Context, Result};
use bitcoin::secp256k1::{schnorr, Keypair, Message, Secp256k1, XOnlyPublicKey};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::crypto::tagged_hash;

const TAG_MSG: &str = "pact/msg/v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Envelope {
    pub v: u32,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub swap_id: String,
    /// 32-byte x-only identity pubkey, hex.
    pub from: String,
    pub body: Value,
    /// 64-byte BIP340 signature, hex. Empty until signed.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sig: String,
}

/// Canonical JSON (spec §8.1): keys sorted bytewise, no whitespace,
/// integers in decimal. Implemented explicitly rather than relying on
/// serde_json map ordering so that a `preserve_order` feature enabled by
/// any other dependency cannot silently change signatures.
pub fn canonical_json(value: &Value) -> Result<String> {
    let mut out = String::new();
    write_canonical(value, &mut out)?;
    Ok(out)
}

fn write_canonical(value: &Value, out: &mut String) -> Result<()> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            if !n.is_i64() && !n.is_u64() {
                bail!("floats are forbidden in pact messages (spec §8.1): {n}");
            }
            out.push_str(&n.to_string());
        }
        Value::String(s) => out.push_str(&serde_json::to_string(s)?),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(item, out)?;
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_unstable();
            out.push('{');
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(key)?);
                out.push(':');
                write_canonical(&map[*key], out)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

/// The 32-byte digest an envelope's signature commits to.
pub fn signing_digest(envelope: &Envelope) -> Result<[u8; 32]> {
    let mut unsigned = envelope.clone();
    unsigned.sig = String::new();
    let value = serde_json::to_value(&unsigned)?;
    let canonical = canonical_json(&value)?;
    Ok(tagged_hash(TAG_MSG, canonical.as_bytes()))
}

/// Sign an envelope with the identity keypair, filling `from` and `sig`.
pub fn sign(envelope: &mut Envelope, identity: &Keypair) -> Result<()> {
    let secp = Secp256k1::new();
    envelope.from = identity.x_only_public_key().0.to_string();
    let digest = signing_digest(envelope)?;
    let sig = secp.sign_schnorr_no_aux_rand(&Message::from_digest(digest), identity);
    envelope.sig = hex::encode(sig.as_ref());
    Ok(())
}

/// Verify `sig` against the `from` key. The caller is responsible for
/// checking that `from` matches the identity pinned for this swap (§8.2).
pub fn verify(envelope: &Envelope) -> Result<()> {
    let secp = Secp256k1::verification_only();
    let from_bytes: [u8; 32] = hex::decode(&envelope.from)
        .context("from is not hex")?
        .try_into()
        .map_err(|_| anyhow::anyhow!("from must be 32 bytes (x-only pubkey)"))?;
    let pubkey = XOnlyPublicKey::from_slice(&from_bytes)?;
    let sig_bytes: [u8; 64] = hex::decode(&envelope.sig)
        .context("sig is not hex")?
        .try_into()
        .map_err(|_| anyhow::anyhow!("sig must be 64 bytes"))?;
    let sig = schnorr::Signature::from_slice(&sig_bytes)?;
    let digest = signing_digest(envelope)?;
    secp.verify_schnorr(&sig, &Message::from_digest(digest), &pubkey)
        .context("invalid envelope signature")
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1::{Secp256k1, SecretKey};

    fn test_identity() -> Keypair {
        let sk = SecretKey::from_slice(&[0x42; 32]).unwrap();
        Keypair::from_secret_key(&Secp256k1::new(), &sk)
    }

    fn envelope() -> Envelope {
        Envelope {
            v: 1,
            msg_type: "abort".into(),
            swap_id: "0011223344556677".into(),
            from: String::new(),
            body: serde_json::json!({ "reason": "test" }),
            sig: String::new(),
        }
    }

    #[test]
    fn canonical_sorts_keys_and_rejects_floats() {
        let v: Value = serde_json::from_str(r#"{"b":1,"a":{"z":[2,"x"],"y":null}}"#).unwrap();
        assert_eq!(
            canonical_json(&v).unwrap(),
            r#"{"a":{"y":null,"z":[2,"x"]},"b":1}"#
        );
        let f: Value = serde_json::from_str(r#"{"x":1.5}"#).unwrap();
        assert!(canonical_json(&f).is_err());
    }

    #[test]
    fn sign_verify_roundtrip() {
        let identity = test_identity();
        let mut env = envelope();
        sign(&mut env, &identity).unwrap();
        verify(&env).unwrap();

        let mut tampered = env.clone();
        tampered.swap_id = "ffffffffffffffff".into();
        assert!(verify(&tampered).is_err());
    }
}
