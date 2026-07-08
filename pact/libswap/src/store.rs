//! Persistence — spec §11.
//!
//! The transcript-equivalent swap state is part of the *safety* state, not
//! a cache: it MUST be durable before any funding broadcast. Swap records
//! are stored as JSON blobs in SQLite (one row per swap) plus a counter
//! for the next BIP32 swap index.
//!
//! Seed storage (#120): a user passphrase encrypts the BIP39 mnemonic with
//! scrypt + ChaCha20-Poly1305 (`PACTSEEDv1`). With NO passphrase the seed is
//! still never written plaintext — it is ChaCha20-Poly1305'd under a machine key
//! held in the OS keystore (`PACTSEEDv2-keyring`, Windows/macOS only), or, where
//! no keystore is compiled/available (all Linux — see the Cargo target gate),
//! under a built-in obfuscation key (`PACTSEEDv2-obfs`). The obfuscation key
//! ships in the binary, so an obfs seed is treated as UNENCRYPTED wherever trust
//! is decided (mainnet gate, `walletstatus`); it only lifts the file off
//! plaintext ASCII. The machine key auto-unlocks, so the daemon keeps signing
//! across restarts; only a passphrase seed can be `locked`.

use anyhow::{bail, Context, Result};
use chacha20poly1305::aead::Aead;
use chacha20poly1305::{ChaCha20Poly1305, KeyInit};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::adaptor_swap::AdaptorState;

/// One unsent Nostr outbox row: `(id, kind, recipient, payload)`.
pub type NostrOutboxRow = (i64, String, Option<String>, String);
use crate::keys::PactSeed;
use crate::messages::ChainRef;
use crate::swap::{Role, State};

pub const SEED_FILE: &str = "seed.mnemonic";
pub const DB_FILE: &str = "pact.sqlite";
const SEED_MAGIC: &str = "PACTSEEDv1";
/// scrypt cost: N=2^15, r=8, p=1 (~30 MB, tens of ms) — interactive-grade.
const SCRYPT_LOG_N: u8 = 15;
/// The two *unattended* at-rest wraps (#120), used when no passphrase is set.
/// `-keyring`: `MAGIC:key_id:nonce:ct`, key held in the OS keystore under
/// `key_id`. `-obfs`: `MAGIC:nonce:ct`, the fallback constant key.
const SEED_V2_KEYRING: &str = "PACTSEEDv2-keyring";
const SEED_V2_OBFS: &str = "PACTSEEDv2-obfs";
/// OS-keystore service name; the per-seed account is the random `key_id` stored
/// in the seed line itself (so the entry survives a same-machine folder move,
/// but not a copy to a different machine — which is the point). Only referenced
/// by the Windows/macOS keystore path, so it's gated to avoid an unused-const
/// warning on Linux (where the seed always takes the obfuscation wrap).
#[cfg(any(windows, target_os = "macos"))]
const KEYRING_SERVICE: &str = "pactd-seed";
/// The obfuscation fallback key. This is NOT a secret — it ships in the
/// open-source binary. It only raises a no-keystore seed from plaintext ASCII to
/// a binary blob (Bitcoin-Core-unencrypted parity) and is always treated as
/// UNENCRYPTED for trust. Bytes spell "PACT-seed-obfs-v2-do-not-trust!!".
const OBFUSCATION_KEY: [u8; 32] = [
    0x50, 0x41, 0x43, 0x54, 0x2d, 0x73, 0x65, 0x65, 0x64, 0x2d, 0x6f, 0x62, 0x66, 0x73, 0x2d, 0x76,
    0x32, 0x2d, 0x64, 0x6f, 0x2d, 0x6e, 0x6f, 0x74, 0x2d, 0x74, 0x72, 0x75, 0x73, 0x74, 0x21, 0x21,
];

/// One party's durable view of one swap. Hex fields use lowercase hex;
/// txids are big-endian display order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapRecord {
    pub swap_id: String,
    pub role: Role,
    pub state: State,
    /// When this swap was first recorded (unix seconds, `engine::local_now`).
    /// Lets history sort by time. Records persisted before this field existed
    /// have no value in their JSON blob and deserialize to 0 (see migration
    /// note in [`Store::open`]).
    pub created_at: u64,
    /// Our local BIP32 swap index `i` (spec §4.2) — `Some` for the initiator,
    /// whose counter the swap id itself derives from; `None` for the
    /// participant, whose keys are anchored to `hash_h` instead (no counter).
    pub swap_index: Option<u32>,
    pub chain_a: ChainRef,
    pub chain_b: ChainRef,
    pub amount_a: u64,
    pub amount_b: u64,
    pub hash_h: String,
    pub t1: u32,
    pub t2: u32,
    pub n_a: u32,
    pub n_b: u32,
    pub alice_refund_pubkey_a: String,
    pub alice_redeem_pubkey_b: String,
    pub bob_redeem_pubkey_a: Option<String>,
    pub bob_refund_pubkey_b: Option<String>,
    /// Counterparty identity pubkey, pinned from their first message (§8.2).
    pub counterparty_identity: Option<String>,
    pub htlc_a_txid: Option<String>,
    pub htlc_a_vout: Option<u32>,
    pub htlc_b_txid: Option<String>,
    pub htlc_b_vout: Option<u32>,
    /// Tip height when the chain-B HTLC was recorded — spend-scan start.
    pub htlc_b_height: Option<u64>,
    /// The preimage, once known. For the initiator this is derivable from
    /// the seed; for the participant it is learned from chain B.
    pub preimage: Option<String>,
    /// Refund transaction for the leg we funded, signed at funding time
    /// (spec §6.3) and broadcast by the scheduler once MTP >= T.
    pub refund_tx_hex: Option<String>,
    /// Txid of our redeem/refund, once broadcast.
    pub final_txid: Option<String>,
    /// Full hex of that spend — kept for RBF fee-bumping and rebroadcast
    /// while unconfirmed (spec §7.4).
    pub final_tx_hex: Option<String>,
    /// Chain tip height at which the nurse last *acted* (broadcast a bump or
    /// replacement) on this swap's pending tx. The bump loop backs out when the
    /// tip hasn't advanced since (≤1 action per block), turning the 30s poll
    /// into block-driven cadence. Defaults to 0 for records persisted before
    /// this field existed and for fresh swaps — 0 ≠ any real tip, so the first
    /// post-load tick is free to act.
    #[serde(default)]
    pub last_action_height: u64,
}

/// One party's durable view of one **v2** (adaptor) swap (spec v2 §9).
/// Mirrors [`SwapRecord`] but for the Taproot/MuSig2 flow: keys are x-only /
/// full points, the secret is the adaptor `t`/`T`, and the handshake carries
/// per-leg nonces + partial signatures. Stored as a JSON blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptorSwapRecord {
    pub swap_id: String,
    pub role: Role,
    pub state: AdaptorState,
    pub created_at: u64,
    /// `Some` for the initiator (local counter — the adaptor secret `t`, and
    /// so `T` and the swap id, derive from it); `None` for the participant,
    /// whose keys are anchored to `adaptor_point` instead (spec §4.2).
    pub swap_index: Option<u32>,
    pub chain_a: ChainRef,
    pub chain_b: ChainRef,
    pub amount_a: u64,
    pub amount_b: u64,
    pub t1: u32,
    pub t2: u32,
    /// Confirmation depth (reorg-safety) for leg A / leg B, resolved from the
    /// local per-coin setting at init/accept (spec v2 §8, inheriting v1 §7.3).
    /// `n_b` gates the initiator's reveal (don't publish `t` until Bob's leg-B
    /// funding is this deep) and the redeem-completion check; `n_a` the
    /// leg-A redeem completion. Local policy, not consensus — each party sets
    /// these from its own config.
    pub n_a: u32,
    pub n_b: u32,
    /// Adaptor point `T` (compressed hex). The secret `t` is seed-derived by
    /// the initiator and never stored.
    pub adaptor_point: String,
    // MuSig2 signer keys (full, compressed hex) + funder refund keys (x-only).
    pub alice_swap_a: String,
    pub alice_swap_b: String,
    pub alice_refund_a: String,
    pub bob_swap_a: Option<String>,
    pub bob_swap_b: Option<String>,
    pub bob_refund_b: Option<String>,
    /// Fresh core-wallet sweep addresses for the cooperative redeems,
    /// communicated in init/accept so both parties build the identical redeem tx
    /// and the proceeds land in a spendable core wallet (not a swap-key addr).
    /// `sweep_a` = where leg A is redeemed (Bob's addr); `sweep_b` = leg B
    /// (Alice's addr). `None` → the deterministic swap-key fallback.
    pub sweep_a: Option<String>,
    pub sweep_b: Option<String>,
    /// Negotiated cooperative-redeem feerates (sat/vB), one per chain, fixed at
    /// init (see [`crate::messages::InitV2Body::redeem_feerate_a`]). Both parties
    /// store the SAME values (the initiator's, carried in the signed init) so the
    /// redeem txs — and their MuSig2 sighashes — are byte-identical. The fee is
    /// committed into the adaptor signature and unbumpable, so the rate is
    /// over-provisioned at init (M2).
    pub redeem_feerate_a: u64,
    pub redeem_feerate_b: u64,
    pub counterparty_identity: Option<String>,
    // Funding outpoints (built before broadcast, spec v2 §7).
    pub funding_a_txid: Option<String>,
    pub funding_a_vout: Option<u32>,
    pub funding_b_txid: Option<String>,
    pub funding_b_vout: Option<u32>,
    // Counterparty handshake material (hex), per redeem session.
    pub their_pubnonce_a: Option<String>,
    pub their_pubnonce_b: Option<String>,
    pub their_partial_a: Option<String>,
    pub their_partial_b: Option<String>,
    // Assembled adaptor signatures (hex), once both partials are in.
    pub adaptor_sig_a: Option<String>,
    pub adaptor_sig_b: Option<String>,
    pub final_txid_a: Option<String>,
    pub final_txid_b: Option<String>,
    /// Full hex of our last-broadcast spend on each leg, kept while it is
    /// unconfirmed so the scheduler can rebroadcast, RBF-bump (the single-key
    /// refund), or CPFP-bump (the cooperative redeem, whose own fee is locked
    /// into the pre-signed adaptor signature — v2+). See spec/protocol-v2.md.
    pub final_tx_a_hex: Option<String>,
    pub final_tx_b_hex: Option<String>,
    /// Chain tip height at which a nurse last *acted* (RBF/CPFP/replacement) on
    /// this swap. Backs out the bump loop when the tip hasn't advanced since
    /// (≤1 action per block). See [`SwapRecord::last_action_height`]; defaults
    /// to 0 (pre-existing records / fresh swaps → first tick may act).
    #[serde(default)]
    pub last_action_height: u64,
    /// The participant's leg-B funding tx, BUILT but not yet broadcast (spec v2
    /// §7 two-phase: the redeems are signed over its outpoint, and it is
    /// broadcast only after the swap is `Signed` and leg A is verified on-chain
    /// `n_a`-deep — so the participant never commits leg B before it can claim
    /// leg A). Persisted so a crash between build and broadcast rebroadcasts the
    /// exact tx the adaptor signatures commit to. `None` for the initiator (which
    /// funds leg A directly).
    #[serde(default)]
    pub funding_b_tx_hex: Option<String>,
    /// Set once the participant has broadcast its pre-built leg-B funding, so the
    /// scheduler broadcasts it exactly once.
    #[serde(default)]
    pub funding_b_broadcast: bool,
}

