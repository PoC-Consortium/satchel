//! Key and secret derivation from the Pact seed — spec §4.
//!
//! All material derives from one BIP39 seed via BIP32:
//!
//! | Material                          | Path                               |
//! |-----------------------------------|------------------------------------|
//! | Identity key (BIP340)             | `m/7228'/0'/0'`                    |
//! | Swap key, initiator (chain c, i)  | `m/7228'/1'/coin(c)'/i'`           |
//! | Swap key, participant (anchored)  | `m/7228'/1'/coin(c)'/a'/b'/c'/d'`  |
//! | Preimage source (swap i)          | `m/7228'/2'/i'`                    |
//! | Refund key (v2, chain c)          | `m/7228'/3'/…` (same i/anchored split) |
//!
//! Two index schemes (spec §4.2): the **initiator** allocates a local
//! monotonic counter `i` — its deterministic preimage/adaptor secret at that
//! index is what the swap id itself is derived from, so the counter is the
//! root of the swap's identity. The **participant** learns the swap's public
//! anchor (v1: hash `H`; v2: adaptor point `T`) before deriving any key, so
//! its keys are *anchored*: four hardened 31-bit levels taken from a tagged
//! hash of the anchor. Anchored keys need no counter — the same seed on any
//! machine derives the same key from the anchor alone (for v1 the anchor is
//! embedded in the on-chain HTLC script), and two different swaps can never
//! collide on one key. The path depths differ (counter 4 vs anchored 7), so
//! the two schemes can never derive the same key either.

use anyhow::Result;
use bitcoin::bip32::{ChildNumber, Xpriv};
use bitcoin::secp256k1::{All, Keypair, PublicKey, Secp256k1, SecretKey, XOnlyPublicKey};
use bitcoin::NetworkKind;

// Protocol hashes live in pact-proto now; re-exported so existing
// `crate::keys::{tagged_hash, hash_preimage, swap_id}` callers are unchanged.
pub use pact_proto::crypto::{hash_preimage, swap_id, tagged_hash};

/// BIP32 purpose for Pact: "PACT" on a phone keypad.
pub const PURPOSE: u32 = 7228;
/// Asset constants for `coin(c)` — asset, not network (spec §4.1).
pub const COIN_BTC: u32 = 0;
/// `0x504F4358` = ASCII "POCX" (matches the node's `POCX` assignment marker).
pub const COIN_POCX: u32 = 0x504F_4358;

const TAG_PREIMAGE: &str = "pact/htlc/preimage/v1";
/// v2 adaptor secret tag (spec v2 §3.1).
const TAG_ADAPTOR: &str = "pact/adaptor/secret/v2";
/// Participant swap-key anchor tag (spec §4.2).
const TAG_KEY_ANCHOR: &str = "pact/swap-key-anchor/v1";

/// Anchored path levels: four hardened 31-bit indices from the tagged hash of
/// the swap's public anchor (v1: hash `H`; v2: compressed adaptor point `T`).
/// 124 hash bits — collision-free in practice, unlike a single truncated u32
/// (birthday bound ~46k swaps at 31 bits).
fn anchor_levels(anchor: &[u8]) -> [u32; 4] {
    let h = tagged_hash(TAG_KEY_ANCHOR, anchor);
    core::array::from_fn(|i| {
        u32::from_be_bytes(h[4 * i..4 * i + 4].try_into().expect("4 bytes")) & 0x7FFF_FFFF
    })
}

/// The Pact seed: hot transit keys only; proceeds sweep to the core wallet.
pub struct PactSeed {
    master: Xpriv,
    secp: Secp256k1<All>,
}

impl PactSeed {
    /// From a BIP39 mnemonic phrase (+ optional passphrase).
    pub fn from_mnemonic(phrase: &str, passphrase: &str) -> Result<Self> {
        let mnemonic = bip39::Mnemonic::parse_normalized(phrase)?;
        Self::from_seed(&mnemonic.to_seed_normalized(passphrase))
    }

