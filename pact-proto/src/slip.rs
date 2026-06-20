//! Private-offer slips — the off-market artifact (spec/protocol.md §10).
//!
//! A *slip* is a private offer serialized for an out-of-band channel (chat):
//! the exact same signed `offer` envelope that would otherwise go to the
//! Corkboard, base64url-encoded with a version prefix.
//!
//! ```text
//! pactoffer1:<base64url(canonical_json(offer_envelope))>
//! ```
//!
//! No new wire fields and no protocol bump: the encoded bytes are the
//! unchanged offer envelope (`v`, `type:"offer"`, `swap_id`, `from`, `body`,
//! `sig`). [`decode_slip`] is the only trust gate — it MUST reject an unknown
//! prefix, malformed base64, a non-`offer` envelope, and a bad BIP340
//! signature BEFORE returning, so nothing unverified is ever shown to a user.

use anyhow::{bail, Context, Result};

use crate::envelope::{canonical_json, verify, Envelope};

/// Slip version prefix. Bumping this is how a future incompatible slip format
/// is introduced; `decode_slip` rejects anything that does not match.
const SLIP_PREFIX: &str = "pactoffer1:";

/// base64url alphabet (RFC 4648 §5, URL- and filename-safe), no padding.
const B64URL: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Encode bytes as base64url *without* padding (matches `decode_b64url`).
fn encode_b64url(input: &[u8]) -> String {
    let mut out = String::new();
    for chunk in input.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(B64URL[(n >> 18 & 63) as usize] as char);
        out.push(B64URL[(n >> 12 & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(B64URL[(n >> 6 & 63) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(B64URL[(n & 63) as usize] as char);
        }
    }
    out
}

/// Decode unpadded base64url. Rejects any character outside the alphabet and a
/// dangling single character (an impossible base64 tail).
fn decode_b64url(input: &str) -> Result<Vec<u8>> {
    fn val(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some(u32::from(c - b'A')),
            b'a'..=b'z' => Some(u32::from(c - b'a') + 26),
            b'0'..=b'9' => Some(u32::from(c - b'0') + 52),
            b'-' => Some(62),
            b'_' => Some(63),
            _ => None,
        }
    }
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3 + 3);
    for chunk in bytes.chunks(4) {
        if chunk.len() == 1 {
            bail!("malformed base64 in slip (dangling character)");
        }
        let mut acc = 0u32;
        for &c in chunk {
            let v = val(c).context("malformed base64 in slip (bad character)")?;
            acc = (acc << 6) | v;
        }
        // Left-align the accumulated bits for this (possibly short) group.
        acc <<= 6 * (4 - chunk.len());
        out.push((acc >> 16 & 0xff) as u8);
        if chunk.len() >= 3 {
            out.push((acc >> 8 & 0xff) as u8);
        }
        if chunk.len() == 4 {
            out.push((acc & 0xff) as u8);
        }
    }
    Ok(out)
}

/// Serialize a signed `offer` envelope into a pasteable slip string. The caller
/// is responsible for passing a fully signed offer (this is what the engine's
/// `make_private_offer` does).
pub fn encode_slip(offer: &Envelope) -> Result<String> {
    let value = serde_json::to_value(offer)?;
    let canonical = canonical_json(&value)?;
    Ok(format!(
        "{SLIP_PREFIX}{}",
        encode_b64url(canonical.as_bytes())
    ))
}

/// Decode + fully validate a slip back into the maker's signed offer envelope.
///
/// Rejects, in order, BEFORE returning anything to the caller:
/// 1. an unknown / missing version prefix,
/// 2. malformed base64url,
/// 3. bytes that are not a valid `Envelope` JSON,
/// 4. an envelope whose `type` is not `"offer"`,
/// 5. a bad BIP340 signature over the canonical JSON (verified against `from`).
pub fn decode_slip(slip: &str) -> Result<Envelope> {
    let body = slip
        .strip_prefix(SLIP_PREFIX)
        .context("not a pact offer slip (expected the pactoffer1: prefix)")?;
    let bytes = decode_b64url(body.trim())?;
    let envelope: Envelope =
        serde_json::from_slice(&bytes).context("slip does not decode to a pact envelope")?;
    if envelope.msg_type != "offer" {
        bail!("slip is a {:?} envelope, not an offer", envelope.msg_type);
    }
    // Signature gate: the slip is bearer, but it must be the maker's genuine,
    // untampered offer. `verify` checks `sig` over the canonical JSON against
    // `from`, so any edit in transit fails here, before a card is shown.
    verify(&envelope).context("slip signature does not verify")?;
    Ok(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::sign;
    use bitcoin::secp256k1::{Keypair, Secp256k1, SecretKey};

    fn identity() -> Keypair {
        Keypair::from_secret_key(
            &Secp256k1::new(),
            &SecretKey::from_slice(&[0x24; 32]).unwrap(),
        )
    }

    fn signed_offer() -> Envelope {
        let mut env = Envelope {
            v: 1,
            msg_type: "offer".into(),
            swap_id: "0011223344556677".into(),
            from: String::new(),
            body: serde_json::json!({
                "protocol": "pact-htlc-v1",
                "network": "regtest",
                "give_asset": "btcx",
                "give_amount": 100000u64,
                "get_asset": "btc",
                "get_amount": 5000u64,
                "t1_secs": 7200u32,
                "t2_secs": 3600u32,
                "created": 1_700_000_000u64,
            }),
            sig: String::new(),
        };
        sign(&mut env, &identity()).unwrap();
        env
    }

    #[test]
    fn base64url_roundtrip_all_lengths() {
        // Cover every chunk-remainder (0,1,2 trailing bytes) so the codec's
        // short-group handling is exercised.
        for n in 0..20usize {
            let data: Vec<u8> = (0..n).map(|i| (i as u8).wrapping_mul(37)).collect();
            let encoded = encode_b64url(&data);
            assert!(!encoded.contains('='), "must be unpadded: {encoded}");
            assert_eq!(decode_b64url(&encoded).unwrap(), data, "n={n}");
        }
    }

    #[test]
    fn slip_roundtrip_preserves_envelope() {
        let offer = signed_offer();
        let slip = encode_slip(&offer).unwrap();
        assert!(slip.starts_with("pactoffer1:"));
        let decoded = decode_slip(&slip).unwrap();
        assert_eq!(decoded, offer);
    }

    #[test]
    fn reject_unknown_prefix() {
        let offer = signed_offer();
        let slip = encode_slip(&offer).unwrap();
        // Right base64, wrong prefix.
        let bad = slip.replace("pactoffer1:", "pactoffer2:");
        assert!(decode_slip(&bad).is_err());
        // No prefix at all.
        assert!(decode_slip("AAAA").is_err());
    }

    #[test]
    fn reject_malformed_base64() {
        // Characters outside the base64url alphabet (`*`, `+`, `/`, `=`).
        assert!(decode_slip("pactoffer1:****").is_err());
        assert!(decode_slip("pactoffer1:AB+/").is_err());
        // A dangling single character is an impossible base64 tail.
        assert!(decode_slip("pactoffer1:AAAAA").is_err());
    }

    #[test]
    fn reject_non_offer_type() {
        // A correctly-signed envelope that is not an offer must be refused.
        let mut env = signed_offer();
        env.msg_type = "take".into();
        sign(&mut env, &identity()).unwrap(); // re-sign so the sig is valid
        let slip = encode_slip(&env).unwrap();
        let err = decode_slip(&slip).unwrap_err().to_string();
        assert!(err.contains("not an offer"), "{err}");
    }

    #[test]
    fn reject_bad_signature() {
        let mut offer = signed_offer();
        let slip = encode_slip(&offer).unwrap();
        // Tamper with the body AFTER signing: the canonical JSON changes, so
        // the (now stale) signature must fail to verify.
        offer.body["get_amount"] = serde_json::json!(999u64);
        let tampered = encode_slip(&offer).unwrap();
        assert_ne!(slip, tampered);
        assert!(decode_slip(&tampered).is_err());

        // An entirely empty signature is also rejected.
        offer.sig = String::new();
        let unsigned = encode_slip(&offer).unwrap();
        assert!(decode_slip(&unsigned).is_err());
    }
}
