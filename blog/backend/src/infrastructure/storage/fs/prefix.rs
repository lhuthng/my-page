// Ranged reads (HTTP Range requests) and prefix listing/deletion on the
// filesystem backend.
use super::FsStore;
use crate::infrastructure::storage::StorageError;

impl FsStore {
    pub async fn get_object_range(
        &self,
        key: &str,
        start: u64,
        end: u64,
    ) -> Result<Vec<u8>, StorageError> {
        use tokio::io::{AsyncReadExt, AsyncSeekExt};
        if end < start {
            return Err(StorageError(format!(
                "get_object_range {key}: end before start"
            )));
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
                        )));
                    }
                };
                for entry in entries {
                    let entry = match entry {
                        Ok(entry) => entry,
                        Err(e) => {
                            return Err(StorageError(format!(
                                "list_prefix {prefix_owned}: {}: {e}",
                                dir.display()
                            )));
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