    /// From raw BIP39 seed bytes. The BIP32 network kind only affects
    /// xprv/xpub serialization, never derived keys; Main is used throughout.
    pub fn from_seed(seed: &[u8]) -> Result<Self> {
        Ok(Self {
            master: Xpriv::new_master(NetworkKind::Main, seed)?,
            secp: Secp256k1::new(),
        })
    }

    fn derive(&self, path: &[u32]) -> Result<SecretKey> {
        let path: Vec<ChildNumber> = path
            .iter()
            .map(|&i| ChildNumber::from_hardened_idx(i).map_err(Into::into))
            .collect::<Result<_>>()?;
        Ok(self.master.derive_priv(&self.secp, &path)?.private_key)
    }

    /// Identity keypair at `m/7228'/0'/0'` — signs handshake messages
    /// (BIP340 Schnorr); never used in any HTLC.
    pub fn identity_keypair(&self) -> Result<Keypair> {
        Ok(self.derive(&[PURPOSE, 0, 0])?.keypair(&self.secp))
    }

    /// x-only identity pubkey (the `from` field of message envelopes).
    pub fn identity_pubkey(&self) -> Result<XOnlyPublicKey> {
        Ok(self.identity_keypair()?.x_only_public_key().0)
    }

    /// Swap secret key at `m/7228'/1'/coin'/index'` (HTLC redeem/refund,
    /// ECDSA). One key per chain per swap; never reused across swaps.
    pub fn swap_secret_key(&self, coin: u32, index: u32) -> Result<SecretKey> {
        self.derive(&[PURPOSE, 1, coin, index])
    }

    /// Compressed pubkey for [`Self::swap_secret_key`].
    pub fn swap_pubkey(&self, coin: u32, index: u32) -> Result<PublicKey> {
        Ok(self.swap_secret_key(coin, index)?.public_key(&self.secp))
    }

    /// Participant swap key at `m/7228'/1'/coin'/a'/b'/c'/d'`, the four levels
    /// taken from the swap's public anchor (spec §4.2) — v1: the 32-byte hash
    /// `H`, v2: the compressed adaptor point `T`. Counter-free: re-derivable
    /// from the anchor alone, and never shared between two different swaps.
    pub fn swap_secret_key_anchored(&self, coin: u32, anchor: &[u8]) -> Result<SecretKey> {
        let [a, b, c, d] = anchor_levels(anchor);
        self.derive(&[PURPOSE, 1, coin, a, b, c, d])
    }

    /// Compressed pubkey for [`Self::swap_secret_key_anchored`].
    pub fn swap_pubkey_anchored(&self, coin: u32, anchor: &[u8]) -> Result<PublicKey> {
        Ok(self
            .swap_secret_key_anchored(coin, anchor)?
            .public_key(&self.secp))
    }

    /// Deterministic preimage for swap index `i` (spec §4.3) —
    /// `s = TaggedHash("pact/htlc/preimage/v1", key at m/7228'/2'/i')`.
    /// Initiator only. Re-derivable from the seed alone.
    pub fn preimage(&self, index: u32) -> Result<[u8; 32]> {
        let k = self.derive(&[PURPOSE, 2, index])?;
        Ok(tagged_hash(TAG_PREIMAGE, &k.secret_bytes()))
    }

    // ---- v2 (pact-htlc-v2): Taproot / MuSig2 adaptor swaps (spec v2 §3) ----

    /// x-only form of the swap key — the BIP340 / MuSig2 signer for a Taproot
    /// key-path spend. Same derivation as [`Self::swap_secret_key`]; only the
    /// public encoding differs (v1 uses it as compressed ECDSA, v2 as x-only).
    pub fn swap_xonly_pubkey(&self, coin: u32, index: u32) -> Result<XOnlyPublicKey> {
        Ok(self
            .swap_secret_key(coin, index)?
            .x_only_public_key(&self.secp)
            .0)
    }

    /// Refund key at `m/7228'/3'/coin'/index'` — signs the single-key CLTV
    /// refund tapleaf (spec v2 §3, §4). A *separate* branch from the MuSig2
    /// swap key so the refund path is single-sig and independent.
    pub fn refund_secret_key(&self, coin: u32, index: u32) -> Result<SecretKey> {
        self.derive(&[PURPOSE, 3, coin, index])
    }

