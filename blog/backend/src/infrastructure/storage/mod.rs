mod dispatch;
pub mod fs;
pub mod r2;

use std::fmt;
use std::path::Path;

#[derive(Debug)]
pub struct StorageError(pub String);

impl fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "storage: {}", self.0)
    }
}

impl std::error::Error for StorageError {}

/// Multipart upload handle returned by [`ObjectStore::create_multipart`]. The
/// id is opaque and is stored in the upload session tables between chunks.
pub struct MultipartSession {
    pub upload_id: String,
}

/// Object store for v86 game artifacts (system chunks, game disks, ISOs,
/// snapshots, saves). `R2` writes to Cloudflare R2 via the S3 API; `Fs` keeps
/// everything on the VM disk under the project-demos root, using the same key
/// layout as the R2 mirror (see `sync_v86_to_r2.sh` / `sync_r2_to_fs.sh`).
#[derive(Clone)]
pub enum ObjectStore {
    R2(r2::R2Client),
    Fs(fs::FsStore),
}

impl ObjectStore {
    /// Selects the backend from `STORAGE_BACKEND` (`auto` | `r2` | `fs`).
    /// `auto` uses R2 when the R2_* variables are configured and falls back to
    /// the filesystem otherwise, so existing deployments keep their behavior.
    pub fn from_env(fs_root: &Path) -> Result<Self, String> {
        let backend = std::env::var("STORAGE_BACKEND").unwrap_or_else(|_| "auto".to_string());
        match backend.as_str() {
            "auto" => Ok(match r2::R2Client::from_env() {
                Some(client) => {
                    println!("Storage backend: Cloudflare R2 (bucket {})", client.bucket);
                    ObjectStore::R2(client)
                }
                None => {
                    println!("Storage backend: filesystem ({})", fs_root.display());
                    ObjectStore::Fs(fs::FsStore::new(fs_root.to_path_buf()))
                }
            }),
            "r2" => r2::R2Client::from_env().map(ObjectStore::R2).ok_or_else(|| {
                "STORAGE_BACKEND=r2 but R2_ACCOUNT_ID / R2_ACCESS_KEY_ID / R2_SECRET_ACCESS_KEY / R2_BUCKET are not fully configured".to_string()
            }),
            "fs" => {
                println!("Storage backend: filesystem ({})", fs_root.display());
                Ok(ObjectStore::Fs(fs::FsStore::new(fs_root.to_path_buf())))
            }
            other => Err(format!(
                "Unsupported STORAGE_BACKEND '{other}' (expected auto, r2, or fs)"
            )),
        }
    }
}
