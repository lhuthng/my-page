use std::path::{Path, PathBuf};

use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use super::StorageError;

/// Filesystem object store. Keys map to paths under the project-demos root
/// (`{PROJECT_DEMOS_PATH}/{key}`), the same layout `sync_v86_to_r2.sh` mirrors
/// into R2, so artifacts stay compatible with either backend and are covered
/// by the compose volume and the backup download.
#[derive(Clone)]
pub struct FsStore {
    root: PathBuf,
}

impl FsStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Resolves a key to a path inside the root. Keys are constructed by
    /// backend code from validated hashes, but anything containing traversal
    /// or empty segments must never reach the disk.
    fn path_for(&self, key: &str) -> Result<PathBuf, StorageError> {
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

    fn multipart_session_dir(&self, key: &str, upload_id: &str) -> Result<PathBuf, StorageError> {
        // The session dir hangs off the key as a sibling with a ".multipart"
        // suffix, which normal object keys never end with.
        let object_path = self.path_for(key)?;
        let name = object_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| StorageError(format!("invalid storage key '{key}'")))?;
        Ok(object_path.with_file_name(format!("{name}.multipart")).join(upload_id))
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

    pub async fn create_multipart(&self, key: &str) -> Result<super::MultipartSession, StorageError> {
        let upload_id = Uuid::new_v4().to_string();
        let session_dir = self.multipart_session_dir(key, &upload_id)?;
        tokio::fs::create_dir_all(&session_dir)
            .await
            .map_err(|e| StorageError(format!("create_multipart {key}: {e}")))?;
        Ok(super::MultipartSession { upload_id })
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
            return Err(StorageError(format!("complete_multipart {key}: no parts uploaded")));
        }
        let session_dir = self.multipart_session_dir(key, upload_id)?;
        if !session_dir.is_dir() {
            return Err(StorageError(format!(
                "complete_multipart {key}: multipart session not found"
            )));
        }
        let object_path = self.path_for(key)?;
        let parent = object_path.parent().ok_or_else(|| {
            StorageError(format!("complete_multipart {key}: key has no parent directory"))
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
            let mut part = tokio::fs::File::open(&part_path)
                .await
                .map_err(|e| StorageError(format!("complete_multipart {key}: part {part_number}: {e}")))?;
            tokio::io::copy(&mut part, &mut output)
                .await
                .map_err(|e| StorageError(format!("complete_multipart {key}: part {part_number}: {e}")))?;
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

    pub async fn put_object_from_file(&self, key: &str, path: &Path) -> Result<(), StorageError> {
        let object_path = self.path_for(key)?;
        let parent = object_path.parent().ok_or_else(|| {
            StorageError(format!("put_object_from_file {key}: key has no parent directory"))
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
            StorageError(format!("put_object_bytes {key}: key has no parent directory"))
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
    pub async fn get_object_range(
        &self,
        key: &str,
        start: u64,
        end: u64,
    ) -> Result<Vec<u8>, StorageError> {
        use tokio::io::{AsyncReadExt, AsyncSeekExt};
        if end < start {
            return Err(StorageError(format!("get_object_range {key}: end before start")));
        }
        let object_path = self.path_for(key)?;
        let mut file = tokio::fs::File::open(&object_path)
            .await
            .map_err(|_| StorageError(format!("get_object_range {key}: not found")))?;
        file.seek(std::io::SeekFrom::Start(start))
            .await
            .map_err(|e| StorageError(format!("get_object_range {key}: seek: {e}")))?;
        let length = end - start + 1;
        let mut buffer = Vec::with_capacity(length as usize);
        file.take(length)
            .read_to_end(&mut buffer)
            .await
            .map_err(|e| StorageError(format!("get_object_range {key}: read: {e}")))?;
        Ok(buffer)
    }

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

    /// Lists every object under a prefix as `(key, size)` pairs, walking the
    /// directory tree on the blocking pool. The prefix must not have a
    /// trailing slash; a missing prefix yields an empty list.
    pub async fn list_prefix(&self, prefix: &str) -> Result<Vec<(String, u64)>, StorageError> {
        let base = self.path_for(prefix)?;
        if !base.is_dir() {
            return Ok(Vec::new());
        }
        let root = self.root.clone();
        let prefix_owned = prefix.to_string();
        tokio::task::spawn_blocking(move || {
            let mut objects = Vec::new();
            let mut stack = vec![base.clone()];
            while let Some(dir) = stack.pop() {
                let entries = match std::fs::read_dir(&dir) {
                    Ok(entries) => entries,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(e) => {
                        return Err(StorageError(format!(
                            "list_prefix {prefix_owned}: {}: {e}",
                            dir.display()
                        )))
                    }
                };
                for entry in entries {
                    let entry = match entry {
                        Ok(entry) => entry,
                        Err(e) => {
                            return Err(StorageError(format!(
                                "list_prefix {prefix_owned}: {}: {e}",
                                dir.display()
                            )))
                        }
                    };
                    let path = entry.path();
                    if path.is_dir() {
                        stack.push(path);
                    } else if path.is_file() {
                        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                        if let Ok(relative) = path.strip_prefix(&root) {
                            objects.push((relative.to_string_lossy().to_string(), size));
                        }
                    }
                }
            }
            Ok(objects)
        })
        .await
        .map_err(|e| StorageError(format!("list_prefix {prefix}: join: {e}")))?
    }

    /// Removes a whole prefix directory. Keys under one prefix always map to a
    /// single directory (e.g. `v86/games/{sha256}`), so this matches the R2
    /// prefix-delete semantics.
    pub async fn delete_prefix(&self, prefix: &str) -> Result<(), StorageError> {
        match tokio::fs::remove_dir_all(self.path_for(prefix)?).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(StorageError(format!("delete_prefix {prefix}: {e}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (FsStore, PathBuf) {
        let root = std::env::temp_dir().join(format!("fs-store-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        (FsStore::new(root.clone()), root)
    }

    #[tokio::test]
    async fn put_get_roundtrip_and_missing_object() {
        let (store, root) = store();
        store
            .put_object_bytes("v86/saves/1/2/save.zst", b"hello save".to_vec())
            .await
            .unwrap();
        assert_eq!(store.get_object("v86/saves/1/2/save.zst").await.unwrap(), b"hello save");
        assert_eq!(
            store.object_size("v86/saves/1/2/save.zst").await.unwrap(),
            Some(10)
        );
        assert_eq!(store.object_size("v86/saves/1/2/missing").await.unwrap(), None);
        assert!(store.get_object("v86/saves/1/2/missing").await.is_err());
        std::fs::remove_dir_all(root).ok();
    }

    #[tokio::test]
    async fn multipart_roundtrip_with_out_of_order_parts() {
        let (store, root) = store();
        let key = "v86/tmp/snapshots/upload-1.zst";
        let session = store.create_multipart(key).await.unwrap();
        let etag1 = store
            .upload_part(key, &session.upload_id, 1, b"aaaa".to_vec())
            .await
            .unwrap();
        let etag2 = store
            .upload_part(key, &session.upload_id, 2, b"bb".to_vec())
            .await
            .unwrap();
        store
            .complete_multipart(key, &session.upload_id, vec![(2, etag2), (1, etag1)])
            .await
            .unwrap();
        assert_eq!(store.get_object(key).await.unwrap(), b"bbaaaa");
        // The session dir is gone and the final object replaced the tmp files.
        assert!(!root.join("v86/tmp/snapshots/upload-1.zst.multipart").exists());
        std::fs::remove_dir_all(root).ok();
    }

    #[tokio::test]
    async fn multipart_abort_removes_session() {
        let (store, root) = store();
        let key = "v86/tmp/snapshots/upload-2.zst";
        let session = store.create_multipart(key).await.unwrap();
        store
            .upload_part(key, &session.upload_id, 1, b"data".to_vec())
            .await
            .unwrap();
        store
            .abort_multipart(key, &session.upload_id)
            .await
            .unwrap();
        assert!(!root.join("v86/tmp/snapshots/upload-2.zst.multipart").exists());
        assert_eq!(store.object_size(key).await.unwrap(), None);
        // Aborting an unknown session is a no-op, like S3.
        store.abort_multipart(key, "does-not-exist").await.unwrap();
        std::fs::remove_dir_all(root).ok();
    }

    #[tokio::test]
    async fn delete_object_and_prefix_are_idempotent() {
        let (store, root) = store();
        store
            .put_object_bytes("v86/games/sha1/full.iso", b"iso".to_vec())
            .await
            .unwrap();
        store
            .put_object_bytes("v86/games/sha1/0-4.img.zst", b"part".to_vec())
            .await
            .unwrap();
        store.delete_object("v86/games/sha1/full.iso").await.unwrap();
        store.delete_object("v86/games/sha1/full.iso").await.unwrap();
        store.delete_prefix("v86/games/sha1").await.unwrap();
        store.delete_prefix("v86/games/sha1").await.unwrap();
        assert_eq!(store.object_size("v86/games/sha1/0-4.img.zst").await.unwrap(), None);
        std::fs::remove_dir_all(root).ok();
    }

    #[tokio::test]
    async fn traversal_keys_are_rejected() {
        let (store, root) = store();
        for key in ["../escape", "a/../../b", "/absolute", "a/./b", "a/../b", ""] {
            assert!(
                store.put_object_bytes(key, b"x".to_vec()).await.is_err(),
                "'{key}' should be rejected"
            );
        }
        // Nothing escaped the root.
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
        std::fs::remove_dir_all(root).ok();
    }

    #[tokio::test]
    async fn get_object_range_reads_inclusive_bounds() {
        let (store, root) = store();
        store
            .put_object_bytes("v86/assets/systems/sha/base.img.zst", b"0123456789".to_vec())
            .await
            .unwrap();
        assert_eq!(
            store
                .get_object_range("v86/assets/systems/sha/base.img.zst", 2, 4)
                .await
                .unwrap(),
            b"234"
        );
        std::fs::remove_dir_all(root).ok();
    }

    #[tokio::test]
    async fn put_object_from_file_copies_content() {
        let (store, root) = store();
        let source = root.join("source.bin");
        std::fs::write(&source, b"built artifact").unwrap();
        store
            .put_object_from_file("v86/games/sha2/base.img.zst", &source)
            .await
            .unwrap();
        assert_eq!(
            store.get_object("v86/games/sha2/base.img.zst").await.unwrap(),
            b"built artifact"
        );
        std::fs::remove_dir_all(root).ok();
    }
}
