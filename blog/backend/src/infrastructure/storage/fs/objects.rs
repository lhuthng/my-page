// Object read/write primitives on the filesystem backend: atomic puts
// (temp file + rename), whole-object reads, and deletes. Ranged reads and
// prefix operations live in `prefix`.
use std::path::{Path, PathBuf};

use uuid::Uuid;

use super::FsStore;
use crate::infrastructure::storage::StorageError;

impl FsStore {
    /// Resolves a key to a path inside the root. Keys are constructed by
    /// backend code from validated hashes, but anything containing traversal
    /// or empty segments must never reach the disk.
    pub(super) fn path_for(&self, key: &str) -> Result<PathBuf, StorageError> {
        let invalid = key.is_empty()
            || key.starts_with('/')
            || key.contains('\\')
            || key
                .split('/')
                .any(|segment| segment.is_empty() || segment == "." || segment == "..");
        if invalid {
            return Err(StorageError(format!("invalid storage key '{key}'")));
        }
        Ok(self.root.join(key))
    }

    pub async fn put_object_from_file(&self, key: &str, path: &Path) -> Result<(), StorageError> {
        let object_path = self.path_for(key)?;
        let parent = object_path.parent().ok_or_else(|| {
            StorageError(format!(
                "put_object_from_file {key}: key has no parent directory"
            ))
        })?;
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| StorageError(format!("put_object_from_file {key}: create parent: {e}")))?;
        let tmp_path = parent.join(format!(
            ".{}.{}.tmp",
            object_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("object"),
            Uuid::new_v4()
        ));
        tokio::fs::copy(path, &tmp_path)
            .await
            .map_err(|e| StorageError(format!("put_object_from_file {key}: {e}")))?;
        tokio::fs::rename(&tmp_path, &object_path)
            .await
            .map_err(|e| StorageError(format!("put_object_from_file {key}: finalize: {e}")))?;
        Ok(())
    }

    pub async fn put_object_bytes(&self, key: &str, bytes: Vec<u8>) -> Result<(), StorageError> {
        let object_path = self.path_for(key)?;
        let parent = object_path.parent().ok_or_else(|| {
            StorageError(format!(
                "put_object_bytes {key}: key has no parent directory"
            ))
        })?;
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| StorageError(format!("put_object_bytes {key}: create parent: {e}")))?;
        let tmp_path = parent.join(format!(
            ".{}.{}.tmp",
            object_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("object"),
            Uuid::new_v4()
        ));
        tokio::fs::write(&tmp_path, &bytes)
            .await
            .map_err(|e| StorageError(format!("put_object_bytes {key}: {e}")))?;
        tokio::fs::rename(&tmp_path, &object_path)
            .await
            .map_err(|e| StorageError(format!("put_object_bytes {key}: finalize: {e}")))?;
        Ok(())
    }

    /// Reads a byte range of an object (inclusive of `end`).
    pub async fn get_object(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        tokio::fs::read(self.path_for(key)?)
            .await
            .map_err(|_| StorageError(format!("get_object {key}: not found")))
    }

    pub async fn get_object_reader(
        &self,
        key: &str,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Send + Unpin>, StorageError> {
        let file = tokio::fs::File::open(self.path_for(key)?)
            .await
            .map_err(|_| StorageError(format!("get_object {key}: not found")))?;
        Ok(Box::new(file))
    }

    pub async fn object_size(&self, key: &str) -> Result<Option<u64>, StorageError> {
        match tokio::fs::metadata(self.path_for(key)?).await {
            Ok(metadata) => Ok(Some(metadata.len())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(StorageError(format!("object_size {key}: {e}"))),
        }
    }

    pub async fn download_to_file(&self, key: &str, path: &Path) -> Result<(), StorageError> {
        tokio::fs::copy(self.path_for(key)?, path)
            .await
            .map_err(|e| StorageError(format!("download_to_file {key}: {e}")))?;
        Ok(())
    }

    pub async fn delete_object(&self, key: &str) -> Result<(), StorageError> {
        match tokio::fs::remove_file(self.path_for(key)?).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(StorageError(format!("delete_object {key}: {e}"))),
        }
    }
}
