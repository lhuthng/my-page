// File transfer plumbing: the fetch primitive, local tree listing, and
// pruning of files the source manifest no longer lists.
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use futures::StreamExt;
use tokio::io::AsyncWriteExt;

/// Outcome of one file fetch.
pub enum Fetched {
    /// Newly downloaded (or would be, in dry-run).
    Downloaded,
    /// Already present with the expected size.
    Current,
    /// Source answered 404 and missing files are allowed.
    Missing,
}

/// Downloads `url` into `target` (temp file + rename). `Missing` = the source
/// has no such file: the manifest is built from the database, so it can list
/// media rows whose file no longer exists on the source disk.
pub async fn download_to_file(
    client: &reqwest::Client,
    url: &str,
    key: &str,
    target: &Path,
    expected_size: Option<u64>,
    allow_missing: bool,
    dry_run: bool,
) -> Result<Fetched, String> {
    if let Some(size) = expected_size
        && target.is_file()
        && let Ok(meta) = tokio::fs::metadata(target).await
        && meta.len() == size
    {
        return Ok(Fetched::Current);
    }
    if dry_run {
        return Ok(Fetched::Downloaded);
    }
    if let Some(parent) = target.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    let response = client
        .get(url)
        .bearer_auth(key)
        .send()
        .await
        .map_err(|e| format!("GET {url}: {e}"))?;
    if !response.status().is_success() {
        let status = response.status();
        if allow_missing && status == reqwest::StatusCode::NOT_FOUND {
            return Ok(Fetched::Missing);
        }
        let body = response.text().await.unwrap_or_default();
        return Err(format!("GET {url}: {status} {}", body.trim()));
    }
    let temp = target.with_extension(format!(
        "{}.sync-tmp",
        target
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
    ));
    let mut file = tokio::fs::File::create(&temp)
        .await
        .map_err(|e| format!("create {}: {e}", temp.display()))?;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| format!("stream {url}: {e}"))?;
        file.write_all(&bytes)
            .await
            .map_err(|e| format!("write {}: {e}", temp.display()))?;
    }
    file.flush()
        .await
        .map_err(|e| format!("flush {}: {e}", temp.display()))?;
    drop(file);
    tokio::fs::rename(&temp, target)
        .await
        .map_err(|e| format!("finalize {}: {e}", target.display()))?;
    Ok(Fetched::Downloaded)
}

pub fn collect_local_files(dir: &Path) -> Vec<(PathBuf, u64)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                out.push((path, size));
            }
        }
    }
    out
}

pub async fn prune_extras(
    local_dir: &Path,
    expected: &HashSet<String>,
    dry_run: bool,
) -> (usize, u64) {
    let mut removed = 0usize;
    let mut freed = 0u64;
    for (path, size) in collect_local_files(local_dir) {
        let Ok(relative) = path.strip_prefix(local_dir) else {
            continue;
        };
        if !expected.contains(&relative.to_string_lossy().to_string()) {
            freed += size;
            removed += 1;
            if !dry_run {
                let _ = tokio::fs::remove_file(&path).await;
            }
        }
    }
    (removed, freed)
}
