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

    pub async fn create_multipart(&self, key: &str) -> Result<MultipartSession, StorageError> {
        match self {
            ObjectStore::R2(client) => {
                let output = client.create_multipart(key).await?;
                let upload_id = output
                    .upload_id()
                    .ok_or_else(|| StorageError(format!("create_multipart {key}: missing upload id")))?
                    .to_string();
                Ok(MultipartSession { upload_id })
            }
            ObjectStore::Fs(store) => store.create_multipart(key).await,
        }
    }

    /// Uploads one part and returns its opaque etag (persisted with the session
    /// and handed back to `complete_multipart`).
    pub async fn upload_part(
        &self,
        key: &str,
        upload_id: &str,
        part_number: i32,
        bytes: Vec<u8>,
    ) -> Result<String, StorageError> {
        match self {
            ObjectStore::R2(client) => client.upload_part(key, upload_id, part_number, bytes).await,
            ObjectStore::Fs(store) => store.upload_part(key, upload_id, part_number, bytes).await,
        }
    }

    pub async fn complete_multipart(
        &self,
        key: &str,
        upload_id: &str,
        parts: Vec<(i32, String)>,
    ) -> Result<(), StorageError> {
        match self {
            ObjectStore::R2(client) => client.complete_multipart(key, upload_id, parts).await,
            ObjectStore::Fs(store) => store.complete_multipart(key, upload_id, parts).await,
        }
    }

    pub async fn abort_multipart(&self, key: &str, upload_id: &str) -> Result<(), StorageError> {
        match self {
            ObjectStore::R2(client) => client.abort_multipart(key, upload_id).await,
            ObjectStore::Fs(store) => store.abort_multipart(key, upload_id).await,
        }
    }

    /// Streams a local file (produced by a transient build) into the store.
    pub async fn put_object_from_file(&self, key: &str, path: &Path) -> Result<(), StorageError> {
        match self {
            ObjectStore::R2(client) => client.put_object_from_file(key, path).await,
            ObjectStore::Fs(store) => store.put_object_from_file(key, path).await,
        }
    }

    /// Uploads an in-memory blob (small per-user floppy saves, ISOs).
    pub async fn put_object_bytes(&self, key: &str, bytes: Vec<u8>) -> Result<(), StorageError> {
        match self {
            ObjectStore::R2(client) => client.put_object_bytes(key, bytes).await,
            ObjectStore::Fs(store) => store.put_object_bytes(key, bytes).await,
        }
    }

    /// Reads a byte range of an object (inclusive of `end`).
    pub async fn get_object_range(
        &self,
        key: &str,
        start: u64,
        end: u64,
    ) -> Result<Vec<u8>, StorageError> {
        match self {
            ObjectStore::R2(client) => client.get_object_range(key, start, end).await,
            ObjectStore::Fs(store) => store.get_object_range(key, start, end).await,
        }
    }

    /// Reads an entire object into memory (used by the snapshot promotion flow).
    pub async fn get_object(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        match self {
            ObjectStore::R2(client) => client.get_object(key).await,
            ObjectStore::Fs(store) => store.get_object(key).await,
        }
    }

    /// Streams an object's bytes (used by the serving routes).
    pub async fn get_object_reader(
        &self,
        key: &str,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Send + Unpin>, StorageError> {
        match self {
            ObjectStore::R2(client) => client.get_object_reader(key).await,
            ObjectStore::Fs(store) => store.get_object_reader(key).await,
        }
    }

    /// Returns the size of an object, or None when it does not exist.
    pub async fn object_size(&self, key: &str) -> Result<Option<u64>, StorageError> {
        match self {
            ObjectStore::R2(client) => client.object_size(key).await,
            ObjectStore::Fs(store) => store.object_size(key).await,
        }
    }

    /// Downloads an entire object to a transient local file for the build step.
    pub async fn download_to_file(&self, key: &str, path: &Path) -> Result<(), StorageError> {
        match self {
            ObjectStore::R2(client) => client.download_to_file(key, path).await,
            ObjectStore::Fs(store) => store.download_to_file(key, path).await,
        }
    }

    pub async fn delete_object(&self, key: &str) -> Result<(), StorageError> {
        match self {
            ObjectStore::R2(client) => client.delete_object(key).await,
            ObjectStore::Fs(store) => store.delete_object(key).await,
        }
    }

    /// Deletes every object under a prefix (used for version/system cleanup).
    pub async fn delete_prefix(&self, prefix: &str) -> Result<(), StorageError> {
        match self {
            ObjectStore::R2(client) => client.delete_prefix(prefix).await,
            ObjectStore::Fs(store) => store.delete_prefix(prefix).await,
        }
    }

    /// Lists every object under a prefix as `(key, size)` pairs (used by the
    /// sync manifest to enumerate artifacts that actually exist). The prefix
    /// must not have a trailing slash.
    pub async fn list_prefix(&self, prefix: &str) -> Result<Vec<(String, u64)>, StorageError> {
        match self {
            ObjectStore::R2(client) => client.list_prefix(prefix).await,
            ObjectStore::Fs(store) => store.list_prefix(prefix).await,
        }
    }
}
