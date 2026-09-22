// Multipart upload sessions on the filesystem backend: session dirs hang
// off the object key as a `<name>.multipart/` sibling, which normal object
// keys never end with.
use std::path::{Path, PathBuf};

use tokio::io::AsyncWriteExt;

use uuid::Uuid;

use super::FsStore;
use crate::infrastructure::storage::{MultipartSession, StorageError};

impl FsStore {
    fn multipart_session_dir(&self, key: &str, upload_id: &str) -> Result<PathBuf, StorageError> {
        // The session dir hangs off the key as a sibling with a ".multipart"
        // suffix, which normal object keys never end with.
        let object_path = self.path_for(key)?;
        let name = object_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| StorageError(format!("invalid storage key '{key}'")))?;
        Ok(object_path
            .with_file_name(format!("{name}.multipart"))
            .join(upload_id))
    }

    /// Removes a multipart session dir and prunes the per-key multipart
    /// parent when the last session leaves it.
    async fn remove_session(&self, session_dir: &Path) -> Result<(), StorageError> {
        tokio::fs::remove_dir_all(session_dir)
            .await
            .map_err(|e| StorageError(format!("multipart session cleanup: {e}")))?;
        if let Some(parent) = session_dir.parent() {
            // Only succeeds when no other session remains; otherwise ignored.
            let _ = tokio::fs::remove_dir(parent).await;
        }
        Ok(())
    }

    pub async fn create_multipart(&self, key: &str) -> Result<MultipartSession, StorageError> {
        let upload_id = Uuid::new_v4().to_string();
        let session_dir = self.multipart_session_dir(key, &upload_id)?;
        tokio::fs::create_dir_all(&session_dir)
            .await
            .map_err(|e| StorageError(format!("create_multipart {key}: {e}")))?;
        Ok(MultipartSession { upload_id })
    }

    pub async fn upload_part(
        &self,
        key: &str,
        upload_id: &str,
        part_number: i32,
        bytes: Vec<u8>,
    ) -> Result<String, StorageError> {
        if part_number < 1 {
            return Err(StorageError(format!(
                "upload_part {key}#{part_number}: part number must be positive"
            )));
        }
        let session_dir = self.multipart_session_dir(key, upload_id)?;
        if !session_dir.is_dir() {
            return Err(StorageError(format!(
                "upload_part {key}#{part_number}: multipart session not found"
            )));
        }
        let part_path = session_dir.join(format!("part-{part_number:08}"));
        tokio::fs::write(&part_path, &bytes)
            .await
            .map_err(|e| StorageError(format!("upload_part {key}#{part_number}: {e}")))?;
        Ok(format!("fs-{part_number:08}-{}", bytes.len()))
    }

    /// Concatenates the uploaded parts in the order given, writes the final
    /// object atomically (temp file + rename), and drops the session dir.
    pub async fn complete_multipart(
        &self,
        key: &str,
        upload_id: &str,
        parts: Vec<(i32, String)>,
    ) -> Result<(), StorageError> {
        if parts.is_empty() {
            return Err(StorageError(format!(
                "complete_multipart {key}: no parts uploaded"
            )));
        }
        let session_dir = self.multipart_session_dir(key, upload_id)?;
        if !session_dir.is_dir() {
            return Err(StorageError(format!(
                "complete_multipart {key}: multipart session not found"
            )));
        }
        let object_path = self.path_for(key)?;
        let parent = object_path.parent().ok_or_else(|| {
            StorageError(format!(
                "complete_multipart {key}: key has no parent directory"
            ))
        })?;
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| StorageError(format!("complete_multipart {key}: create parent: {e}")))?;
        let tmp_path = parent.join(format!(
            ".{}.{}.tmp",
            object_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("object"),
            upload_id
        ));
        let mut output = tokio::fs::File::create(&tmp_path)
            .await
            .map_err(|e| StorageError(format!("complete_multipart {key}: {e}")))?;
        for (part_number, _) in parts {
            let part_path = session_dir.join(format!("part-{part_number:08}"));
            let mut part = tokio::fs::File::open(&part_path).await.map_err(|e| {
                StorageError(format!("complete_multipart {key}: part {part_number}: {e}"))
            })?;
            tokio::io::copy(&mut part, &mut output).await.map_err(|e| {
                StorageError(format!("complete_multipart {key}: part {part_number}: {e}"))
            })?;
        }
        output
            .flush()
            .await
            .map_err(|e| StorageError(format!("complete_multipart {key}: flush: {e}")))?;
        drop(output);
        tokio::fs::rename(&tmp_path, &object_path)
            .await
            .map_err(|e| StorageError(format!("complete_multipart {key}: finalize: {e}")))?;
        self.remove_session(&session_dir)
            .await
            .map_err(|e| StorageError(format!("complete_multipart {key}: {e}")))?;
        Ok(())
    }

    pub async fn abort_multipart(&self, key: &str, upload_id: &str) -> Result<(), StorageError> {
        let session_dir = self.multipart_session_dir(key, upload_id)?;
        if !session_dir.is_dir() {
            return Ok(());
        }
        self.remove_session(&session_dir).await
    }
}