pub struct Store {
    conn: Connection,
    data_dir: PathBuf,
    passphrase: Option<String>,
}

/// Seed-lifecycle status for the `walletstatus` RPC and the Satchel wizard.
///
/// A merchant with no seed yet is in first-run state (`seed_exists=false`).
/// An `encrypted` seed with no passphrase loaded is `locked`: the daemon is
/// up but cannot sign until an `unlock` (or a restart with PACT_PASSPHRASE).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WalletStatus {
    pub seed_exists: bool,
    pub encrypted: bool,
    pub locked: bool,
}

/// A maker's own posted offer (the `my_offers` registry row). `valid_for` is
/// the maker-set lifetime in seconds (final expiry = `created + valid_for`);
/// the rolling relay TTL is a separate system constant applied at publish time.
#[derive(Debug, Clone)]
pub struct MyOffer {
    pub offer_id: String,
    pub envelope: String,
    pub created: u64,
    pub valid_for: u64,
    pub last_refresh: u64,
    pub state: String,
}

impl Store {
    /// Create a fresh data dir with a new random seed (encrypted when a
    /// passphrase is given). Fails if a seed already exists — never
    /// overwrite key material.
    pub fn init(data_dir: &Path, passphrase: Option<&str>) -> Result<Self> {
        let mut store = Self::open(data_dir, None)?;
        store.create_seed(passphrase, 12)?;
        Ok(store)
    }

    pub fn open(data_dir: &Path, passphrase: Option<&str>) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let conn = Connection::open(data_dir.join(DB_FILE))?;
        conn.busy_timeout(Duration::from_secs(10))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS swaps (
                 swap_id TEXT PRIMARY KEY,
                 record  TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS meta (
                 key   TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS pending_takes (
                 offer_id   TEXT PRIMARY KEY,
                 offer      TEXT NOT NULL,
                 created_at INTEGER NOT NULL
             );
             -- v2 MuSig2 use-once nonce sessions (spec v2 §3.2). One row per
             -- (swap, leg) signing session; state advances monotonically
             -- none(absent) -> committed -> revealed -> consumed. The secret
             -- nonce is written BEFORE its public nonce is released, and never
             -- regenerated; on resume we reload this row. See spec/protocol-v2.md
             -- nonce-safety design.
             CREATE TABLE IF NOT EXISTS nonce_sessions (
                 swap_id     TEXT NOT NULL,
                 leg         TEXT NOT NULL,
                 state       TEXT NOT NULL,
                 secnonce    BLOB NOT NULL,
                 partial_sig BLOB,
                 PRIMARY KEY (swap_id, leg)
             );
             -- v2 (pact-htlc-v2) adaptor swaps, one JSON blob per swap.
             CREATE TABLE IF NOT EXISTS adaptor_swaps (
                 swap_id TEXT PRIMARY KEY,
                 record  TEXT NOT NULL
             );
             -- Nostr transport (spec/protocol.md §8.8): the async relay
             -- service buffers all I/O through these local tables so the
             -- sync engine only ever touches SQLite. `nostr_inbox.id` is a
             -- local autoincrement, which lets NostrBoard mimic the HTTP
             -- relay's (id, blob) cursor contract unchanged.
             CREATE TABLE IF NOT EXISTS nostr_outbox (
                 id        INTEGER PRIMARY KEY AUTOINCREMENT,
                 kind      TEXT NOT NULL,            -- offer | revoke | giftwrap
                 recipient TEXT,                     -- x-only pubkey hex (giftwrap)
                 payload   TEXT NOT NULL,            -- offer envelope JSON, or sealed blob
                 created   INTEGER NOT NULL,
                 sent      INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE IF NOT EXISTS nostr_inbox (
                 id       INTEGER PRIMARY KEY AUTOINCREMENT,
                 event_id TEXT NOT NULL UNIQUE,      -- nostr event id, for cross-relay dedup
                 blob     TEXT NOT NULL,             -- inner PACTSEALED1 blob (gift-wrap unwrapped)
                 created  INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS nostr_offer_cache (
                 event_id TEXT PRIMARY KEY,
                 d_tag    TEXT NOT NULL,             -- swap_id (addressable identifier)
                 envelope TEXT NOT NULL,             -- signed offer envelope JSON
                 created  INTEGER NOT NULL,
                 expires  INTEGER NOT NULL DEFAULT 0 -- 0 = no NIP-40 expiry
             );
             -- The maker's OWN posted offers (offer-lifecycle). Drives the refresh
             -- loop, revoke-on-close, and the My-offers view. The signed envelope is
             -- kept so a live offer can be re-published (Nostr addressable replace /
             -- HTTP re-POST) to roll its short relay TTL forward until
             -- `created + valid_for` (the maker-set FINAL expiry). state advances
             -- live -> taken | revoked | expired.
             CREATE TABLE IF NOT EXISTS my_offers (
                 offer_id     TEXT PRIMARY KEY,      -- = offer envelope swap_id
                 envelope     TEXT NOT NULL,         -- signed offer envelope JSON
                 created      INTEGER NOT NULL,      -- body.created (post time)
                 valid_for    INTEGER NOT NULL,      -- ttl_secs; 0 = no expiry
                 last_refresh INTEGER NOT NULL DEFAULT 0,
                 state        TEXT NOT NULL DEFAULT 'live'
             );",
        )?;
        let store = Self {
            conn,
            data_dir: data_dir.to_path_buf(),
            passphrase: passphrase.map(str::to_string),
        };
        // #120: bring any pre-existing plaintext seed up to the never-plaintext
        // bar in place. Best-effort — a keystore/disk hiccup must never block
        // startup; the next boot retries. Only touches plaintext (see below).
        if let Err(e) = store.migrate_seed_at_rest() {
            eprintln!("seed: at-rest migration skipped ({e:#})");
        }
        Ok(store)
    }

    /// #120: re-wrap an existing *plaintext* seed in place (OS-keystore key, or
    /// obfuscation + warning) so a legacy seed stops sitting on disk as ASCII.
    /// Only ever touches a plaintext file — passphrase (`PACTSEEDv1`) and
    /// already-wrapped (`PACTSEEDv2-*`) seeds are left exactly as they are, so
    /// this can never *downgrade* a stronger wrap (guardrail 2).
    fn migrate_seed_at_rest(&self) -> Result<()> {
        let seed_path = self.data_dir.join(SEED_FILE);
        let Ok(contents) = std::fs::read_to_string(&seed_path) else {
            return Ok(()); // no seed yet
        };
        let line = contents.trim();
        if line.is_empty()
            || line.starts_with(SEED_MAGIC)
            || line.starts_with(SEED_V2_KEYRING)
            || line.starts_with(SEED_V2_OBFS)
        {
            return Ok(()); // already wrapped (or empty) — nothing to migrate
        }
        let wrapped = wrap_unattended(line)?;
        write_seed_atomic(&seed_path, &wrapped)?;
        eprintln!(
            "seed: migrated a plaintext seed to encrypted-at-rest ({})",
            seed_path.display()
        );
        Ok(())
    }

    /// Seed-lifecycle snapshot — drives `walletstatus`, the first-run wizard,
    /// and the lock/unlock UX. Cheap: no scrypt, just a file probe.
    pub fn wallet_status(&self) -> Result<WalletStatus> {
        let path = self.data_dir.join(SEED_FILE);
        if !path.exists() {
            return Ok(WalletStatus {
                seed_exists: false,
                encrypted: false,
                locked: false,
            });
        }
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("reading seed at {}", path.display()))?;
        let encrypted = is_encrypted_seed_file(&contents);
        // Only a passphrase seed can be *locked*: it needs an in-memory
        // passphrase to read. Keyring seeds auto-unlock from the OS keystore, so
        // they are encrypted-but-never-locked (a keyring seed the machine can no
        // longer decrypt surfaces at read time as a reconfirm-with-mnemonic
        // error, #120 guardrail 2 — not as a lock). We only ever hold a
        // passphrase that has actually decrypted the seed, so "held" ⇒ "usable".
        let locked = is_passphrase_seed_file(&contents) && self.passphrase.is_none();
        Ok(WalletStatus {
            seed_exists: true,
            encrypted,
            locked,
        })
    }

    /// Write a mnemonic to disk and adopt it as this Store's live seed. A
    /// non-empty passphrase encrypts it (`PACTSEEDv1`); otherwise it is wrapped
    /// unattended (#120: OS-keystore key, or obfuscation) — never plaintext.
    /// Refuses to clobber a seed we can still read.
    fn install_seed(&mut self, phrase: &str, passphrase: Option<&str>) -> Result<()> {
        let seed_path = self.data_dir.join(SEED_FILE);
        if seed_path.exists() {
            // The one clobber we allow is the reconfirm-with-mnemonic recovery
            // (#120 guardrail 2): a `PACTSEEDv2-keyring` seed whose OS-keystore
            // key has vanished (moved to a new machine / reset keychain) is
            // unreadable, so re-importing the mnemonic re-provisions it under a
            // fresh machine key. A seed we can still read is never overwritten.
            let existing = std::fs::read_to_string(&seed_path).unwrap_or_default();
            let unreadable_keyring =
                existing.trim().starts_with(SEED_V2_KEYRING) && self.seed().is_err();
            anyhow::ensure!(
                unreadable_keyring,
                "{} already exists — refusing to overwrite a seed",
                seed_path.display()
            );
        }
        let pass = passphrase.filter(|p| !p.is_empty());
        let contents = match pass {
            Some(pass) => encrypt_seed(phrase, pass)?,
            None => wrap_unattended(phrase)?,
        };
        write_seed_atomic(&seed_path, &contents)?;
        self.passphrase = pass.map(str::to_string);
        Ok(())
    }

    /// Generate a new random BIP39 seed and return the mnemonic **once** for
    /// the user to back up — Satchel keeps no recovery copy. Encrypted when a
    /// passphrase is supplied. `words` is 12 or 24 (phoenix parity): 12
    /// (128-bit) is the DEFAULT — this is a hot transit wallet, not custody
    /// storage, and 128 bits already matches secp256k1's security level — 24
    /// (256-bit) for those who want the longer phrase.
    pub fn create_seed(&mut self, passphrase: Option<&str>, words: usize) -> Result<String> {
        let phrase = self.generate_mnemonic(words)?;
        self.install_seed(&phrase, passphrase)?;
        Ok(phrase)
    }

    /// Generate a fresh random BIP39 mnemonic **without persisting it** — for an
    /// onboarding flow that shows + confirms the phrase before committing. The
    /// mnemonic is only written once it's passed back to [`Self::import_seed`].
    /// `words`: 12 or 24, see [`Self::create_seed`].
    pub fn generate_mnemonic(&self, words: usize) -> Result<String> {
        let bytes = match words {
            12 => 16,
            24 => 32,
            n => bail!("seed length must be 12 or 24 words, not {n}"),
        };
        let mut entropy = [0u8; 32];
        use bitcoin::secp256k1::rand::RngCore;
        bitcoin::secp256k1::rand::thread_rng().fill_bytes(&mut entropy[..bytes]);
        Ok(bip39::Mnemonic::from_entropy(&entropy[..bytes])?.to_string())
    }

    /// Import a user-supplied BIP39 mnemonic (validated). Returns the
    /// normalized phrase. Encrypted when a passphrase is supplied.
    pub fn import_seed(&mut self, mnemonic: &str, passphrase: Option<&str>) -> Result<String> {
        let parsed = bip39::Mnemonic::parse_normalized(mnemonic.trim())
            .context("not a valid BIP39 mnemonic")?;
        let phrase = parsed.to_string();
        self.install_seed(&phrase, passphrase)?;
        Ok(phrase)
    }

    /// Supply the passphrase for an existing encrypted seed, verifying it by
    /// trial decryption before holding it in memory (`lncli unlock`-style).
    /// Idempotent on an already-unlocked store; a no-op error on plaintext.
    pub fn unlock(&mut self, passphrase: &str) -> Result<()> {
        let path = self.data_dir.join(SEED_FILE);
        let contents =
            std::fs::read_to_string(&path).context("no seed yet — create or import one first")?;
        // Only a passphrase seed needs unlocking; keyring/obfs auto-read.
        anyhow::ensure!(
            is_passphrase_seed_file(&contents),
            "seed is not passphrase-encrypted — no unlock needed"
        );
        // Errors (wrong passphrase) before we adopt anything.
        decrypt_seed(contents.trim(), passphrase)?;
        self.passphrase = Some(passphrase.to_string());
        Ok(())
    }

    /// Whether the on-disk seed is *encrypted* — the file is useless without an
    /// external secret (a passphrase, or the OS-keystore key). Obfuscation and
    /// plaintext are NOT encrypted. Callers gate networks on this (#120).
    pub fn seed_is_encrypted(&self) -> Result<bool> {
        let path = self.data_dir.join(SEED_FILE);
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("no seed at {} — run `pact init` first", path.display()))?;
        Ok(is_encrypted_seed_file(&contents))
    }