    /// x-only pubkey for [`Self::refund_secret_key`] (the refund tapleaf key).
    pub fn refund_xonly_pubkey(&self, coin: u32, index: u32) -> Result<XOnlyPublicKey> {
        Ok(self
            .refund_secret_key(coin, index)?
            .x_only_public_key(&self.secp)
            .0)
    }

    /// Participant refund key at `m/7228'/3'/coin'/a'/b'/c'/d'` — the anchored
    /// analog of [`Self::refund_secret_key`] (see [`Self::swap_secret_key_anchored`]).
    pub fn refund_secret_key_anchored(&self, coin: u32, anchor: &[u8]) -> Result<SecretKey> {
        let [a, b, c, d] = anchor_levels(anchor);
        self.derive(&[PURPOSE, 3, coin, a, b, c, d])
    }

    /// x-only pubkey for [`Self::refund_secret_key_anchored`].
    pub fn refund_xonly_pubkey_anchored(&self, coin: u32, anchor: &[u8]) -> Result<XOnlyPublicKey> {
        Ok(self
            .refund_secret_key_anchored(coin, anchor)?
            .x_only_public_key(&self.secp)
            .0)
    }

    /// Deterministic adaptor secret `t` for swap index `i` (spec v2 §3.1) —
    /// `t = TaggedHash("pact/adaptor/secret/v2", key at m/7228'/2'/i')`,
    /// as a valid secp256k1 scalar. Initiator only; the v2 analog of the v1
    /// preimage. Re-derivable from the seed alone, so losing the state DB
    /// never loses the secret.
    pub fn adaptor_secret(&self, index: u32) -> Result<SecretKey> {
        let k = self.derive(&[PURPOSE, 2, index])?;
        let bytes = tagged_hash(TAG_ADAPTOR, &k.secret_bytes());
        SecretKey::from_slice(&bytes)
            .map_err(|e| anyhow::anyhow!("adaptor secret not a valid scalar: {e}"))
    }

    /// The adaptor point `T = t·G` (compressed pubkey) — shared in `init`,
    /// not secret (spec v2 §3.1).
    pub fn adaptor_point(&self, index: u32) -> Result<PublicKey> {
        Ok(self.adaptor_secret(index)?.public_key(&self.secp))
    }
}

