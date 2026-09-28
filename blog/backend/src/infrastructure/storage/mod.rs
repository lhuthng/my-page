mod dispatch;
pub mod fs;
pub mod media_key;
pub mod r2;

use std::env;
use std::fmt;
use std::path::Path;

pub use media_key::{AUDIO_OBJECT_PREFIX, MediaKey, audio_object_key, is_audio, media_key};

/// `Cache-Control` every media object carries. It is part of the object's
/// contract rather than a caller's choice: it is the header the client ends up
/// caching on once it is talking to the bucket directly, which is why the
/// redirect that sends it there does not itself need to be cacheable.
pub const MEDIA_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

#[derive(Debug)]
pub struct StorageError(pub String);

impl fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "storage: {}", self.0)
    }
}

impl std::error::Error for StorageError {}

/// Connection settings for one S3-compatible bucket.
///
/// One bucket, from one variable set: `retro-games` holds the v86 artifacts
/// under `v86/` and the audiobook audio under `audio/`. The two are switched
/// independently — `STORAGE_BACKEND` for the artifacts, `AUDIO_BACKEND` for the
/// audio — so either can be on the bucket or on the disk on its own. Nothing
/// else is in the bucket: images, covers, avatars, video and models are
/// disk-only, see `media_key::audio_object_key`.
pub struct S3Settings {
    pub endpoint: String,
    /// R2 ignores this and wants `auto`; a real S3 endpoint needs its region.
    pub region: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
}

impl S3Settings {
    /// The bucket from the `R2_*` variables. `None` when the account is not
    /// configured at all, which is what `STORAGE_BACKEND=auto` uses to choose
    /// between R2 and the filesystem.
    pub fn from_r2_env() -> Option<Self> {
        let account_id = env::var("R2_ACCOUNT_ID").ok()?;
        let access_key = env::var("R2_ACCESS_KEY_ID").ok()?;
        let secret_key = env::var("R2_SECRET_ACCESS_KEY").ok()?;
        let bucket = env::var("R2_BUCKET").ok()?;

        Some(Self {
            endpoint: env::var("R2_ENDPOINT")
                .unwrap_or_else(|_| format!("https://{account_id}.r2.cloudflarestorage.com")),
            region: "auto".to_string(),
            bucket,
            access_key,
            secret_key,
        })
    }

    /// The same settings, but an error naming every missing variable instead of
    /// a `None`.
    ///
    /// Used where R2 has been explicitly asked for — `STORAGE_BACKEND=r2` or
    /// `AUDIO_BACKEND=r2` — and absence is therefore a misconfiguration rather
    /// than a reason to fall back to the disk. Failing at startup beats failing
    /// at the first upload.
    pub fn require_r2_env() -> Result<Self, String> {
        Self::from_r2_env().ok_or_else(|| {
            let missing: Vec<&str> = [
                "R2_ACCOUNT_ID",
                "R2_ACCESS_KEY_ID",
                "R2_SECRET_ACCESS_KEY",
                "R2_BUCKET",
            ]
            .into_iter()
            .filter(|name| env::var(name).is_err())
            .collect();
            format!("{} not set", missing.join(", "))
        })
    }
}

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
            "r2" => S3Settings::require_r2_env()
                .map(r2::R2Client::from_settings)
                .map(ObjectStore::R2)
                .map_err(|missing| format!("STORAGE_BACKEND=r2 but {missing}")),
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