    pub fn seed(&self) -> Result<PactSeed> {
        let path = self.data_dir.join(SEED_FILE);
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("no seed at {} — run `pact init` first", path.display()))?;
        let mnemonic = self.decrypt_contents(contents.trim())?;
        PactSeed::from_mnemonic(&mnemonic, "")
    }

    /// Decrypt on-disk seed contents to the raw mnemonic, dispatching on the
    /// wrap: passphrase (`PACTSEEDv1`, needs the in-memory passphrase), keyring
    /// (`PACTSEEDv2-keyring`, key from the OS keystore), obfuscation
    /// (`PACTSEEDv2-obfs`), or legacy plaintext (pre-#120, not yet migrated).
    fn decrypt_contents(&self, line: &str) -> Result<String> {
        if line.starts_with(SEED_MAGIC) {
            let pass = self
                .passphrase
                .as_deref()
                .context("seed is encrypted — set PACT_PASSPHRASE or run `unlock`")?;
            decrypt_seed(line, pass)
        } else if let Some(rest) = line.strip_prefix(&format!("{SEED_V2_KEYRING}:")) {
            let mut parts = rest.split(':');
            let key_id = parts.next().context("malformed keyring seed")?;
            let nonce = parts.next().context("malformed keyring seed")?;
            let ct = parts.next().context("malformed keyring seed")?;
            let key = keyring_get(key_id).context(
                "this machine can no longer unlock the seed (OS-keystore key missing) — \
                 re-import your recovery phrase to continue",
            )?;
            decrypt_v2(nonce, ct, &key)
        } else if let Some(rest) = line.strip_prefix(&format!("{SEED_V2_OBFS}:")) {
            let mut parts = rest.split(':');
            let nonce = parts.next().context("malformed obfs seed")?;
            let ct = parts.next().context("malformed obfs seed")?;
            decrypt_v2(nonce, ct, &OBFUSCATION_KEY)
        } else {
            // Legacy plaintext (migrated to a wrap on the next `Store::open`).
            Ok(line.to_string())
        }
    }

    /// Allocate the next BIP32 swap index (monotonic, never reused —
    /// spec §4.2 counts aborted attempts too).
    pub fn next_swap_index(&self) -> Result<u32> {
        let current = self.peek_next_swap_index()?;
        self.conn.execute(
            "INSERT INTO meta (key, value) VALUES ('next_swap_index', ?1)
             ON CONFLICT(key) DO UPDATE SET value = ?1",
            params![(current + 1).to_string()],
        )?;
        Ok(current)
    }

    /// Read the next swap index WITHOUT allocating it — used to stamp the
    /// counter into a rescue snapshot (issue #54).
    pub fn peek_next_swap_index(&self) -> Result<u32> {
        Ok(self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'next_swap_index'",
                [],
                |row| row.get::<_, String>(0),
            )
            .map(|v| v.parse().unwrap_or(0))
            .unwrap_or(0))
    }

    /// Raise the next-swap-index counter to at least `n` (never lowers it) — on
    /// rescue this restores the high-water mark from the backed-up snapshots so a
    /// fresh machine never reissues an index a completed swap already used (which
    /// would reuse HTLC/adaptor keys). Idempotent.
    pub fn set_next_swap_index_at_least(&self, n: u32) -> Result<()> {
        let current = self.peek_next_swap_index()?;
        if n > current {
            self.conn.execute(
                "INSERT INTO meta (key, value) VALUES ('next_swap_index', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = ?1",
                params![n.to_string()],
            )?;
        }
        Ok(())
    }

    pub fn put(&self, record: &SwapRecord) -> Result<()> {
        self.conn.execute(
            "INSERT INTO swaps (swap_id, record) VALUES (?1, ?2)
             ON CONFLICT(swap_id) DO UPDATE SET record = ?2",
            params![record.swap_id, serde_json::to_string(record)?],
        )?;
        Ok(())
    }

    pub fn get(&self, swap_id: &str) -> Result<SwapRecord> {
        let json: String = self
            .conn
            .query_row(
                "SELECT record FROM swaps WHERE swap_id = ?1",
                params![swap_id],
                |row| row.get(0),
            )
            .with_context(|| format!("unknown swap {swap_id}"))?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn list(&self) -> Result<Vec<SwapRecord>> {
        let mut stmt = self
            .conn
            .prepare("SELECT record FROM swaps ORDER BY swap_id")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.map(|json| Ok(serde_json::from_str(&json?)?)).collect()
    }

    pub fn meta_get(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .ok())
    }

    pub fn meta_set(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![key, value],
        )?;
        Ok(())
    }

    /// Delete a single meta row (no-op if absent).
    pub fn meta_del(&self, key: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM meta WHERE key = ?1", params![key])?;
        Ok(())
    }

    /// All `(key, value)` meta rows whose key starts with `prefix`, key-sorted.
    /// Used to enumerate locally-stored private offers (`private_offer:<id>`),
    /// which live in `meta` so the board-offer revoke/served guards apply to
    /// them unchanged.
    pub fn meta_with_prefix(&self, prefix: &str) -> Result<Vec<(String, String)>> {
        let pattern = format!("{}%", prefix.replace('%', "\\%").replace('_', "\\_"));
        let mut stmt = self
            .conn
            .prepare("SELECT key, value FROM meta WHERE key LIKE ?1 ESCAPE '\\' ORDER BY key")?;
        let rows = stmt.query_map(params![pattern], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Relay messages with id <= this cursor have been processed.
    pub fn relay_cursor(&self) -> Result<i64> {
        Ok(self
            .meta_get("relay_cursor")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(0))
    }

    pub fn set_relay_cursor(&self, cursor: i64) -> Result<()> {
        self.meta_set("relay_cursor", &cursor.to_string())
    }

    /// The persisted fee-bump policy for this merchant, or `None` if never set
    /// (callers fall back to [`FeeBumpPolicy::default`]). Stored as a JSON blob in
    /// `meta` so a CLI/RPC-set policy survives restart with no Satchel involved.
    pub fn fee_policy(&self) -> Result<Option<crate::FeeBumpPolicy>> {
        match self.meta_get("fee_policy")? {
            Some(json) => Ok(Some(
                serde_json::from_str(&json).context("corrupt stored fee_policy")?,
            )),
            None => Ok(None),
        }
    }

    pub fn set_fee_policy(&self, policy: &crate::FeeBumpPolicy) -> Result<()> {
        self.meta_set("fee_policy", &serde_json::to_string(policy)?)
    }

    // ---- Nostr transport buffers (spec/protocol.md §8.8) ----
    // NostrBoard (sync) and the relay service (async) communicate only
    // through these; neither calls the other directly.

    /// Queue an item for the relay service to publish. `kind` is one of
    /// `offer` | `revoke` | `giftwrap`; `recipient` is the x-only pubkey for
    /// `giftwrap` (None for offers/revokes).
    pub fn nostr_outbox_push(
        &self,
        kind: &str,
        recipient: Option<&str>,
        payload: &str,
        created: u64,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO nostr_outbox (kind, recipient, payload, created, sent)
             VALUES (?1, ?2, ?3, ?4, 0)",
            params![kind, recipient, payload, created as i64],
        )?;
        Ok(())
    }

    /// Unsent outbox rows in insertion order: `(id, kind, recipient, payload)`.
    pub fn nostr_outbox_pending(&self) -> Result<Vec<NostrOutboxRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, recipient, payload FROM nostr_outbox WHERE sent = 0 ORDER BY id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Mark an outbox row published.
    pub fn nostr_outbox_mark_sent(&self, id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE nostr_outbox SET sent = 1 WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Store a received gift-wrap's inner sealed blob, deduped by nostr
    /// event id across relays. Returns true if newly inserted (false = dup).
    pub fn nostr_inbox_insert(&self, event_id: &str, blob: &str, created: u64) -> Result<bool> {
        let n = self.conn.execute(
            "INSERT OR IGNORE INTO nostr_inbox (event_id, blob, created) VALUES (?1, ?2, ?3)",
            params![event_id, blob, created as i64],
        )?;
        Ok(n > 0)
    }

    /// Inbox blobs newer than `since_id` as `(id, blob)` — the same
    /// contract the HTTP relay's poll returns, so the engine's cursor loop
    /// is unchanged.
    pub fn nostr_inbox_since(&self, since_id: i64) -> Result<Vec<(i64, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, blob FROM nostr_inbox WHERE id > ?1 ORDER BY id")?;
        let rows = stmt.query_map(params![since_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Upsert a discovered offer event (addressable: keep latest per d_tag).
    pub fn nostr_offer_cache_upsert(
        &self,
        event_id: &str,
        d_tag: &str,
        envelope: &str,
        created: u64,
        expires: u64,
    ) -> Result<()> {
        // Addressable semantics: keep ONLY the freshest event per d_tag. A relay
        // can serve a STALE copy of an addressable event, so events for one d_tag
        // arrive out of order across the pool. The old `created < ?` delete left a
        // DOUBLED listing whenever the newer event was applied before the older
        // one (delete matched nothing, then the stale row was inserted alongside).
        // Instead: ignore an event older than what we already hold, otherwise
        // replace EVERY row for the d_tag — so a listing can never double.
        let newest: Option<i64> = self.conn.query_row(
            "SELECT MAX(created) FROM nostr_offer_cache WHERE d_tag = ?1",
            params![d_tag],
            |r| r.get::<_, Option<i64>>(0),
        )?;
        if matches!(newest, Some(c) if (created as i64) < c) {
            return Ok(()); // a fresher event for this d_tag is already cached
        }
        self.conn.execute(
            "DELETE FROM nostr_offer_cache WHERE d_tag = ?1",
            params![d_tag],
        )?;
        self.conn.execute(
            "INSERT OR REPLACE INTO nostr_offer_cache (event_id, d_tag, envelope, created, expires)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![event_id, d_tag, envelope, created as i64, expires as i64],
        )?;
        Ok(())
    }

    /// Drop a cached offer by d_tag (swap_id) — used on revoke/deletion.
    pub fn nostr_offer_cache_remove(&self, d_tag: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM nostr_offer_cache WHERE d_tag = ?1",
            params![d_tag],
        )?;
        Ok(())
    }

    /// Active (non-expired) cached offer envelope JSONs. `now` in unix secs.
    pub fn nostr_offer_cache_active(&self, now: u64) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare(
                // One row per d_tag (the freshest event), so any rows left doubled
                // by an older build still render as a single listing.
                "SELECT envelope FROM nostr_offer_cache c
                 WHERE (c.expires = 0 OR c.expires > ?1)
                   AND c.created = (SELECT MAX(created) FROM nostr_offer_cache WHERE d_tag = c.d_tag)",
            )?;
        let rows = stmt.query_map(params![now as i64], |row| row.get::<_, String>(0))?;
        rows.map(|r| Ok(r?)).collect()
    }

    // ---- maker's own offers (offer-lifecycle registry) ----

    /// Record (or refresh) an offer we just posted, in `live` state.
    pub fn my_offer_put(
        &self,
        offer_id: &str,
        envelope: &str,
        created: u64,
        valid_for: u64,
        now: u64,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO my_offers (offer_id, envelope, created, valid_for, last_refresh, state)
             VALUES (?1, ?2, ?3, ?4, ?5, 'live')
             ON CONFLICT(offer_id) DO UPDATE SET
                 envelope = ?2, created = ?3, valid_for = ?4, last_refresh = ?5, state = 'live'",
            params![
                offer_id,
                envelope,
                created as i64,
                valid_for as i64,
                now as i64
            ],
        )?;
        Ok(())
    }

    fn row_to_my_offer(row: &rusqlite::Row) -> rusqlite::Result<MyOffer> {
        Ok(MyOffer {
            offer_id: row.get(0)?,
            envelope: row.get(1)?,
            created: row.get::<_, i64>(2)?.max(0) as u64,
            valid_for: row.get::<_, i64>(3)?.max(0) as u64,
            last_refresh: row.get::<_, i64>(4)?.max(0) as u64,
            state: row.get(5)?,
        })
    }

    /// Offers still in `live` state — for the refresh loop and revoke-on-close.
    pub fn my_offers_live(&self) -> Result<Vec<MyOffer>> {
        let mut stmt = self.conn.prepare(
            "SELECT offer_id, envelope, created, valid_for, last_refresh, state
             FROM my_offers WHERE state = 'live'",
        )?;
        let rows = stmt.query_map([], Self::row_to_my_offer)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Every registered offer (any state) — for the My-offers view.
    pub fn my_offers_all(&self) -> Result<Vec<MyOffer>> {
        let mut stmt = self.conn.prepare(
            "SELECT offer_id, envelope, created, valid_for, last_refresh, state
             FROM my_offers ORDER BY created DESC",
        )?;
        let rows = stmt.query_map([], Self::row_to_my_offer)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Advance an offer's lifecycle state (`taken` | `revoked` | `expired`).
    pub fn my_offer_set_state(&self, offer_id: &str, state: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE my_offers SET state = ?2 WHERE offer_id = ?1",
            params![offer_id, state],
        )?;
        Ok(())
    }

    /// Mark `revoked` only if still `live` — so the C5 auto-revoke that fires when
    /// a take commits doesn't clobber the `taken` state set at commit time. Returns
    /// how many rows changed (1 if this actually withdrew a live offer of ours, 0
    /// if it was already terminal or not ours) so callers can log real revocations.
    pub fn my_offer_mark_revoked(&self, offer_id: &str) -> Result<usize> {
        let n = self.conn.execute(
            "UPDATE my_offers SET state = 'revoked' WHERE offer_id = ?1 AND state = 'live'",
            params![offer_id],
        )?;
        Ok(n)
    }

    /// Stamp the last successful re-publish of a live offer.
    pub fn my_offer_touch_refresh(&self, offer_id: &str, now: u64) -> Result<()> {
        self.conn.execute(
            "UPDATE my_offers SET last_refresh = ?2 WHERE offer_id = ?1",
            params![offer_id, now as i64],
        )?;
        Ok(())
    }

    /// Remember an offer we've taken, until the maker's init arrives.
    /// `created_at` (unix secs, `engine::local_now`) stamps when we took it,
    /// so the scheduler can prune handshakes the maker never answered (C8).
    /// Re-taking the same offer refreshes the timestamp, restarting its clock.
    pub fn put_pending_take(
        &self,
        offer_id: &str,
        offer_json: &str,
        created_at: u64,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO pending_takes (offer_id, offer, created_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(offer_id) DO UPDATE SET offer = ?2, created_at = ?3",
            params![offer_id, offer_json, created_at as i64],
        )?;
        Ok(())
    }

    pub fn pending_takes(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT offer_id, offer FROM pending_takes")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Pending takes with their `created_at` stamp — for time-based pruning.
    pub fn pending_takes_with_age(&self) -> Result<Vec<(String, String, u64)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT offer_id, offer, created_at FROM pending_takes")?;
        let rows = stmt.query_map([], |row| {
            let created_at: i64 = row.get(2)?;
            Ok((row.get(0)?, row.get(1)?, created_at.max(0) as u64))
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    pub fn remove_pending_take(&self, offer_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM pending_takes WHERE offer_id = ?1",
            params![offer_id],
        )?;
        Ok(())
    }

    // ---- v2 adaptor swap records ----

    pub fn put_adaptor(&self, record: &AdaptorSwapRecord) -> Result<()> {
        self.conn.execute(
            "INSERT INTO adaptor_swaps (swap_id, record) VALUES (?1, ?2)
             ON CONFLICT(swap_id) DO UPDATE SET record = ?2",
            params![record.swap_id, serde_json::to_string(record)?],
        )?;
        Ok(())
    }

    pub fn get_adaptor(&self, swap_id: &str) -> Result<AdaptorSwapRecord> {
        let json: String = self
            .conn
            .query_row(
                "SELECT record FROM adaptor_swaps WHERE swap_id = ?1",
                params![swap_id],
                |row| row.get(0),
            )
            .with_context(|| format!("unknown adaptor swap {swap_id}"))?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn list_adaptor(&self) -> Result<Vec<AdaptorSwapRecord>> {
        let mut stmt = self
            .conn
            .prepare("SELECT record FROM adaptor_swaps ORDER BY swap_id")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.map(|json| Ok(serde_json::from_str(&json?)?)).collect()
    }

    // ---- v2 MuSig2 use-once nonce sessions (spec v2 §3.2) ----

    /// Load the persisted nonce session for `(swap_id, leg)`, if any. The
    /// engine calls this on resume: a present row means the secret nonce was
    /// already generated and MUST be reused as-is (not regenerated), and a
    /// `Consumed` row carries the partial signature to re-send rather than
    /// re-sign.
    pub fn nonce_session(&self, swap_id: &str, leg: &str) -> Result<Option<NonceSession>> {
        self.conn
            .query_row(
                "SELECT state, secnonce, partial_sig FROM nonce_sessions
                 WHERE swap_id = ?1 AND leg = ?2",
                params![swap_id, leg],
                |row| {
                    let state: String = row.get(0)?;
                    let secnonce: Vec<u8> = row.get(1)?;
                    let partial_sig: Option<Vec<u8>> = row.get(2)?;
                    Ok((state, secnonce, partial_sig))
                },
            )
            .map(|(state, secnonce, partial_sig)| {
                Some(NonceSession {
                    state: NonceState::parse(&state),
                    secnonce,
                    partial_sig,
                })
            })
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other.into()),
            })
    }

    /// Commit a freshly generated secret nonce, write-ahead, before its public
    /// nonce is released (spec v2 §3.2). Refuses to overwrite an existing
    /// session — reusing a `(swap_id, leg)` slot with a new nonce is exactly
    /// the reuse that leaks the key, so callers MUST [`Self::nonce_session`]
    /// first and reuse any persisted nonce.
    pub fn nonce_commit(&self, swap_id: &str, leg: &str, secnonce: &[u8]) -> Result<()> {
        let changed = self.conn.execute(
            "INSERT OR IGNORE INTO nonce_sessions (swap_id, leg, state, secnonce)
             VALUES (?1, ?2, 'committed', ?3)",
            params![swap_id, leg, secnonce],
        )?;
        if changed == 0 {
            bail!(
                "nonce session {swap_id}/{leg} already exists — refusing to overwrite (reuse risk)"
            );
        }
        Ok(())
    }

    /// Advance `committed → revealed` (the public nonce has been sent). Forward
    /// only; a no-op if already `revealed`, an error if not yet committed or
    /// already consumed.
    pub fn nonce_reveal(&self, swap_id: &str, leg: &str) -> Result<()> {
        self.nonce_advance(swap_id, leg, NonceState::Committed, NonceState::Revealed)
    }

    /// Advance `revealed → consumed`, recording the produced partial signature
    /// so a later request re-sends it rather than signing again. Forward only.
    ///
    /// One-signature-per-nonce is enforced HERE, at the store — not merely via the
    /// engine's call ordering (spec v2 §3.2: a secret nonce used for two different
    /// messages leaks the MuSig2 signing key). A second consume on an already-
    /// consumed slot is accepted ONLY if it carries the byte-identical partial (an
    /// idempotent re-send after a restart); a *differing* partial is refused, so
    /// no future caller can ever coax two signatures out of one nonce.
    pub fn nonce_consume(&self, swap_id: &str, leg: &str, partial_sig: &[u8]) -> Result<()> {
        let existing: Option<Vec<u8>> = match self.conn.query_row(
            "SELECT partial_sig FROM nonce_sessions
             WHERE swap_id = ?1 AND leg = ?2 AND state = 'consumed'",
            params![swap_id, leg],
            |r| r.get::<_, Option<Vec<u8>>>(0),
        ) {
            Ok(v) => v,
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(e.into()),
        };
        if let Some(prev) = existing {
            anyhow::ensure!(
                prev == partial_sig,
                "nonce session {swap_id}/{leg} already consumed with a DIFFERENT partial \
                 signature — refusing (one-signature-per-nonce; reuse would leak the \
                 MuSig2 signing key, spec v2 §3.2)"
            );
            return Ok(()); // idempotent re-send of the identical partial
        }
        let updated = self.conn.execute(
            "UPDATE nonce_sessions SET state = 'consumed', partial_sig = ?3
             WHERE swap_id = ?1 AND leg = ?2 AND state = 'revealed'",
            params![swap_id, leg, partial_sig],
        )?;
        if updated == 0 {
            bail!("nonce session {swap_id}/{leg} not in a consumable state");
        }
        Ok(())
    }

    fn nonce_advance(
        &self,
        swap_id: &str,
        leg: &str,
        from: NonceState,
        to: NonceState,
    ) -> Result<()> {
        let updated = self.conn.execute(
            "UPDATE nonce_sessions SET state = ?4
             WHERE swap_id = ?1 AND leg = ?2 AND state IN (?3, ?4)",
            params![swap_id, leg, from.as_str(), to.as_str()],
        )?;
        if updated == 0 {
            bail!(
                "nonce session {swap_id}/{leg} not in state {} (cannot advance to {})",
                from.as_str(),
                to.as_str()
            );
        }
        Ok(())
    }
}

/// State of a MuSig2 nonce session (spec v2 §3.2). Advances monotonically;
/// `none` is represented by the row's absence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NonceState {
    Committed,
    Revealed,
    Consumed,
}

impl NonceState {
    fn as_str(self) -> &'static str {
        match self {
            NonceState::Committed => "committed",
            NonceState::Revealed => "revealed",
            NonceState::Consumed => "consumed",
        }
    }
    fn parse(s: &str) -> Self {
        match s {
            "revealed" => NonceState::Revealed,
            "consumed" => NonceState::Consumed,
            _ => NonceState::Committed,
        }
    }
}

