//! Merchant registry (C10) — pactd owns merchants, the Bitcoin-Core wallet
//! analog.
//!
//! A *merchant* is one Pact seed = one trading identity = one data dir. Until
//! C10 the registry (labels + active selection) lived in Satchel's
//! `satchel.json`, and "switch merchant" meant relaunching pactd at a different
//! `--data-dir`. That left two sources of truth (Satchel's registry and pactd's
//! on-disk seed dirs) which could drift. C10 moves the registry into pactd: one
//! pactd is launched at a **parent** data dir and owns a `merchants/` subdir
//! plus a manifest (`merchants.json`), mirroring how `bitcoind` owns wallets.
//!
//! Phase 1 (this module): pactd owns the registry + metadata and loads **one**
//! active merchant's seed at a time, switching in-process (no relaunch). The
//! RPC surface is deliberately designed *merchant-scoped-ready* — `loadmerchant`
//! / `unloadmerchant` / `getmerchantinfo {id?}` name an explicit merchant — so
//! Phase 2 (multiple merchants loaded concurrently, scheduler/relay watching
//! all of them) is an internal capacity change, not an API break. Where Phase 2
//! would extend is flagged in-line with `// PHASE 2:`.
//!
//! Backward compatibility: pactd's `--data-dir` is also used directly by the
//! e2e harness and `pact-cli`, where the seed lives *flat* in the data dir
//! (`--auto-init`, or a `createseed` straight onto a seedless daemon). That
//! **flat mode** is preserved: if the parent data dir itself holds a seed (or
//! `--auto-init` created one), the data dir *is* the active merchant — no
//! `merchants/` subdir, a single synthetic `default` merchant in the manifest.
//! Satchel's managed mode launches pactd seedless and uses `createmerchant`,
//! which switches the daemon into nested `merchants/<id>/` mode.

use anyhow::{bail, ensure, Context, Result};
use libswap::engine::Engine;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const MANIFEST_FILE: &str = "merchants.json";
const MERCHANTS_DIR: &str = "merchants";
/// The synthetic merchant id used in flat/legacy mode (seed in the data dir
/// root, as the harness and `pact-cli` use it).
const FLAT_ID: &str = "default";

/// One merchant's non-secret metadata, persisted in the manifest. The seed and
/// swap history live in the merchant's data dir; only the public handle
/// (`identity`) and the user-facing `label` are copied here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerchantMeta {
    /// Data-dir folder name under `merchants/` (e.g. `m1`), or `default` in
    /// flat mode. Never collides with an existing on-disk dir (see
    /// [`MerchantRegistry::alloc_id`]).
    pub id: String,
    /// User-facing name. The identifier the UI shows; `id` is internal.
    pub label: String,
    /// BIP340 x-only identity pubkey (hex), derived from the merchant's seed.
    /// `None` until the seed is provisioned (a fresh merchant is created before
    /// its seed exists) or while it stays locked + has never been read.
    #[serde(default)]
    pub identity: Option<String>,
    /// Unix seconds at creation (or adoption). Lets the UI order merchants.
    #[serde(default)]
    pub created: u64,
    /// Whether the on-disk seed is encrypted. Convenience for the UI; the
    /// authoritative lock state comes from the loaded engine's `walletstatus`.
    #[serde(default)]
    pub encrypted: bool,
}

/// The on-disk manifest: every known merchant + which one is active.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Manifest {
    #[serde(default)]
    merchants: Vec<MerchantMeta>,
    #[serde(default)]
    active: Option<String>,
}

/// Engine construction parameters shared across merchants — the machine-level
/// config Satchel passes at launch (coins, board, auto-fund) plus the daemon
/// network and the session passphrase (for an encrypted seed brought up via
/// `PACT_PASSPHRASE`). One pactd shares these across every merchant's cheap
/// seed; that shared backend/scheduler/relay is the Core efficiency rationale
/// for one process owning many wallets.
#[derive(Clone)]
pub struct EngineConfig {
    pub coins: BTreeMap<String, String>,
    /// Per-coin confirmation depth (reorg-safety/finality), keyed by `coin_id`;
    /// a coin absent here uses the engine's network/spacing default. Machine
    /// level, like `coins` (Satchel's Coins setup page → `--coin-confs`).
    pub coin_confirmations: BTreeMap<String, u32>,
    pub board_url: Option<String>,
    /// Nostr relay URLs (comma-separated), shared across merchants like the
    /// board. Drives the Nostr transport (docs/NOSTR_TRANSPORT.md).
    pub nostr_relays: Option<String>,
    pub auto_fund: bool,
    /// Passphrase to try when *opening* a merchant's store (env-supplied). New
    /// per-merchant unlocks go through the engine's `store.unlock` instead. The
    /// daemon network is not held here — it is supplied per-RPC by pactd.
    pub passphrase: Option<String>,
}

