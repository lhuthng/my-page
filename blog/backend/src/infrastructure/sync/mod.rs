//! Prod -> dev sync support: manifest generation, sync-key handling, and the
//! post-import database fix pass. Shared between the `/sync` API (prod side)
//! and the `sync-pull` binary (dev side).
//!
//! Direction policy: this flow is deliberately pull-only. The `sync_keys`
//! migration constrains `mode` to `'pull'`, and nothing here can write to the
//! source environment — a future push flow needs its own mode, key scoping and
//! confirmation steps.

use sha2::{Digest, Sha256};

pub const SYNC_KEY_PREFIX: &str = "bsk_";

// ── Keys ─────────────────────────────────────────────────────────────────────

/// Generates a new `bsk_<64 hex>` secret. Only its SHA-256 hash is persisted.
pub fn generate_sync_key() -> Result<String, String> {
    let bytes = {
        use rand::RngCore;
        let mut buf = [0u8; 32];
        rand::rng().fill_bytes(&mut buf);
        buf
    };
    Ok(format!("{SYNC_KEY_PREFIX}{}", hex::encode(bytes)))
}

pub fn hash_sync_key(secret: &str) -> String {
    hex::encode(Sha256::digest(secret.as_bytes()))
}

// ── Manifest ─────────────────────────────────────────────────────────────────

mod files;
mod manifest;
mod rewrite;
#[cfg(test)]
mod tests;

pub use files::{artifact_key_exists, is_valid_artifact_key_shape};
pub use manifest::{ArtifactEntry, DemoDir, DemoFile, MediaEntry, SyncManifest, build_manifest};
pub use rewrite::{FixSummary, canonical_media_url, fix_imported_database};