/// A persisted nonce session loaded for resume.
#[derive(Debug, Clone)]
pub struct NonceSession {
    pub state: NonceState,
    pub secnonce: Vec<u8>,
    pub partial_sig: Option<Vec<u8>>,
}

/// Whether an on-disk seed file is *encrypted* — useless without an external
/// secret (a passphrase, or the OS-keystore key). Obfuscation-wrapped and
/// plaintext seeds are NOT encrypted. Shared with the merchant registry so the
/// format magics live in one place (#120).
pub fn is_encrypted_seed_file(contents: &str) -> bool {
    let line = contents.trim_start();
    line.starts_with(SEED_MAGIC) || line.starts_with(SEED_V2_KEYRING)
}

/// Whether the seed is passphrase-encrypted (`PACTSEEDv1`) — the only wrap that
/// needs an `unlock` before it can be read.
fn is_passphrase_seed_file(contents: &str) -> bool {
    contents.trim_start().starts_with(SEED_MAGIC)
}

/// Whether to use the OS keystore. Compiled to `true`-capable only on
/// Windows/macOS (native, unattended-friendly, no C deps); on Linux and others
/// it is a const `false`, so those always take the obfuscation wrap and the
/// `keyring` crate is never linked (see libswap's Cargo target gate). Also off
/// under the crate's own unit tests (so they never touch the developer's real
/// keychain) and when `PACT_DISABLE_KEYRING` is set (e2e/CI determinism).
#[cfg(any(windows, target_os = "macos"))]
fn keyring_enabled() -> bool {
    !cfg!(test) && std::env::var_os("PACT_DISABLE_KEYRING").is_none()
}
#[cfg(not(any(windows, target_os = "macos")))]
fn keyring_enabled() -> bool {
    false
}