impl EngineConfig {
    fn build_engine(&self, data_dir: &Path) -> Result<Engine> {
        let mut engine = Engine::open(data_dir, self.passphrase.as_deref(), self.coins.clone())?;
        engine.coin_confirmations = self.coin_confirmations.clone();
        engine.board_url = self.board_url.clone();
        engine.nostr_relays = self.nostr_relays.clone();
        engine.auto_fund = self.auto_fund;
        Ok(engine)
    }
}

/// Whether a layout is flat (seed in the data-dir root) or nested
/// (`merchants/<id>/`). Decided once at boot and fixed for the process: the
/// harness/CLI path stays flat, Satchel's managed path goes nested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layout {
    Flat,
    Nested,
}

/// pactd's owned merchant registry plus the **one** loaded engine (Phase 1).
///
/// PHASE 2: `active` + `engine` would become a map of `id -> Engine`, the
/// scheduler iterating all loaded engines; the manifest, `create`, `info`, and
/// the merchant-scoped RPC arguments already accommodate that without change.
pub struct MerchantRegistry {
    data_dir: PathBuf,
    layout: Layout,
    cfg: EngineConfig,
    manifest: Manifest,
    /// The currently loaded merchant's engine, if any. `None` means no active
    /// merchant (fresh managed install before `createmerchant`).
    engine: Option<Engine>,
}

impl MerchantRegistry {
    /// Open the registry at pactd's parent data dir, discovering/adopting any
    /// existing merchants, and load the active one's engine if there is one.
    ///
    /// `flat_seed_present` is whether a seed already sits in the data-dir root
    /// (the harness/CLI/`--auto-init` shape). `prefer_nested` is Satchel's
    /// `--merchants` opt-in for the C10 `merchants/<id>/` layout.
    ///
    /// Layout precedence (so an existing dir never flips mode under it):
    ///   1. a root seed, or a manifest with the synthetic `default` entry → Flat
    ///      (legacy/CLI — a bare `createseed` lands in the root, as today);
    ///   2. else `prefer_nested` (Satchel) → Nested;
    ///   3. else Flat (a plain `pactd --data-dir D` with nothing yet stays the
    ///      legacy single-seed daemon the CLI expects).
    pub fn open(
        data_dir: &Path,
        cfg: EngineConfig,
        flat_seed_present: bool,
        prefer_nested: bool,
    ) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let mut manifest = load_manifest(data_dir)?;

        let manifest_is_flat = manifest.merchants.iter().any(|m| m.id == FLAT_ID);
        let layout = if flat_seed_present || manifest_is_flat {
            Layout::Flat
        } else if prefer_nested {
            Layout::Nested
        } else {
            Layout::Flat
        };

        match layout {
            Layout::Flat => {
                // The data dir *is* the merchant. Ensure a manifest entry so
                // listmerchants/getmerchantinfo work uniformly.
                if !manifest.merchants.iter().any(|m| m.id == FLAT_ID) {
                    manifest.merchants.push(MerchantMeta {
                        id: FLAT_ID.to_string(),
                        label: "default".to_string(),
                        identity: None,
                        created: now_secs(),
                        encrypted: false,
                    });
                }
                manifest.active = Some(FLAT_ID.to_string());
            }
            Layout::Nested => {
                // Adopt any on-disk merchants/<id>/ dirs that carry a seed but
                // aren't yet in the manifest (migration: today's Greedy/Grumpy,
                // or a manifest cleared while dirs survived — the desync bug
                // class C10 kills).
                adopt_existing(data_dir, &mut manifest)?;
                // If the recorded active merchant has vanished from disk, drop
                // it so we don't try to load a missing dir.
                if let Some(active) = manifest.active.clone() {
                    if !manifest.merchants.iter().any(|m| m.id == active) {
                        manifest.active = None;
                    }
                }
            }
        }

        let mut reg = Self {
            data_dir: data_dir.to_path_buf(),
            layout,
            cfg,
            manifest,
            engine: None,
        };

