// ObjectStore method dispatch: each method forwards to the R2 client or the
// filesystem store. `from_env` (backend selection) stays in `mod.rs`.
use std::path::Path;

use super::{MultipartSession, ObjectStore, StorageError};

impl ObjectStore {
    pub async fn create_multipart(&self, key: &str) -> Result<MultipartSession, StorageError> {
        match self {
            ObjectStore::R2(client) => {
                let output = client.create_multipart(key).await?;
                let upload_id = output
                    .upload_id()
                    .ok_or_else(|| {
                        StorageError(format!("create_multipart {key}: missing upload id"))
                    })?
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