fn random_bytes<const N: usize>() -> [u8; N] {
    use bitcoin::secp256k1::rand::RngCore;
    let mut b = [0u8; N];
    bitcoin::secp256k1::rand::thread_rng().fill_bytes(&mut b);
    b
}

/// Encrypt a mnemonic with a raw 32-byte key into `MAGIC[:key_id]:nonce:ct`.
fn encrypt_v2(magic: &str, key_id: Option<&str>, key: &[u8; 32], mnemonic: &str) -> Result<String> {
    let nonce = random_bytes::<12>();
    let cipher = ChaCha20Poly1305::new(key.into());
    let ct = cipher
        .encrypt((&nonce).into(), mnemonic.as_bytes())
        .map_err(|_| anyhow::anyhow!("seed encryption failed"))?;
    Ok(match key_id {
        Some(id) => format!("{magic}:{id}:{}:{}\n", hex::encode(nonce), hex::encode(ct)),
        None => format!("{magic}:{}:{}\n", hex::encode(nonce), hex::encode(ct)),
    })
}

fn decrypt_v2(nonce_hex: &str, ct_hex: &str, key: &[u8; 32]) -> Result<String> {
    let cipher = ChaCha20Poly1305::new(key.into());
    let nonce = hex::decode(nonce_hex)?;
    let pt = cipher
        .decrypt(nonce.as_slice().into(), hex::decode(ct_hex)?.as_slice())
        .map_err(|_| anyhow::anyhow!("seed decryption failed"))?;
    String::from_utf8(pt).context("decrypted seed is not UTF-8")
}