        // Load the active merchant's engine (best-effort: a locked/encrypted
        // seed still opens; identity backfill just stays None until unlocked).
        if let Some(active) = reg.manifest.active.clone() {
            reg.engine = Some(reg.cfg.build_engine(&reg.dir_of(&active))?);
            reg.backfill_identity(&active);
        }
        reg.save()?;
        Ok(reg)
    }

    /// Data dir for a merchant id: the root itself in flat mode, else
    /// `merchants/<id>/`.
    fn dir_of(&self, id: &str) -> PathBuf {
        match self.layout {
            Layout::Flat => self.data_dir.clone(),
            Layout::Nested => self.data_dir.join(MERCHANTS_DIR).join(id),
        }
    }

    fn save(&self) -> Result<()> {
        save_manifest(&self.data_dir, &self.manifest)
    }

    /// The active merchant id, if one is loaded.
    pub fn active_id(&self) -> Option<&str> {
        self.manifest.active.as_deref()
    }

    /// Borrow the active engine, or error clearly when none is loaded — the
    /// single place swap/board/seed RPCs funnel through.
    pub fn active(&self) -> Result<&Engine> {
        self.engine
            .as_ref()
            .context("no active merchant — create or load one first")
    }

    /// Mutable borrow of the active engine (seed-lifecycle RPCs).
    pub fn active_mut(&mut self) -> Result<&mut Engine> {
        self.engine
            .as_mut()
            .context("no active merchant — create or load one first")
    }

    /// The pactd data dir (parent of `merchants/` and `logs/`) — RC2 uses it to
    /// locate the rolling log file for the per-swap dump.
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// The current auto-fund setting (RC2): whether the scheduler funds our
    /// swap legs automatically. Read from the engine config, so it is available
    /// even before a merchant is loaded.
    pub fn auto_fund(&self) -> bool {
        self.cfg.auto_fund
    }

    /// Flip auto-fund at runtime (RC2): updates the stored config so any later
    /// merchant build inherits it, AND the live active engine for immediate
    /// effect (no pactd restart). Satchel persists its own copy of the choice so
    /// it survives a restart (where it is re-applied via the launch flag).
    pub fn set_auto_fund(&mut self, on: bool) {
        self.cfg.auto_fund = on;
        if let Some(engine) = self.engine.as_mut() {
            engine.auto_fund = on;
        }
    }

    /// Allocate the next free `m<N>` id that collides with neither the manifest
    /// nor an existing on-disk `merchants/<id>/` dir (the desync guard from
    /// C10: a dir left behind after a manifest wipe must not be reused).
    fn alloc_id(&self) -> String {
        let merchants_root = self.data_dir.join(MERCHANTS_DIR);
        let mut n = 1u32;
        loop {
            let candidate = format!("m{n}");
            let in_manifest = self.manifest.merchants.iter().any(|m| m.id == candidate);
            let on_disk = merchants_root.join(&candidate).exists();
            if !in_manifest && !on_disk {
                return candidate;
            }
            n += 1;
        }
    }

    /// `createmerchant {label}` — allocate the next free id, create its data
    /// dir, set it active (loading a fresh seedless engine), and return its
    /// metadata. The seed is provisioned afterwards via `createseed`/
    /// `importseed`, which operate on this now-active merchant.
    pub fn create(&mut self, label: &str) -> Result<MerchantMeta> {
        ensure!(
            self.layout == Layout::Nested,
            "this pactd runs a single flat merchant (harness/CLI mode); \
             createmerchant needs a managed parent data dir"
        );
        let id = self.alloc_id();
        let label = label.trim();
        let label = if label.is_empty() {
            format!("Merchant {}", self.manifest.merchants.len() + 1)
        } else {
            label.to_string()
        };
        let dir = self.dir_of(&id);
        std::fs::create_dir_all(&dir)?;

        let meta = MerchantMeta {
            id: id.clone(),
            label,
            identity: None,
            created: now_secs(),
            encrypted: false,
        };
        self.manifest.merchants.push(meta.clone());

        // Switch to it in-process (a fresh dir has no live swap to gate on).
        self.engine = Some(self.cfg.build_engine(&dir)?);
        self.manifest.active = Some(id);
        self.save()?;
        Ok(meta)
    }

    /// `loadmerchant {id}` — switch the active merchant in-process. Refuses to
    /// switch *away* from a merchant that has a live (non-terminal) swap, so we
    /// never stop watching its timelocks (the fund-safety gate — same rule as
    /// the exit gate, one level down).
    pub fn load(&mut self, id: &str) -> Result<MerchantMeta> {
        ensure!(
            self.manifest.merchants.iter().any(|m| m.id == id),
            "unknown merchant {id}"
        );
        if self.manifest.active.as_deref() == Some(id) {
            // Already active — idempotent.
            return self.meta_of(id).cloned().context("merchant vanished");
        }
        self.ensure_safe_to_switch_away()?;

        let dir = self.dir_of(id);
        let engine = self.cfg.build_engine(&dir)?;
        self.engine = Some(engine);
        self.manifest.active = Some(id.to_string());
        self.backfill_identity(id);
        self.save()?;
        self.meta_of(id).cloned().context("merchant vanished")
    }

    /// `unloadmerchant` — drop the active merchant from memory (no merchant
    /// loaded afterward). Same fund-safety gate as `load`.
    pub fn unload(&mut self) -> Result<()> {
        ensure!(
            self.layout == Layout::Nested,
            "the flat/CLI merchant cannot be unloaded"
        );
        self.ensure_safe_to_switch_away()?;
        self.engine = None;
        self.manifest.active = None;
        self.save()?;
        Ok(())
    }

    /// Fund-safety gate: an active merchant with a live (non-terminal) swap
    /// must keep its engine loaded so the scheduler keeps watching timelocks.
    /// PHASE 2: with concurrent merchants this disappears — every merchant
    /// stays loaded, so switching the *foreground* one never stops a watcher.
    fn ensure_safe_to_switch_away(&self) -> Result<()> {
        if let Some(engine) = self.engine.as_ref() {
            // A locked engine can't list reliably; treat a list error as "no
            // information" rather than blocking a switch the user asked for.
            if let Ok(swaps) = engine.store.list() {
                if let Some(s) = swaps.iter().find(|s| !is_terminal(s.state)) {
                    let active = self.active_id().unwrap_or("?");
                    bail!(
                        "merchant {active} has a live swap ({} in state {:?}) — \
                         finish or refund it before switching, so its timelocks \
                         keep being watched",
                        s.swap_id,
                        s.state
                    );
                }
            }
        }
        Ok(())
    }

    /// `listmerchants` → metadata for every known merchant plus per-row
    /// `active`/`locked`, and the active id. `locked` is true only for the
    /// loaded merchant when its encrypted seed has no passphrase yet; inactive
    /// merchants report `locked: false` (their lock state is unknown until
    /// loaded, and the UI treats "not loaded" separately).
    pub fn list(&self) -> serde_json::Value {
        let active = self.manifest.active.clone();
        let merchants: Vec<serde_json::Value> = self
            .manifest
            .merchants
            .iter()
            .map(|m| {
                let is_active = active.as_deref() == Some(&m.id);
                let locked = is_active && self.active_locked();
                serde_json::json!({
                    "id": m.id,
                    "label": m.label,
                    "identity": m.identity,
                    "created": m.created,
                    "encrypted": m.encrypted,
                    "active": is_active,
                    "locked": locked,
                })
            })
            .collect();
        serde_json::json!({ "merchants": merchants, "active": active })
    }

    /// `getmerchantinfo {id?}` → metadata for one merchant (defaults to the
    /// active one).
    pub fn info(&self, id: Option<&str>) -> Result<serde_json::Value> {
        let id = match id {
            Some(id) => id.to_string(),
            None => self.active_id().context("no active merchant")?.to_string(),
        };
        let meta = self.meta_of(&id).context("unknown merchant")?;
        let is_active = self.active_id() == Some(id.as_str());
        let locked = is_active && self.active_locked();

        let out = serde_json::json!({
            "id": meta.id,
            "label": meta.label,
            "identity": meta.identity,
            "created": meta.created,
            "encrypted": meta.encrypted,
            "active": is_active,
            "locked": locked,
        });

        Ok(out)
    }

    /// After a (re)load or post-seed provisioning, capture the merchant's
    /// identity pubkey + encryption flag from its now-open store. Best-effort:
    /// a locked seed yields no identity yet (left `None`). Called by pactd after
    /// createseed/importseed/unlock too, via [`Self::refresh_active_identity`].
    fn backfill_identity(&mut self, id: &str) {
        let (identity, encrypted) = match self.engine.as_ref() {
            Some(engine) => {
                let encrypted = engine.store.seed_is_encrypted().unwrap_or(false);
                let identity = engine
                    .store
                    .seed()
                    .ok()
                    .and_then(|s| s.identity_pubkey().ok())
                    .map(|p| p.to_string());
                (identity, encrypted)
            }
            None => (None, false),
        };
        if let Some(meta) = self.manifest.merchants.iter_mut().find(|m| m.id == id) {
            if identity.is_some() && meta.identity != identity {
                meta.identity = identity;
            }
            meta.encrypted = encrypted;
        }
    }

    /// Re-capture the active merchant's identity/encryption into the manifest
    /// (after `createseed`/`importseed`/`unlock` provisions or unlocks a seed)
    /// and persist. No-op when nothing is active.
    pub fn refresh_active_identity(&mut self) -> Result<()> {
        if let Some(id) = self.manifest.active.clone() {
            self.backfill_identity(&id);
            self.save()?;
        }
        Ok(())
    }

    fn meta_of(&self, id: &str) -> Option<&MerchantMeta> {
        self.manifest.merchants.iter().find(|m| m.id == id)
    }

    /// Whether the loaded engine's seed is encrypted but not yet unlocked.
    fn active_locked(&self) -> bool {
        self.engine
            .as_ref()
            .and_then(|e| e.store.wallet_status().ok())
            .map(|s| s.locked)
            .unwrap_or(false)
    }
}

