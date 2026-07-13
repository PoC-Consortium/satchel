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
//!
//! The seed/master-key handling and the standard on-chain wallet branches
//! (BIP-84/BIP-86) live in the extracted `keys-btcx` crate
//! ([`WalletSeed`]); only the Pact tree (`m/7228'`) is defined here, built
//! on [`WalletSeed::derive_hardened`].

use anyhow::Result;
use bitcoin::secp256k1::{Keypair, PublicKey, SecretKey, XOnlyPublicKey};

use crate::params::Network;

pub use keys_btcx::{DescriptorKind, WalletSeed, COIN_BTC, COIN_BTCX};

/// The shared SLIP-44 **testnet** coin type (`1'`) — used by Bitcoin Core,
/// Phoenix, and every Core-style wallet for *all* test networks regardless of
/// asset. See [`btcx_coin_type`].
pub const COIN_TESTNET: u32 = 1;

/// BIP-32 coin type for the **Bitcoin-PoCX (BTCX)** asset, network-aware — the
/// single source of truth for `coin(c)` on this asset (spec §4.1).
///
/// | network         | coin type                   |
/// |-----------------|-----------------------------|
/// | mainnet         | [`COIN_BTCX`] (`0x504F4358`) |
/// | testnet/regtest | [`COIN_TESTNET`] (`1'`)      |
///
/// Mainnet keeps the registered per-asset coin type, so a seed's mainnet funds
/// stay portable Phoenix↔Satchel. Testnet and regtest deliberately fall back to
/// the shared SLIP-44 testnet coin type `1'`: the network params (tpub/tprv,
/// `tb1`/`bcrt1`) already separate the keys, so a per-asset testnet coin type
/// buys nothing and breaks portability with Core-style wallets. This is a spec
/// §4.1 deviation scoped to the test networks — mainnet is unchanged.
///
/// Only the BTCX asset routes through here; every other coin (BTC included)
/// keeps its own SLIP-44 registry mapping on all networks. Phoenix adopted the
/// same rule, and the shared on-chain wallet path (`wallet-btcx`) applies it
/// too, so wallet and swap keys agree on every network.
pub fn btcx_coin_type(network: Network) -> u32 {
    match network {
        Network::Mainnet => COIN_BTCX,
        Network::Testnet | Network::Regtest => COIN_TESTNET,
    }
}

// Protocol hashes live in pact-proto now; re-exported so existing
// `crate::keys::{tagged_hash, hash_preimage, swap_id}` callers are unchanged.
pub use pact_proto::crypto::{hash_preimage, swap_id, tagged_hash};

/// BIP32 purpose for Pact: "PACT" on a phone keypad.
pub const PURPOSE: u32 = 7228;

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

/// Per-machine seed-derivation scope — the backbone of the multi-machine
/// partition (§1 of docs/MULTI_MACHINE_122.md). A random **62-bit** value,
/// one per install, injected as **two hardened 31-bit BIP32 levels** into every
/// *initiator / counter-based* derivation so two machines on the same seed
/// derive **different** preimages / adaptor secrets / swap keys at the same
/// counter `i` — closing the catastrophic secret-reuse vector.
///
/// `0` is the reserved **LEGACY** marker: "derive the pre-scope way, injecting
/// no scope levels", so a record written before scopes existed re-derives on the
/// exact original path. Because `0` is never a real machine's own scope (fresh
/// scopes are drawn nonzero, see `machine::load_or_create_scope`), a legacy
/// record is *foreign to every machine* — recoverable only via the confirm-gated
/// path, never silently self-driven. Participant keys are **anchored**, not
/// counter-based, so the scope is only a machine tag on those records, not a
/// derivation input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeriveScope(pub u64);

impl DeriveScope {
    /// The reserved legacy marker — derive the old way, no scope levels.
    pub const LEGACY: DeriveScope = DeriveScope(0);