/// Store a fresh random seed-key in the OS keystore, returning `(key_id, key)`.
#[cfg(any(windows, target_os = "macos"))]
fn keyring_put_new() -> Result<(String, [u8; 32])> {
    let key_id = hex::encode(random_bytes::<8>());
    let key = random_bytes::<32>();
    let entry =
        keyring::Entry::new(KEYRING_SERVICE, &key_id).context("opening OS keystore entry")?;
    entry
        .set_password(&hex::encode(key))
        .context("writing seed key to OS keystore")?;
    Ok((key_id, key))
}
#[cfg(not(any(windows, target_os = "macos")))]
fn keyring_put_new() -> Result<(String, [u8; 32])> {
    anyhow::bail!("no OS keystore on this platform")
}

/// Fetch a seed-key from the OS keystore by `key_id`. Errors (→ reconfirm) when
/// the entry is missing (new machine / reset keychain), or on a platform with no
/// keystore (a keyring seed copied from Windows/macOS to Linux).
#[cfg(any(windows, target_os = "macos"))]
fn keyring_get(key_id: &str) -> Result<[u8; 32]> {
    let entry =
        keyring::Entry::new(KEYRING_SERVICE, key_id).context("opening OS keystore entry")?;
    let hexkey = entry
        .get_password()
        .context("reading seed key from OS keystore")?;
    let bytes = hex::decode(hexkey.trim()).context("bad keystore key encoding")?;
    <[u8; 32]>::try_from(bytes.as_slice())
        .map_err(|_| anyhow::anyhow!("keystore key has wrong length"))
}
#[cfg(not(any(windows, target_os = "macos")))]
fn keyring_get(_key_id: &str) -> Result<[u8; 32]> {
    anyhow::bail!("this build has no OS keystore — re-import your recovery phrase to continue")
}

/// The *unattended* at-rest wrap (#120) for a seed created/migrated without a
/// passphrase: OS-keystore key when available, else the obfuscation key (with a
/// warning). Never fails — the obfuscation path always succeeds.
fn wrap_unattended(mnemonic: &str) -> Result<String> {
    if keyring_enabled() {
        match keyring_put_new() {
            Ok((key_id, key)) => return encrypt_v2(SEED_V2_KEYRING, Some(&key_id), &key, mnemonic),
            Err(e) => eprintln!(
                "warning: no OS keystore available ({e:#}); storing the seed with obfuscation \
                 only — treat it as UNENCRYPTED. Set a passphrase for real at-rest encryption."
            ),
        }
    }
    encrypt_v2(SEED_V2_OBFS, None, &OBFUSCATION_KEY, mnemonic)
}

/// Atomically write seed-file `contents` (temp file + fsync + rename): a plain
/// truncating write can leave a corrupt/partial seed on a crash and there is no
/// backup copy, so the file is only ever observed fully written or not at all.
fn write_seed_atomic(seed_path: &Path, contents: &str) -> Result<()> {
    let tmp_path = seed_path.with_extension("seed.tmp");
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp_path)
            .with_context(|| format!("creating {}", tmp_path.display()))?;
        f.write_all(contents.as_bytes())?;
        f.sync_all()?; // flush to disk before the rename
    }
    std::fs::rename(&tmp_path, seed_path)
        .with_context(|| format!("installing seed at {}", seed_path.display()))
}

fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; 32]> {
    let mut key = [0u8; 32];
    let params = scrypt::Params::new(SCRYPT_LOG_N, 8, 1, 32)
        .map_err(|e| anyhow::anyhow!("scrypt params: {e}"))?;
    scrypt::scrypt(passphrase.as_bytes(), salt, &params, &mut key)
        .map_err(|e| anyhow::anyhow!("scrypt key derivation: {e}"))?;
    Ok(key)
}

fn encrypt_seed(mnemonic: &str, passphrase: &str) -> Result<String> {
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 12];
    use bitcoin::secp256k1::rand::RngCore;
    let mut rng = bitcoin::secp256k1::rand::thread_rng();
    rng.fill_bytes(&mut salt);
    rng.fill_bytes(&mut nonce);
    let key = derive_key(passphrase, &salt)?;
    let cipher = ChaCha20Poly1305::new((&key).into());
    let ciphertext = cipher
        .encrypt((&nonce).into(), mnemonic.as_bytes())
        .map_err(|_| anyhow::anyhow!("seed encryption failed"))?;
    Ok(format!(
        "{SEED_MAGIC}:{}:{}:{}\n",
        hex::encode(salt),
        hex::encode(nonce),
        hex::encode(ciphertext)
    ))
}

