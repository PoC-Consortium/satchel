//! Protocol hashes — spec §4. Pure, chain-agnostic.

use bitcoin::hashes::{sha256, Hash, HashEngine};

const TAG_SWAP_ID: &str = "pact/swapid/v1";

/// BIP340-style tagged hash: `SHA256(SHA256(tag) || SHA256(tag) || msg)`.
pub fn tagged_hash(tag: &str, msg: &[u8]) -> [u8; 32] {
    let tag_hash = sha256::Hash::hash(tag.as_bytes());
    let mut engine = sha256::Hash::engine();
    engine.input(tag_hash.as_byte_array());
    engine.input(tag_hash.as_byte_array());
    engine.input(msg);
    sha256::Hash::from_engine(engine).to_byte_array()
}

/// `H = SHA256(s)` (spec §4.3).
pub fn hash_preimage(s: &[u8; 32]) -> [u8; 32] {
    sha256::Hash::hash(s).to_byte_array()
}

/// `swap_id = hex(TaggedHash("pact/swapid/v1", H)[0..8))` (spec §4.4).
pub fn swap_id(hash_h: &[u8; 32]) -> String {
    hex::encode(&tagged_hash(TAG_SWAP_ID, hash_h)[..8])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swap_id_is_16_hex_and_deterministic() {
        let h = hash_preimage(&[0x11; 32]);
        let id = swap_id(&h);
        assert_eq!(id.len(), 16);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(id, swap_id(&h));
    }
}