    /// Widest valid scope: two hardened 31-bit levels ⇒ 62 bits.
    pub const MASK: u64 = (1u64 << 62) - 1;

    /// True for the legacy (pre-scope) marker.
    pub fn is_legacy(self) -> bool {
        self.0 == 0
    }

    /// The hardened BIP32 levels this scope injects: empty for LEGACY (keeping
    /// the pre-scope path), else `[scope_hi(31 bits), scope_lo(31 bits)]`.
    fn levels(self) -> Vec<u32> {
        if self.0 == 0 {
            Vec::new()
        } else {
            let v = self.0 & Self::MASK;
            vec![((v >> 31) & 0x7FFF_FFFF) as u32, (v & 0x7FFF_FFFF) as u32]
        }
    }
}

/// Build a counter-based derivation path `prefix ++ scope-levels ++ [index]`.
/// LEGACY injects no scope levels, so a pre-scope record reproduces its original
/// (shorter) path; a real scope inserts its two hardened levels between the
/// branch prefix and the counter. Path depths stay distinct from the 7-level
/// anchored scheme (scoped b1/b3 = 6, b2 = 5, legacy b1/b3 = 4, b2 = 3), so no
/// two schemes ever derive the same key.
fn scoped_path(prefix: &[u32], scope: DeriveScope, index: u32) -> Vec<u32> {
    let mut path = prefix.to_vec();
    path.extend(scope.levels());
    path.push(index);
    path
}

/// The Pact seed: hot transit keys only; proceeds sweep to the core wallet.
/// A thin wrapper over [`WalletSeed`] (keys-btcx) that adds the Pact
/// protocol tree (`m/7228'`) on top of the same master key.
pub struct PactSeed {
    seed: WalletSeed,
}

impl PactSeed {
    /// From a BIP39 mnemonic phrase (+ optional passphrase).
    pub fn from_mnemonic(phrase: &str, passphrase: &str) -> Result<Self> {
        Ok(Self {
            seed: WalletSeed::from_mnemonic(phrase, passphrase)?,
        })
    }

    /// From raw BIP39 seed bytes. The BIP32 network kind only affects
    /// xprv/xpub serialization, never derived keys; Main is used throughout.
    pub fn from_seed(seed: &[u8]) -> Result<Self> {
        Ok(Self {
            seed: WalletSeed::from_seed(seed)?,
        })
    }

    /// The underlying [`WalletSeed`] — the standard on-chain wallet branches
    /// (BIP-84/BIP-86 descriptors) of the SAME mnemonic. The purposes
    /// (84'/86') are disjoint from the Pact tree (7228') by construction.
    pub fn wallet(&self) -> &WalletSeed {
        &self.seed
    }

    fn derive(&self, path: &[u32]) -> Result<SecretKey> {
        self.seed.derive_hardened(path)
    }

    /// Identity keypair at `m/7228'/0'/0'` — signs handshake messages
    /// (BIP340 Schnorr); never used in any HTLC.
    pub fn identity_keypair(&self) -> Result<Keypair> {
        Ok(self.derive(&[PURPOSE, 0, 0])?.keypair(self.seed.secp()))
    }

    /// x-only identity pubkey (the `from` field of message envelopes).
    pub fn identity_pubkey(&self) -> Result<XOnlyPublicKey> {
        Ok(self.identity_keypair()?.x_only_public_key().0)
    }

    /// Swap secret key at `m/7228'/1'/coin'/scope_hi'/scope_lo'/index'` (HTLC
    /// redeem/refund, ECDSA), or the legacy `m/7228'/1'/coin'/index'` when
    /// `scope` is LEGACY. One key per chain per swap; never reused across swaps,
    /// and — with a real scope — never shared across machines on the same seed.
    pub fn swap_secret_key(&self, coin: u32, scope: DeriveScope, index: u32) -> Result<SecretKey> {
        self.derive(&scoped_path(&[PURPOSE, 1, coin], scope, index))
    }