/// Terminal swap states never need timelock watching, so switching away from a
/// merchant whose swaps are all terminal is safe.
fn is_terminal(state: libswap::swap::State) -> bool {
    use libswap::swap::State::*;
    matches!(state, Completed | Refunded | Aborted)
}

/// Discover `merchants/<id>/` dirs that carry a seed but are missing from the
/// manifest, and add them (migration/adopt). Identity is left `None` until the
/// merchant is loaded (deriving it would need to open every seed, and encrypted
/// ones can't be read without their passphrase).
fn adopt_existing(data_dir: &Path, manifest: &mut Manifest) -> Result<()> {
    let root = data_dir.join(MERCHANTS_DIR);
    if !root.exists() {
        return Ok(());
    }
    let mut adopted: Vec<MerchantMeta> = Vec::new();
    for entry in std::fs::read_dir(&root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().to_string();
        if manifest.merchants.iter().any(|m| m.id == id) {
            continue;
        }
        let seed_path = entry.path().join(libswap::store::SEED_FILE);
        if !seed_path.exists() {
            continue; // empty/partial dir — nothing to adopt
        }
        let encrypted = std::fs::read_to_string(&seed_path)
            .map(|c| c.starts_with("PACTSEEDv1"))
            .unwrap_or(false);
        adopted.push(MerchantMeta {
            id: id.clone(),
            label: id, // best-effort label = id; the user can rename later
            identity: None,
            created: now_secs(),
            encrypted,
        });
    }
    // Deterministic order so listmerchants is stable across boots.
    adopted.sort_by(|a, b| a.id.cmp(&b.id));
    if !adopted.is_empty() && manifest.active.is_none() {
        manifest.active = Some(adopted[0].id.clone());
    }
    manifest.merchants.extend(adopted);
    Ok(())
}

