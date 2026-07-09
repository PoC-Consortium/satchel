//! The pact-htlc-v1 witness script and P2WSH output — spec §5.
//!
//! ```text
//! OP_IF
//!     OP_SIZE 32 OP_EQUALVERIFY
//!     OP_SHA256 <H> OP_EQUALVERIFY
//!     OP_DUP OP_HASH160 <hash160(redeem_pubkey)>
//! OP_ELSE
//!     <T> OP_CHECKLOCKTIMEVERIFY OP_DROP
//!     OP_DUP OP_HASH160 <hash160(refund_pubkey)>
//! OP_ENDIF
//! OP_EQUALVERIFY
//! OP_CHECKSIG
//! ```

use anyhow::{bail, Result};
use bitcoin::hashes::{hash160, sha256, Hash};
use bitcoin::opcodes::all::{
    OP_CHECKSIG, OP_CLTV, OP_DROP, OP_DUP, OP_ELSE, OP_ENDIF, OP_EQUALVERIFY, OP_HASH160, OP_IF,
    OP_SHA256, OP_SIZE,
};
use bitcoin::script::Builder;
use bitcoin::secp256k1::PublicKey;
use bitcoin::ScriptBuf;

use crate::params::ChainParams;

/// Locktimes below this are block heights, which v1 forbids (spec §5).
pub const MIN_TIME_LOCKTIME: u32 = 500_000_000;

/// One HTLC instance (one chain leg of a swap).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Htlc {
    pub hash_h: [u8; 32],
    pub redeem_pubkey: PublicKey,
    pub refund_pubkey: PublicKey,
    /// Absolute Unix-time locktime `T` (CLTV, evaluated against BIP113 MTP).
    pub locktime: u32,
}

impl Htlc {
    pub fn new(
        hash_h: [u8; 32],
        redeem_pubkey: PublicKey,
        refund_pubkey: PublicKey,
        locktime: u32,
    ) -> Result<Self> {
        if locktime < MIN_TIME_LOCKTIME {
            bail!(
                "locktime {locktime} is a block height; v1 requires Unix-time locktimes (spec §5)"
            );
        }
        Ok(Self {
            hash_h,
            redeem_pubkey,
            refund_pubkey,
            locktime,
        })
    }

    /// The exact v1 witness script (spec §5).
    pub fn witness_script(&self) -> ScriptBuf {
        let redeem_pkh = hash160::Hash::hash(&self.redeem_pubkey.serialize()).to_byte_array();
        let refund_pkh = hash160::Hash::hash(&self.refund_pubkey.serialize()).to_byte_array();
        Builder::new()
            .push_opcode(OP_IF)
            .push_opcode(OP_SIZE)
            .push_int(32)
            .push_opcode(OP_EQUALVERIFY)
            .push_opcode(OP_SHA256)
            .push_slice(self.hash_h)
            .push_opcode(OP_EQUALVERIFY)
            .push_opcode(OP_DUP)
            .push_opcode(OP_HASH160)
            .push_slice(redeem_pkh)
            .push_opcode(OP_ELSE)
            .push_int(i64::from(self.locktime))
            .push_opcode(OP_CLTV)
            .push_opcode(OP_DROP)
            .push_opcode(OP_DUP)
            .push_opcode(OP_HASH160)
            .push_slice(refund_pkh)
            .push_opcode(OP_ENDIF)
            .push_opcode(OP_EQUALVERIFY)
            .push_opcode(OP_CHECKSIG)
            .into_script()
    }

    /// P2WSH scriptPubKey: `OP_0 <SHA256(witness_script)>`.
    pub fn script_pubkey(&self) -> ScriptBuf {
        ScriptBuf::new_p2wsh(&self.witness_script().wscript_hash())
    }

    /// bech32 address under the given chain's HRP.
    pub fn address(&self, chain: &ChainParams) -> Result<String> {
        chain.p2wsh_address(&self.witness_script())
    }
}

/// Extract the swap preimage from the witness of a transaction input that
/// spends an HTLC via the hash branch (spec §9.4). Backend data is
/// untrusted, so the candidate is verified against `H` rather than taken
/// positionally on faith.
pub fn extract_preimage(witness_items: &[Vec<u8>], hash_h: &[u8; 32]) -> Option<[u8; 32]> {
    for item in witness_items {
        if item.len() == 32 {
            let candidate: [u8; 32] = item.as_slice().try_into().expect("length checked");
            if sha256::Hash::hash(&candidate).to_byte_array() == *hash_h {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::hashes::{sha256, Hash};
    use std::str::FromStr;

    fn test_htlc() -> Htlc {
        // Arbitrary but fixed keys (generator point and 2G).
        let redeem = PublicKey::from_str(
            "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
        )
        .unwrap();
        let refund = PublicKey::from_str(
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
        )
        .unwrap();
        let s = [0x11u8; 32];
        let hash_h = sha256::Hash::hash(&s).to_byte_array();
        Htlc::new(hash_h, redeem, refund, 1_780_000_000).unwrap()
    }

    #[test]
    fn script_structure() {
        let script = test_htlc().witness_script();
        let bytes = script.as_bytes();
        // OP_IF OP_SIZE PUSH1 0x20 OP_EQUALVERIFY OP_SHA256 PUSH32 ...
        assert_eq!(&bytes[..7], &[0x63, 0x82, 0x01, 0x20, 0x88, 0xa8, 0x20]);
        assert_eq!(*bytes.last().unwrap(), 0xac); // OP_CHECKSIG
        assert_eq!(bytes[bytes.len() - 2], 0x88); // OP_EQUALVERIFY
        assert_eq!(bytes[bytes.len() - 3], 0x68); // OP_ENDIF
                                                  // 4-byte minimal CScriptNum push of the locktime in the ELSE branch.
        let lt = 1_780_000_000u32.to_le_bytes();
        let needle = [0x67u8, 0x04, lt[0], lt[1], lt[2], lt[3], 0xb1, 0x75];
        assert!(bytes.windows(needle.len()).any(|w| w == needle));
    }

    #[test]
    fn height_locktime_rejected() {
        let h = test_htlc();
        assert!(Htlc::new(h.hash_h, h.redeem_pubkey, h.refund_pubkey, 800_000).is_err());
    }

    #[test]
    fn p2wsh_and_addresses() {
        let htlc = test_htlc();
        let spk = htlc.script_pubkey();
        assert!(spk.is_p2wsh());
        let pocx = htlc.address(&crate::params::BTCX_REGTEST).unwrap();
        let btc = htlc.address(&crate::params::BTC_REGTEST).unwrap();
        assert!(pocx.starts_with("rpocx1"));
        assert!(btc.starts_with("bcrt1"));
    }

    #[test]
    fn preimage_extraction() {
        let s = [0x11u8; 32];
        let hash_h = sha256::Hash::hash(&s).to_byte_array();
        let witness = vec![
            vec![0x30u8; 71],
            vec![0x02u8; 33],
            s.to_vec(),
            vec![0x01],
            vec![0u8; 90],
        ];
        assert_eq!(extract_preimage(&witness, &hash_h), Some(s));
        // A wrong 32-byte item must not be accepted.
        let witness_bad = vec![vec![0x22u8; 32]];
        assert_eq!(extract_preimage(&witness_bad, &hash_h), None);
    }
}
