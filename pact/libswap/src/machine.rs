//! Per-machine seed-derivation scope — the `machine.json` backbone of the
//! multi-machine partition (§1 of docs/MULTI_MACHINE_122.md).
//!
//! Every pactd install owns one random **62-bit** [`DeriveScope`], persisted in
//! a dedicated `machine.json` at the pactd data-dir **root** (above any
//! per-merchant store, so it survives a per-merchant DB loss and is shared by
//! all merchants — different seeds already diverge their keys, so one scope
//! never collides across merchants). The scope is an engine derivation input:
//! injected as two hardened BIP32 levels into every initiator key/preimage, it
//! makes two machines on one seed derive *different* secrets at the same swap
//! counter, closing the catastrophic secret-reuse vector.
//!
//! pactd owns this file end-to-end; Satchel never touches it. It is generated on
//! first read if absent, and **rotated** on the #120 reconfirm-with-mnemonic
//! path so a data-dir *copy* to another machine self-heals to a fresh scope
//! (the old machine keeps the old one → no cross-machine collision).

use crate::keys::DeriveScope;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The machine-scope file, at the pactd data-dir root (per-network for free —
/// the data dir already nests by network).
pub const MACHINE_FILE: &str = "machine.json";

#[derive(Debug, Serialize, Deserialize)]
struct MachineFile {
    /// On-disk schema version (room to grow; only `derive_scope` matters today).
    version: u32,
    /// The install's 62-bit seed-derivation scope. `0` would be the legacy
    /// marker, but a persisted file is always written nonzero.
    derive_scope: u64,
}

/// Draw a fresh nonzero 62-bit scope from the CSPRNG. `0` is the reserved legacy
/// marker (never a real machine's own scope), so the astronomically-unlikely
/// zero draw is retried.
fn fresh_scope() -> u64 {
    use bitcoin::secp256k1::rand::RngCore;
    loop {
        let mut b = [0u8; 8];
        bitcoin::secp256k1::rand::thread_rng().fill_bytes(&mut b);
        let v = u64::from_le_bytes(b) & DeriveScope::MASK;
        if v != 0 {
            return v;
        }
    }
}

fn write_scope(root: &Path, scope: u64) -> Result<DeriveScope> {
    let path = root.join(MACHINE_FILE);
    let mf = MachineFile {
        version: 1,
        derive_scope: scope,
    };
    std::fs::write(&path, serde_json::to_string_pretty(&mf)?)
        .with_context(|| format!("writing machine scope {}", path.display()))?;
    Ok(DeriveScope(scope))
}

/// Read the install's [`DeriveScope`] from `<root>/machine.json`, generating and
/// persisting a fresh one if the file is absent (or unreadable / zero-valued —
/// a corrupt file self-heals to a new scope, which at worst demotes pre-existing
/// in-flight swaps to the confirm-gated recovery path). Call this once with the
/// pactd data-dir **root** and share the result across all merchants.
pub fn load_or_create_scope(root: &Path) -> Result<DeriveScope> {
    let path = root.join(MACHINE_FILE);
    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Ok(mf) = serde_json::from_str::<MachineFile>(&text) {
            let v = mf.derive_scope & DeriveScope::MASK;
            if v != 0 {
                return Ok(DeriveScope(v));
            }
        }
    }
    write_scope(root, fresh_scope())
}

/// A short, stable, human-facing label for a machine scope (§5) — e.g.
/// `"M-7f3a"`. A ONE-WAY tag (tagged hash of the scope, not the raw scope), so
/// the UI can name and group machines — this machine in Settings, and each
/// "Another machine" group of followed swaps — without ever exposing the
/// derivation salt. Two machines get different labels with overwhelming
/// probability; the legacy scope (0) shows as `"M-legacy"`.
pub fn machine_label(scope: DeriveScope) -> String {
    if scope.is_legacy() {
        return "M-legacy".to_string();
    }
    let h = crate::keys::tagged_hash("pact/machine-label/v1", &scope.0.to_be_bytes());
    format!("M-{:02x}{:02x}", h[0], h[1])
}

/// Rotate the install's scope to a fresh random value (the #120 copy-heal path).
/// After a rotation this machine's *own* in-flight swaps carry the OLD scope, so
/// they demote to followed and need an explicit take-over — deliberate: in a
/// data-dir copy the original machine may still be alive, so auto-adopting would
/// recreate the double-drive. Never auto-adopt on rotation.
pub fn rotate_scope(root: &Path) -> Result<DeriveScope> {
    write_scope(root, fresh_scope())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("pact-machine-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn generates_persists_and_is_stable() {
        let dir = tmp("stable");
        let a = load_or_create_scope(&dir).unwrap();
        assert!(!a.is_legacy(), "a fresh scope is never the legacy marker");
        assert!(a.0 <= DeriveScope::MASK, "scope stays within 62 bits");
        // Re-reading returns the SAME persisted scope (no churn per boot).
        let b = load_or_create_scope(&dir).unwrap();
        assert_eq!(a, b);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rotate_changes_the_scope() {
        let dir = tmp("rotate");
        let a = load_or_create_scope(&dir).unwrap();
        let b = rotate_scope(&dir).unwrap();
        assert_ne!(a, b, "rotation draws a fresh scope");
        assert!(!b.is_legacy());
        // The rotated value persists.
        assert_eq!(b, load_or_create_scope(&dir).unwrap());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn corrupt_file_self_heals() {
        let dir = tmp("corrupt");
        std::fs::write(dir.join(MACHINE_FILE), "not json").unwrap();
        let a = load_or_create_scope(&dir).unwrap();
        assert!(!a.is_legacy());
        std::fs::remove_dir_all(&dir).ok();
    }
}