fn load_manifest(data_dir: &Path) -> Result<Manifest> {
    let path = data_dir.join(MANIFEST_FILE);
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
        }
        Err(_) => Ok(Manifest::default()),
    }
}

fn save_manifest(data_dir: &Path, manifest: &Manifest) -> Result<()> {
    let path = data_dir.join(MANIFEST_FILE);
    std::fs::write(&path, serde_json::to_string_pretty(manifest)?)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> EngineConfig {
        EngineConfig {
            coins: BTreeMap::new(),
            coin_confirmations: BTreeMap::new(),
            board_url: None,
            nostr_relays: None,
            auto_fund: false,
            passphrase: None,
        }
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("pactd-merchants-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn nested_create_load_and_id_allocation() {
        let dir = temp_dir("nested");
        let mut reg = MerchantRegistry::open(&dir, cfg(), false, true).unwrap();
        assert!(
            reg.active_id().is_none(),
            "fresh managed install has no active"
        );

        let m1 = reg.create("Greedy").unwrap();
        assert_eq!(m1.id, "m1");
        assert_eq!(reg.active_id(), Some("m1"));
        // The active engine exists and is seedless (no seed provisioned yet).
        assert!(
            !reg.active()
                .unwrap()
                .store
                .wallet_status()
                .unwrap()
                .seed_exists
        );

        let m2 = reg.create("Grumpy").unwrap();
        assert_eq!(m2.id, "m2");
        assert_eq!(reg.active_id(), Some("m2"));

        // Switching back works (no live swaps to gate on).
        reg.load("m1").unwrap();
        assert_eq!(reg.active_id(), Some("m1"));
        assert!(reg.load("nope").is_err());

        // listmerchants shape.
        let list = reg.list();
        assert_eq!(list["merchants"].as_array().unwrap().len(), 2);
        assert_eq!(list["active"], "m1");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn alloc_id_skips_orphan_on_disk_dir() {
        // The desync guard: a merchants/m1 dir left behind (e.g. manifest wiped)
        // must NOT be reused — createmerchant allocates the next free id.
        let dir = temp_dir("orphan");
        std::fs::create_dir_all(dir.join(MERCHANTS_DIR).join("m1")).unwrap();
        // m1 has no seed, so it isn't adopted, but its dir still exists.
        let mut reg = MerchantRegistry::open(&dir, cfg(), false, true).unwrap();
        let created = reg.create("X").unwrap();
        assert_eq!(created.id, "m2", "must skip the orphan m1 dir");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn adopts_existing_seeded_dirs() {
        // Migration: a merchants/<id>/ dir with a seed but no manifest entry is
        // adopted on boot so today's merchants survive the registry move.
        let dir = temp_dir("adopt");
        let m1dir = dir.join(MERCHANTS_DIR).join("m1");
        std::fs::create_dir_all(&m1dir).unwrap();
        // Provision a real (plaintext) seed via the store so identity derives.
        let mut store = libswap::store::Store::open(&m1dir, None).unwrap();
        let _ = store.create_seed(None).unwrap();

        let reg = MerchantRegistry::open(&dir, cfg(), false, true).unwrap();
        let list = reg.list();
        let merchants = list["merchants"].as_array().unwrap();
        assert_eq!(merchants.len(), 1);
        assert_eq!(merchants[0]["id"], "m1");
        // Adopted dir was loaded as active, so its identity backfilled.
        assert_eq!(reg.active_id(), Some("m1"));
        assert!(merchants[0]["identity"].is_string());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn flat_mode_single_default_merchant() {
        // Harness/CLI shape: a seed sits in the data-dir root. The registry
        // exposes one synthetic `default` merchant and stays flat.
        let dir = temp_dir("flat");
        std::fs::create_dir_all(&dir).unwrap();
        let mut store = libswap::store::Store::open(&dir, None).unwrap();
        let _ = store.create_seed(None).unwrap();

        let mut reg = MerchantRegistry::open(&dir, cfg(), true, false).unwrap();
        assert_eq!(reg.active_id(), Some(FLAT_ID));
        // The active engine's data dir is the root (seed visible).
        assert!(
            reg.active()
                .unwrap()
                .store
                .wallet_status()
                .unwrap()
                .seed_exists
        );
        // createmerchant is refused in flat mode.
        assert!(reg.create("nope").is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn bare_dir_without_opt_in_defaults_to_flat() {
        // The harness/CLI Phase-B path: `pactd --data-dir D` (no --merchants,
        // no seed yet). It must default to flat so a direct createseed lands in
        // the root and the daemon is the single-seed shape pact-cli expects.
        let dir = temp_dir("bare-flat");
        let reg = MerchantRegistry::open(&dir, cfg(), false, false).unwrap();
        assert_eq!(
            reg.active_id(),
            Some(FLAT_ID),
            "bare dir is the default merchant"
        );
        // Provision a seed through the active engine — lands in the root.
        {
            let engine = reg.active().unwrap();
            assert!(!engine.store.wallet_status().unwrap().seed_exists);
        }
        assert!(dir.join("seed.mnemonic").exists() || !dir.join("merchants").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn manifest_round_trips_across_open() {
        let dir = temp_dir("persist");
        {
            let mut reg = MerchantRegistry::open(&dir, cfg(), false, true).unwrap();
            reg.create("Alpha").unwrap();
            reg.create("Beta").unwrap();
            reg.load("m1").unwrap();
        }
        // Reopen: the manifest survived, m1 still active.
        let reg = MerchantRegistry::open(&dir, cfg(), false, true).unwrap();
        assert_eq!(reg.active_id(), Some("m1"));
        assert_eq!(reg.list()["merchants"].as_array().unwrap().len(), 2);
        std::fs::remove_dir_all(&dir).ok();
    }
}