fn decrypt_seed(line: &str, passphrase: &str) -> Result<String> {
    let mut parts = line.split(':');
    let (magic, salt, nonce, ciphertext) = (
        parts.next().unwrap_or_default(),
        parts.next().context("malformed seed file")?,
        parts.next().context("malformed seed file")?,
        parts.next().context("malformed seed file")?,
    );
    if magic != SEED_MAGIC {
        bail!("unknown seed file format {magic:?}");
    }
    let key = derive_key(passphrase, &hex::decode(salt)?)?;
    let cipher = ChaCha20Poly1305::new((&key).into());
    let nonce = hex::decode(nonce)?;
    let plaintext = cipher
        .decrypt(nonce.as_slice().into(), hex::decode(ciphertext)?.as_slice())
        .map_err(|_| anyhow::anyhow!("seed decryption failed — wrong passphrase?"))?;
    String::from_utf8(plaintext).context("decrypted seed is not UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::Network;

    fn record(id: &str) -> SwapRecord {
        SwapRecord {
            swap_id: id.into(),
            role: Role::Initiator,
            state: State::Created,
            created_at: 1_700_000_123,
            swap_index: Some(0),
            chain_a: ChainRef {
                coin_id: "btcx".into(),
                network: Network::Regtest,
            },
            chain_b: ChainRef {
                coin_id: "btc".into(),
                network: Network::Regtest,
            },
            amount_a: 1,
            amount_b: 1,
            hash_h: "00".repeat(32),
            t1: 1_700_000_001,
            t2: 1_700_000_000,
            n_a: 1,
            n_b: 1,
            alice_refund_pubkey_a: String::new(),
            alice_redeem_pubkey_b: String::new(),
            bob_redeem_pubkey_a: None,
            bob_refund_pubkey_b: None,
            counterparty_identity: None,
            htlc_a_txid: None,
            htlc_a_vout: None,
            htlc_b_txid: None,
            htlc_b_vout: None,
            htlc_b_height: None,
            preimage: None,
            refund_tx_hex: None,
            final_txid: None,
            final_tx_hex: None,
            last_action_height: 0,
        }
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("libswap-store-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn offer_cache_keeps_one_row_per_dtag() {
        let dir = temp_dir("offer-cache-dedup");
        let store = Store::init(&dir, None).unwrap();
        let far = 9_999_999_999; // far-future expiry: stays active for the test

        // A newer event arrives FIRST, then a STALE older copy for the same d_tag
        // (relays can serve addressable events out of order). The stale one is
        // ignored — the listing must not double.
        store
            .nostr_offer_cache_upsert("evNew", "off1", "{\"v\":\"new\"}", 200, far)
            .unwrap();
        store
            .nostr_offer_cache_upsert("evOld", "off1", "{\"v\":\"old\"}", 100, far)
            .unwrap();
        let active = store.nostr_offer_cache_active(0).unwrap();
        assert_eq!(active.len(), 1, "one listing per d_tag");
        assert!(active[0].contains("new"), "the freshest event wins");

        // A genuinely newer event replaces the row in place (still one).
        store
            .nostr_offer_cache_upsert("evNewer", "off1", "{\"v\":\"newer\"}", 300, far)
            .unwrap();
        let active = store.nostr_offer_cache_active(0).unwrap();
        assert_eq!(active.len(), 1);
        assert!(active[0].contains("newer"));

        // A different d_tag coexists; remove drops only its own listing.
        store
            .nostr_offer_cache_upsert("evB", "off2", "{\"v\":\"b\"}", 150, far)
            .unwrap();
        assert_eq!(store.nostr_offer_cache_active(0).unwrap().len(), 2);
        store.nostr_offer_cache_remove("off1").unwrap();
        assert_eq!(store.nostr_offer_cache_active(0).unwrap().len(), 1);
    }

    #[test]
    fn my_offers_registry_lifecycle() {
        let dir = temp_dir("my-offers");
        let store = Store::init(&dir, None).unwrap();

        store
            .my_offer_put("aa", "{\"e\":1}", 1_700_000_000, 1800, 1_700_000_000)
            .unwrap();
        store
            .my_offer_put("bb", "{\"e\":2}", 1_700_000_000, 1800, 1_700_000_000)
            .unwrap();
        assert_eq!(store.my_offers_live().unwrap().len(), 2);
        assert_eq!(store.my_offers_all().unwrap().len(), 2);

        // Refresh stamps last_refresh.
        store.my_offer_touch_refresh("aa", 1_700_000_600).unwrap();
        let aa = store
            .my_offers_all()
            .unwrap()
            .into_iter()
            .find(|o| o.offer_id == "aa")
            .unwrap();
        assert_eq!(aa.last_refresh, 1_700_000_600);
        assert_eq!(aa.valid_for, 1800);
        assert_eq!(aa.state, "live");

        // `taken` is terminal: the auto-revoke (mark_revoked) must not clobber it.
        store.my_offer_set_state("aa", "taken").unwrap();
        store.my_offer_mark_revoked("aa").unwrap(); // no-op: not live
        let aa = store
            .my_offers_all()
            .unwrap()
            .into_iter()
            .find(|o| o.offer_id == "aa")
            .unwrap();
        assert_eq!(aa.state, "taken");

        // A still-live offer revokes to `revoked`, and leaves the live set.
        store.my_offer_mark_revoked("bb").unwrap();
        let bb = store
            .my_offers_all()
            .unwrap()
            .into_iter()
            .find(|o| o.offer_id == "bb")
            .unwrap();
        assert_eq!(bb.state, "revoked");
        assert_eq!(store.my_offers_live().unwrap().len(), 0);
    }

    #[test]
    fn init_open_roundtrip_and_index_allocation() {
        let dir = temp_dir("plain");
        let store = Store::init(&dir, None).unwrap();
        assert!(
            Store::init(&dir, None).is_err(),
            "must not overwrite a seed"
        );
        store.seed().unwrap();
        assert!(!store.seed_is_encrypted().unwrap());

        assert_eq!(store.next_swap_index().unwrap(), 0);
        assert_eq!(store.next_swap_index().unwrap(), 1);

        let mut rec = record("aabb");
        store.put(&rec).unwrap();
        rec.state = State::Accepted;
        store.put(&rec).unwrap();
        let loaded = store.get("aabb").unwrap();
        assert_eq!(loaded.state, State::Accepted);
        assert_eq!(store.list().unwrap().len(), 1);
        assert!(store.get("nope").is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn encrypted_seed_roundtrip() {
        let dir = temp_dir("enc");
        let store = Store::init(&dir, Some("correct horse")).unwrap();
        assert!(store.seed_is_encrypted().unwrap());
        let identity = store.seed().unwrap().identity_pubkey().unwrap();

        // Reopen with the right passphrase: same seed.
        let reopened = Store::open(&dir, Some("correct horse")).unwrap();
        assert_eq!(
            reopened.seed().unwrap().identity_pubkey().unwrap(),
            identity
        );

        // Wrong or missing passphrase must fail, not yield a different seed.
        assert!(Store::open(&dir, Some("wrong")).unwrap().seed().is_err());
        assert!(Store::open(&dir, None).unwrap().seed().is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unattended_seed_is_never_plaintext_and_roundtrips() {
        // #120: a no-passphrase seed is wrapped, not written as ASCII. Under the
        // crate's own tests the keyring is disabled, so it takes the obfuscation
        // wrap — which is "not plaintext" but is treated as UNENCRYPTED.
        let dir = temp_dir("unattended");
        let store = Store::init(&dir, None).unwrap();
        let on_disk = std::fs::read_to_string(dir.join(SEED_FILE)).unwrap();
        assert!(
            on_disk.starts_with(SEED_V2_OBFS),
            "no-passphrase seed must be wrapped, got: {on_disk}"
        );
        assert!(
            !is_encrypted_seed_file(&on_disk),
            "obfs counts as unencrypted"
        );
        let status = store.wallet_status().unwrap();
        assert!(status.seed_exists && !status.encrypted && !status.locked);
        // Readable now and after reopen (auto-unlock, no passphrase).
        let id = store.seed().unwrap().identity_pubkey().unwrap();
        let reopened = Store::open(&dir, None).unwrap();
        assert_eq!(reopened.seed().unwrap().identity_pubkey().unwrap(), id);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn migrates_legacy_plaintext_seed_on_open() {
        // A pre-#120 seed sits on disk as the raw mnemonic. Opening the store
        // must re-wrap it in place (never-plaintext), preserving the same seed.
        let dir = temp_dir("migrate");
        std::fs::create_dir_all(&dir).unwrap();
        let phrase = "abandon abandon abandon abandon abandon abandon \
                      abandon abandon abandon abandon abandon about";
        std::fs::write(dir.join(SEED_FILE), format!("{phrase}\n")).unwrap();
        assert!(!is_encrypted_seed_file(phrase), "precondition: plaintext");

        let store = Store::open(&dir, None).unwrap();
        let migrated = std::fs::read_to_string(dir.join(SEED_FILE)).unwrap();
        assert!(
            migrated.starts_with(SEED_V2_OBFS),
            "plaintext seed re-wrapped on open, got: {migrated}"
        );
        // Same seed, still readable.
        let want = PactSeed::from_mnemonic(phrase, "")
            .unwrap()
            .identity_pubkey()
            .unwrap();
        assert_eq!(store.seed().unwrap().identity_pubkey().unwrap(), want);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn create_seed_roundtrip_unencrypted() {
        let dir = temp_dir("create-plain");
        let mut store = Store::open(&dir, None).unwrap();
        assert!(!store.wallet_status().unwrap().seed_exists);

        let mnemonic = store.create_seed(None, 12).unwrap();
        assert_eq!(mnemonic.split_whitespace().count(), 12);
        let status = store.wallet_status().unwrap();
        assert!(status.seed_exists && !status.encrypted && !status.locked);
        // The seed is usable immediately and matches the returned mnemonic.
        let identity = store.seed().unwrap().identity_pubkey().unwrap();
        let from_mnemonic = PactSeed::from_mnemonic(&mnemonic, "")
            .unwrap()
            .identity_pubkey()
            .unwrap();
        assert_eq!(identity, from_mnemonic);

        // Never overwrite an existing seed.
        assert!(store.create_seed(None, 12).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn create_seed_roundtrip_encrypted_and_unlock() {
        let dir = temp_dir("create-enc");
        let mut store = Store::open(&dir, None).unwrap();
        store.create_seed(Some("hunter2"), 12).unwrap();
        let status = store.wallet_status().unwrap();
        assert!(
            status.encrypted && !status.locked,
            "creator holds the passphrase: {status:?}"
        );
        let identity = store.seed().unwrap().identity_pubkey().unwrap();

        // A fresh open with no passphrase is locked; seed() refuses.
        let mut reopened = Store::open(&dir, None).unwrap();
        let st = reopened.wallet_status().unwrap();
        assert!(st.encrypted && st.locked, "reopen must be locked: {st:?}");
        assert!(
            reopened.seed().is_err(),
            "locked store must not yield a seed"
        );

        // Wrong passphrase fails and leaves it locked.
        assert!(reopened.unlock("wrong").is_err());
        assert!(reopened.wallet_status().unwrap().locked);

        // Right passphrase unlocks; same identity as the creator saw.
        reopened.unlock("hunter2").unwrap();
        assert!(!reopened.wallet_status().unwrap().locked);
        assert_eq!(
            reopened.seed().unwrap().identity_pubkey().unwrap(),
            identity
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn import_seed_roundtrip_and_validation() {
        const PHRASE: &str =
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let dir = temp_dir("import");
        let mut store = Store::open(&dir, None).unwrap();
        assert!(store.import_seed("not a real mnemonic", None).is_err());
        let returned = store.import_seed(PHRASE, None).unwrap();
        assert_eq!(returned, PHRASE);
        let expected = PactSeed::from_mnemonic(PHRASE, "")
            .unwrap()
            .identity_pubkey()
            .unwrap();
        assert_eq!(store.seed().unwrap().identity_pubkey().unwrap(), expected);

        // Importing the same phrase encrypted into a *second* merchant dir
        // yields the same identity — the data dir is the only difference.
        let dir2 = temp_dir("import2");
        let mut store2 = Store::open(&dir2, None).unwrap();
        store2.import_seed(PHRASE, Some("pw")).unwrap();
        assert!(store2.wallet_status().unwrap().encrypted);
        assert_eq!(store2.seed().unwrap().identity_pubkey().unwrap(), expected);
        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(&dir2).ok();
    }

    #[test]
    fn two_merchants_have_distinct_identities() {
        // A "merchant" is one seed = one data dir; switching merchants is just
        // pointing pactd at another dir. Two created merchants are unlinkable.
        let dir_a = temp_dir("merchant-a");
        let dir_b = temp_dir("merchant-b");
        let id_a = {
            let mut s = Store::open(&dir_a, None).unwrap();
            s.create_seed(None, 12).unwrap();
            s.seed().unwrap().identity_pubkey().unwrap()
        };
        let id_b = {
            let mut s = Store::open(&dir_b, Some("pw")).unwrap();
            s.create_seed(Some("pw"), 12).unwrap();
            s.seed().unwrap().identity_pubkey().unwrap()
        };
        assert_ne!(
            id_a, id_b,
            "independent seeds must be unlinkable identities"
        );

        // Reopening merchant A still yields A's identity (state is the dir).
        let reopened = Store::open(&dir_a, None).unwrap();
        assert_eq!(reopened.seed().unwrap().identity_pubkey().unwrap(), id_a);
        std::fs::remove_dir_all(&dir_a).ok();
        std::fs::remove_dir_all(&dir_b).ok();
    }

    #[test]
    fn old_records_without_refund_tx_field_still_parse() {
        let mut value = serde_json::to_value(record("cc")).unwrap();
        value.as_object_mut().unwrap().remove("refund_tx_hex");
        let parsed: SwapRecord = serde_json::from_value(value).unwrap();
        assert!(parsed.refund_tx_hex.is_none());
    }

    #[test]
    fn created_at_roundtrips_through_store() {
        let dir = temp_dir("created-at");
        let store = Store::init(&dir, None).unwrap();
        let rec = record("dd"); // created_at = 1_700_000_123
        store.put(&rec).unwrap();
        assert_eq!(store.get("dd").unwrap().created_at, 1_700_000_123);
        assert_eq!(store.list().unwrap()[0].created_at, 1_700_000_123);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn record_fields_are_required_no_silent_default() {
        // No backward compat: a blob missing a field (e.g. a record written
        // before created_at existed) no longer silently defaults — it fails to
        // load rather than masking a malformed record.
        let mut value = serde_json::to_value(record("ee")).unwrap();
        value.as_object_mut().unwrap().remove("created_at");
        assert!(serde_json::from_value::<SwapRecord>(value).is_err());
    }

    #[test]
    fn pending_take_stamps_and_returns_created_at() {
        // C8: the take timestamp is persisted and read back for pruning.
        let dir = temp_dir("pending-take-age");
        let store = Store::init(&dir, None).unwrap();
        store
            .put_pending_take("offer-1", "{}", 1_700_000_500)
            .unwrap();
        store
            .put_pending_take("offer-2", "{}", 1_700_000_600)
            .unwrap();

        let mut aged = store.pending_takes_with_age().unwrap();
        aged.sort_by_key(|(id, _, _)| id.clone());
        assert_eq!(aged.len(), 2);
        assert_eq!(aged[0], ("offer-1".into(), "{}".into(), 1_700_000_500));
        assert_eq!(aged[1].2, 1_700_000_600);

        // Re-taking refreshes the timestamp (ON CONFLICT updates created_at).
        store
            .put_pending_take("offer-1", "{}", 1_700_009_999)
            .unwrap();
        let refreshed = store
            .pending_takes_with_age()
            .unwrap()
            .into_iter()
            .find(|(id, _, _)| id == "offer-1")
            .unwrap();
        assert_eq!(refreshed.2, 1_700_009_999);

        store.remove_pending_take("offer-1").unwrap();
        assert_eq!(store.pending_takes().unwrap().len(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn nonce_session_lifecycle_commit_reveal_consume() {
        let dir = temp_dir("nonce-life");
        let store = Store::init(&dir, None).unwrap();
        assert!(store.nonce_session("swap1", "redeem_b").unwrap().is_none());

        store
            .nonce_commit("swap1", "redeem_b", &[0xaa; 132])
            .unwrap();
        let s = store.nonce_session("swap1", "redeem_b").unwrap().unwrap();
        assert_eq!(s.state, NonceState::Committed);
        assert_eq!(s.secnonce, vec![0xaa; 132]);
        assert!(s.partial_sig.is_none());

        store.nonce_reveal("swap1", "redeem_b").unwrap();
        assert_eq!(
            store
                .nonce_session("swap1", "redeem_b")
                .unwrap()
                .unwrap()
                .state,
            NonceState::Revealed
        );

        store
            .nonce_consume("swap1", "redeem_b", &[0x55; 32])
            .unwrap();
        let s = store.nonce_session("swap1", "redeem_b").unwrap().unwrap();
        assert_eq!(s.state, NonceState::Consumed);
        assert_eq!(s.partial_sig, Some(vec![0x55; 32]));
        // reveal/consume are idempotent (forward-only allows staying).
        store
            .nonce_consume("swap1", "redeem_b", &[0x55; 32])
            .unwrap();
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn nonce_commit_refuses_overwrite_reuse() {
        // The whole point of the use-once design: a second commit on the same
        // (swap, leg) with a different nonce must be rejected, not silently
        // overwrite — that overwrite is the key-leaking reuse.
        let dir = temp_dir("nonce-reuse");
        let store = Store::init(&dir, None).unwrap();
        store.nonce_commit("s", "redeem_a", &[0x01; 132]).unwrap();
        assert!(store.nonce_commit("s", "redeem_a", &[0x02; 132]).is_err());
        // Original nonce is untouched.
        assert_eq!(
            store
                .nonce_session("s", "redeem_a")
                .unwrap()
                .unwrap()
                .secnonce,
            vec![0x01; 132]
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn nonce_consume_rejects_differing_partial() {
        // One-signature-per-nonce (spec v2 §3.2): re-consuming a slot with the
        // SAME partial is an idempotent no-op (restart re-send), but a DIFFERENT
        // partial must be refused — two partials under one nonce leak the key.
        let dir = temp_dir("nonce-consume-reuse");
        let store = Store::init(&dir, None).unwrap();
        store.nonce_commit("s", "redeem_a", &[0x09; 132]).unwrap();
        store.nonce_reveal("s", "redeem_a").unwrap();
        store.nonce_consume("s", "redeem_a", &[0xAA; 32]).unwrap();
        // Idempotent re-send of the identical partial: accepted.
        store.nonce_consume("s", "redeem_a", &[0xAA; 32]).unwrap();
        // A different partial under the same consumed nonce: refused.
        assert!(store.nonce_consume("s", "redeem_a", &[0xBB; 32]).is_err());
        // The originally recorded partial is untouched.
        assert_eq!(
            store
                .nonce_session("s", "redeem_a")
                .unwrap()
                .unwrap()
                .partial_sig,
            Some(vec![0xAA; 32])
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn nonce_state_is_forward_only() {
        let dir = temp_dir("nonce-forward");
        let store = Store::init(&dir, None).unwrap();
        // Cannot reveal/consume a session that was never committed.
        assert!(store.nonce_reveal("s", "leg").is_err());
        assert!(store.nonce_consume("s", "leg", &[0u8; 32]).is_err());
        // Cannot consume before revealing.
        store.nonce_commit("s", "leg", &[0x07; 132]).unwrap();
        assert!(store.nonce_consume("s", "leg", &[0u8; 32]).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn nonce_session_survives_restart() {
        // Simulated daemon restart: a committed (but unconsumed) session must
        // reload from disk so the engine reuses the SAME nonce, never a fresh
        // one (spec v2 §3.2 resume rule).
        let dir = temp_dir("nonce-restart");
        {
            let store = Store::init(&dir, None).unwrap();
            store.nonce_commit("s", "redeem_b", &[0x42; 132]).unwrap();
            store.nonce_reveal("s", "redeem_b").unwrap();
        }
        let store = Store::open(&dir, None).unwrap(); // "restart"
        let s = store.nonce_session("s", "redeem_b").unwrap().unwrap();
        assert_eq!(s.state, NonceState::Revealed);
        assert_eq!(s.secnonce, vec![0x42; 132]);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn pending_takes_migration_is_idempotent() {
        // Re-opening a db must not fail on the C8 `ALTER TABLE ADD COLUMN`
        // (the column already exists the second time → duplicate-column error
        // is swallowed). Old rows surface created_at = 0.
        let dir = temp_dir("pending-take-migrate");
        {
            let store = Store::init(&dir, None).unwrap();
            store.put_pending_take("o", "{}", 42).unwrap();
        }
        let store = Store::open(&dir, None).unwrap(); // second open re-runs ALTER
        let aged = store.pending_takes_with_age().unwrap();
        assert_eq!(aged, vec![("o".into(), "{}".into(), 42u64)]);
        std::fs::remove_dir_all(&dir).ok();
    }
}