    /// Compressed pubkey for [`Self::swap_secret_key`].
    pub fn swap_pubkey(&self, coin: u32, scope: DeriveScope, index: u32) -> Result<PublicKey> {
        Ok(self
            .swap_secret_key(coin, scope, index)?
            .public_key(self.seed.secp()))
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
            .public_key(self.seed.secp()))
    }

    /// Deterministic preimage for swap index `i` (spec §4.3) —
    /// `s = TaggedHash("pact/htlc/preimage/v1", key at m/7228'/2'/scope…/i')`
    /// (branch 2 carries no coin level; LEGACY drops the scope levels).
    /// Initiator only. Re-derivable from the seed + the swap's scope alone.
    pub fn preimage(&self, scope: DeriveScope, index: u32) -> Result<[u8; 32]> {
        let k = self.derive(&scoped_path(&[PURPOSE, 2], scope, index))?;
        Ok(tagged_hash(TAG_PREIMAGE, &k.secret_bytes()))
    }

    // ---- v2 (pact-htlc-v2): Taproot / MuSig2 adaptor swaps (spec v2 §3) ----

    /// x-only form of the swap key — the BIP340 / MuSig2 signer for a Taproot
    /// key-path spend. Same derivation as [`Self::swap_secret_key`]; only the
    /// public encoding differs (v1 uses it as compressed ECDSA, v2 as x-only).
    pub fn swap_xonly_pubkey(
        &self,
        coin: u32,
        scope: DeriveScope,
        index: u32,
    ) -> Result<XOnlyPublicKey> {
        Ok(self
            .swap_secret_key(coin, scope, index)?
            .x_only_public_key(self.seed.secp())
            .0)
    }

    /// Refund key at `m/7228'/3'/coin'/scope_hi'/scope_lo'/index'` (or legacy
    /// `m/7228'/3'/coin'/index'`) — signs the single-key CLTV refund tapleaf
    /// (spec v2 §3, §4). A *separate* branch from the MuSig2 swap key so the
    /// refund path is single-sig and independent.
    pub fn refund_secret_key(
        &self,
        coin: u32,
        scope: DeriveScope,
        index: u32,
    ) -> Result<SecretKey> {
        self.derive(&scoped_path(&[PURPOSE, 3, coin], scope, index))
    }

    /// x-only pubkey for [`Self::refund_secret_key`] (the refund tapleaf key).
    pub fn refund_xonly_pubkey(
        &self,
        coin: u32,
        scope: DeriveScope,
        index: u32,
    ) -> Result<XOnlyPublicKey> {
        Ok(self
            .refund_secret_key(coin, scope, index)?
            .x_only_public_key(self.seed.secp())
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
            .x_only_public_key(self.seed.secp())
            .0)
    }

    /// Deterministic adaptor secret `t` for swap index `i` (spec v2 §3.1) —
    /// `t = TaggedHash("pact/adaptor/secret/v2", key at m/7228'/2'/scope…/i')`,
    /// as a valid secp256k1 scalar (branch 2, same key as [`Self::preimage`] but
    /// a different tagged-hash domain, so the scope carries over for free).
    /// Initiator only; the v2 analog of the v1 preimage. Re-derivable from the
    /// seed + the swap's scope alone, so losing the state DB never loses it.
    pub fn adaptor_secret(&self, scope: DeriveScope, index: u32) -> Result<SecretKey> {
        let k = self.derive(&scoped_path(&[PURPOSE, 2], scope, index))?;
        let bytes = tagged_hash(TAG_ADAPTOR, &k.secret_bytes());
        SecretKey::from_slice(&bytes)
            .map_err(|e| anyhow::anyhow!("adaptor secret not a valid scalar: {e}"))
    }

    /// The adaptor point `T = t·G` (compressed pubkey) — shared in `init`,
    /// not secret (spec v2 §3.1).
    pub fn adaptor_point(&self, scope: DeriveScope, index: u32) -> Result<PublicKey> {
        Ok(self
            .adaptor_secret(scope, index)?
            .public_key(self.seed.secp()))
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

    /// The pre-scope legacy path (unchanged derivations).
    const LEG: DeriveScope = DeriveScope::LEGACY;

    #[test]
    fn deterministic_and_distinct() {
        let a = seed();
        let b = seed();
        assert_eq!(a.preimage(LEG, 0).unwrap(), b.preimage(LEG, 0).unwrap());
        assert_ne!(a.preimage(LEG, 0).unwrap(), a.preimage(LEG, 1).unwrap());
        assert_eq!(
            a.swap_pubkey(COIN_BTCX, LEG, 0).unwrap(),
            b.swap_pubkey(COIN_BTCX, LEG, 0).unwrap()
        );
        assert_ne!(
            a.swap_pubkey(COIN_BTCX, LEG, 0).unwrap(),
            a.swap_pubkey(COIN_BTC, LEG, 0).unwrap()
        );
    }

    #[test]
    fn btcx_coin_type_is_network_aware() {
        // Mainnet keeps the registered per-asset coin type (Phoenix↔Satchel
        // portable); the test networks fall back to SLIP-44 1' (Core parity).
        assert_eq!(btcx_coin_type(Network::Mainnet), COIN_BTCX);
        assert_eq!(btcx_coin_type(Network::Testnet), COIN_TESTNET);
        assert_eq!(btcx_coin_type(Network::Regtest), COIN_TESTNET);
        assert_eq!(COIN_TESTNET, 1);
        // Only BTCX deviates on test nets — mainnet stays on the canonical value.
        assert_ne!(btcx_coin_type(Network::Regtest), COIN_BTCX);
    }

    #[test]
    fn preimage_is_32_bytes_and_hashes() {
        let s = seed().preimage(LEG, 0).unwrap();
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
        assert_ne!(ident, s.swap_pubkey(COIN_BTC, LEG, 0).unwrap());
    }

    /// The seed-scope backbone (§1): a real scope must change every
    /// counter-based initiator secret at the SAME index — this is the whole
    /// cross-machine partition. Two distinct scopes, and the legacy path, must
    /// all diverge; the same scope must reproduce.
    #[test]
    fn scope_partitions_initiator_secrets() {
        let s = seed();
        let s1 = DeriveScope(0x0000_0000_1234_5678);
        let s2 = DeriveScope(0x0000_0003_9abc_def0 & DeriveScope::MASK);
        // Preimage / hash H → swap_id all diverge by scope at the same index.
        assert_ne!(s.preimage(s1, 0).unwrap(), s.preimage(LEG, 0).unwrap());
        assert_ne!(s.preimage(s1, 0).unwrap(), s.preimage(s2, 0).unwrap());
        assert_eq!(s.preimage(s1, 0).unwrap(), seed().preimage(s1, 0).unwrap());
        // Adaptor secret t (branch 2, same key as preimage) diverges too.
        assert_ne!(
            s.adaptor_secret(s1, 0).unwrap(),
            s.adaptor_secret(s2, 0).unwrap()
        );
        // Swap key (branch 1) and refund key (branch 3) diverge by scope.
        assert_ne!(
            s.swap_pubkey(COIN_BTC, s1, 0).unwrap(),
            s.swap_pubkey(COIN_BTC, LEG, 0).unwrap()
        );
        assert_ne!(
            s.refund_xonly_pubkey(COIN_BTC, s1, 0).unwrap(),
            s.refund_xonly_pubkey(COIN_BTC, s2, 0).unwrap()
        );
        // A scoped key must never collide with any anchored (participant) key.
        let h = [0x44u8; 32];
        let anchored = s.swap_secret_key_anchored(COIN_BTC, &h).unwrap();
        assert_ne!(anchored, s.swap_secret_key(COIN_BTC, s1, 0).unwrap());
    }

    // ---- v2 derivations ----

    #[test]
    fn adaptor_secret_deterministic_and_point_matches() {
        let secp = bitcoin::secp256k1::Secp256k1::new();
        let (a, b) = (seed(), seed());
        // Deterministic across instances, distinct per index.
        assert_eq!(
            a.adaptor_secret(LEG, 0).unwrap(),
            b.adaptor_secret(LEG, 0).unwrap()
        );
        assert_ne!(
            a.adaptor_secret(LEG, 0).unwrap(),
            a.adaptor_secret(LEG, 1).unwrap()
        );
        // T = t·G.
        let t = a.adaptor_secret(LEG, 3).unwrap();
        assert_eq!(a.adaptor_point(LEG, 3).unwrap(), t.public_key(&secp));
        // The v2 adaptor secret is NOT the v1 preimage (different tag).
        assert_ne!(
            a.adaptor_secret(LEG, 0).unwrap().secret_bytes(),
            a.preimage(LEG, 0).unwrap()
        );
    }

    #[test]
    fn refund_key_is_separate_branch() {
        let s = seed();
        // Deterministic, distinct per coin/index.
        assert_eq!(
            s.refund_xonly_pubkey(COIN_BTC, LEG, 0).unwrap(),
            seed().refund_xonly_pubkey(COIN_BTC, LEG, 0).unwrap()
        );
        assert_ne!(
            s.refund_secret_key(COIN_BTC, LEG, 0).unwrap(),
            s.refund_secret_key(COIN_BTCX, LEG, 0).unwrap()
        );
        // Branch 3' refund key is independent of the branch 1' swap key.
        assert_ne!(
            s.refund_secret_key(COIN_BTC, LEG, 0).unwrap(),
            s.swap_secret_key(COIN_BTC, LEG, 0).unwrap()
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
            s.swap_secret_key_anchored(COIN_BTCX, &h1).unwrap()
        );
        // Refund branch is independent of the swap branch for the same anchor.
        assert_ne!(
            s.refund_secret_key_anchored(COIN_BTC, &h1).unwrap(),
            s.swap_secret_key_anchored(COIN_BTC, &h1).unwrap()
        );
        // A 33-byte anchor (v2 compressed point) works and differs.
        let t = s.adaptor_point(DeriveScope::LEGACY, 0).unwrap().serialize();
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
            assert_ne!(anchored, s.swap_secret_key(COIN_BTC, LEG, i).unwrap());
        }
    }

    // ---- nodeless wallet branch (keys-btcx; docs/NODELESS_WALLET.md D1) ----

    #[test]
    fn pact_tree_is_disjoint_from_wallet_branches() {
        // The Pact purpose (7228') can never collide with the standard wallet
        // purposes (84'/86') the underlying WalletSeed derives — same master
        // key, different first level.
        let s = seed();
        let account = s
            .wallet()
            .wallet_account_xpriv(DescriptorKind::Bip86, COIN_BTC)
            .unwrap();
        assert_ne!(
            account.private_key,
            s.derive(&[PURPOSE, 0, 0]).unwrap(),
            "wallet account key must not equal a Pact-tree key"
        );
        // And the wrapper really shares the master key with the wallet seed:
        // deriving m/86'/0'/0' through either path gives the same key.
        assert_eq!(
            account.private_key,
            s.wallet().derive_hardened(&[86, COIN_BTC, 0]).unwrap()
        );
    }

    #[test]
    fn swap_xonly_matches_swap_pubkey() {
        let s = seed();
        let xonly = s.swap_xonly_pubkey(COIN_BTCX, LEG, 0).unwrap();
        assert_eq!(
            xonly,
            s.swap_pubkey(COIN_BTCX, LEG, 0)
                .unwrap()
                .x_only_public_key()
                .0
        );
    }
}