/// v2 swap identifier (spec v2 §3.3): `hex(TaggedHash("pact/swapid/v2", T)[0..8])`
/// over the adaptor point `T` — the v2 analog of v1's `swap_id` over `H`.
pub fn swap_id_v2(adaptor_point: &PublicKey) -> String {
    let h = tagged_hash("pact/swapid/v2", &adaptor_point.serialize());
    hex::encode(&h[..8])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The standard BIP39 test mnemonic; used for spec test vectors too.
    pub const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    fn seed() -> PactSeed {
        PactSeed::from_mnemonic(TEST_MNEMONIC, "").unwrap()
    }

    #[test]
    fn deterministic_and_distinct() {
        let a = seed();
        let b = seed();
        assert_eq!(a.preimage(0).unwrap(), b.preimage(0).unwrap());
        assert_ne!(a.preimage(0).unwrap(), a.preimage(1).unwrap());
        assert_eq!(
            a.swap_pubkey(COIN_POCX, 0).unwrap(),
            b.swap_pubkey(COIN_POCX, 0).unwrap()
        );
        assert_ne!(
            a.swap_pubkey(COIN_POCX, 0).unwrap(),
            a.swap_pubkey(COIN_BTC, 0).unwrap()
        );
    }

    #[test]
    fn preimage_is_32_bytes_and_hashes() {
        let s = seed().preimage(0).unwrap();
        let h = hash_preimage(&s);
        assert_eq!(s.len(), 32);
        let id = swap_id(&h);
        assert_eq!(id.len(), 16);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn identity_is_not_a_swap_key() {
        let s = seed();
        let ident = s.identity_keypair().unwrap().public_key();
        assert_ne!(ident, s.swap_pubkey(COIN_BTC, 0).unwrap());
    }

    // ---- v2 derivations ----

    #[test]
    fn adaptor_secret_deterministic_and_point_matches() {
        let secp = Secp256k1::new();
        let (a, b) = (seed(), seed());
        // Deterministic across instances, distinct per index.
        assert_eq!(a.adaptor_secret(0).unwrap(), b.adaptor_secret(0).unwrap());
        assert_ne!(a.adaptor_secret(0).unwrap(), a.adaptor_secret(1).unwrap());
        // T = t·G.
        let t = a.adaptor_secret(3).unwrap();
        assert_eq!(a.adaptor_point(3).unwrap(), t.public_key(&secp));
        // The v2 adaptor secret is NOT the v1 preimage (different tag).
        assert_ne!(
            a.adaptor_secret(0).unwrap().secret_bytes(),
            a.preimage(0).unwrap()
        );
    }

    #[test]
    fn refund_key_is_separate_branch() {
        let s = seed();
        // Deterministic, distinct per coin/index.
        assert_eq!(
            s.refund_xonly_pubkey(COIN_BTC, 0).unwrap(),
            seed().refund_xonly_pubkey(COIN_BTC, 0).unwrap()
        );
        assert_ne!(
            s.refund_secret_key(COIN_BTC, 0).unwrap(),
            s.refund_secret_key(COIN_POCX, 0).unwrap()
        );
        // Branch 3' refund key is independent of the branch 1' swap key.
        assert_ne!(
            s.refund_secret_key(COIN_BTC, 0).unwrap(),
            s.swap_secret_key(COIN_BTC, 0).unwrap()
        );
    }

    // ---- anchored (participant) derivations — spec §4.2 ----

    #[test]
    fn anchored_keys_deterministic_and_distinct() {
        let s = seed();
        let h1 = [0x11u8; 32];
        let h2 = [0x22u8; 32];
        // Deterministic across instances.
        assert_eq!(
            s.swap_pubkey_anchored(COIN_BTC, &h1).unwrap(),
            seed().swap_pubkey_anchored(COIN_BTC, &h1).unwrap()
        );
        // Distinct per anchor and per coin.
        assert_ne!(
            s.swap_secret_key_anchored(COIN_BTC, &h1).unwrap(),
            s.swap_secret_key_anchored(COIN_BTC, &h2).unwrap()
        );
        assert_ne!(
            s.swap_secret_key_anchored(COIN_BTC, &h1).unwrap(),
            s.swap_secret_key_anchored(COIN_POCX, &h1).unwrap()
        );
        // Refund branch is independent of the swap branch for the same anchor.
        assert_ne!(
            s.refund_secret_key_anchored(COIN_BTC, &h1).unwrap(),
            s.swap_secret_key_anchored(COIN_BTC, &h1).unwrap()
        );
        // A 33-byte anchor (v2 compressed point) works and differs.
        let t = s.adaptor_point(0).unwrap().serialize();
        assert_eq!(
            s.swap_pubkey_anchored(COIN_BTC, &t).unwrap(),
            seed().swap_pubkey_anchored(COIN_BTC, &t).unwrap()
        );
        assert_ne!(
            s.swap_secret_key_anchored(COIN_BTC, &t).unwrap(),
            s.swap_secret_key_anchored(COIN_BTC, &h1).unwrap()
        );
    }

    #[test]
    fn anchored_never_collides_with_counter_path() {
        // The counter path is 4 levels deep, the anchored path 7 — even an
        // adversarially chosen anchor can't reproduce a counter-derived key.
        let s = seed();
        let h = [0x33u8; 32];
        let anchored = s.swap_secret_key_anchored(COIN_BTC, &h).unwrap();
        for i in 0..64 {
            assert_ne!(anchored, s.swap_secret_key(COIN_BTC, i).unwrap());
        }
    }

    #[test]
    fn swap_xonly_matches_swap_pubkey() {
        let s = seed();
        let xonly = s.swap_xonly_pubkey(COIN_POCX, 0).unwrap();
        assert_eq!(
            xonly,
            s.swap_pubkey(COIN_POCX, 0).unwrap().x_only_public_key().0
        );
    }
}
