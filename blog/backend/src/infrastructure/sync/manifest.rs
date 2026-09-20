//! Prod -> dev sync support: manifest generation, sync-key handling, and the
//! post-import database fix pass. Shared between the `/sync` API (prod side)
//! and the `sync-pull` binary (dev side).
//!
//! Direction policy: this flow is deliberately pull-only. The `sync_keys`
//! migration constrains `mode` to `'pull'`, and nothing here can write to the
//! source environment — a future push flow needs its own mode, key scoping and
//! confirmation steps.

// The transfer manifest: the JSON shape the pull flow fetches first, and the
// directory walker that builds it.
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::domain::entities::media::MediaType;
use crate::infrastructure::storage::ObjectStore;

pub struct SyncManifest {
    pub generated_at: String,
    /// Backend the source environment currently uses for v86 artifacts.
    pub storage_backend: String,
    pub database_size_bytes: u64,
    pub media: Vec<MediaEntry>,
    /// Extracted demo file trees, keyed by project/game id. The dev tool
    /// writes these under its own PROJECT_DEMOS_PATH.
    pub project_demos: Vec<DemoDir>,
    pub game_demos: Vec<DemoDir>,
    /// js-dos + v86 artifact keys relative to PROJECT_DEMOS_PATH, plus the
    /// media-independent `jsdos/` bundles.
    pub artifacts: Vec<ArtifactEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MediaEntry {
    pub hash: String,
    pub file_type: String,
    pub size: i64,
    /// File location relative to the source MEDIA_PATH, computed from the
    /// canonical content-addressed layout. The dev tool joins this with its
    /// own MEDIA_PATH root.
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DemoDir {
    pub id: i64,
    pub files: Vec<DemoFile>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DemoFile {
    pub path: String,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArtifactEntry {
    pub key: String,
    pub size: u64,
}

/// Walks a directory and returns `(relative_path, size)` pairs, skipping
/// transient upload artifacts (dotfiles, `.multipart` sessions, tmp files).
pub(super) fn walk_demo_dir(base: &Path) -> Result<Vec<DemoFile>, std::io::Error> {
    let mut files = Vec::new();
    if !base.is_dir() {
        return Ok(files);
    }
    let mut stack = vec![base.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file()
                && !name.starts_with('.')
                && !name.ends_with(".tmp")
                && !name.contains(".multipart")
            {
                let relative = path
                    .strip_prefix(base)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                files.push(DemoFile {
                    path: relative.to_string_lossy().to_string(),
                    size: entry.metadata().map(|m| m.len()).unwrap_or(0),
                });
            }
        }
    }
    Ok(files)
}

/// Builds the full sync manifest from the database plus whatever the object
/// store actually contains, so both R2 and fs source environments produce the
/// same shape.
pub async fn build_manifest(
    pool: &sqlx::SqlitePool,
    media_dir: &Path,
    demos_dir: &Path,
    storage: &ObjectStore,
    db_path: &Path,
) -> Result<SyncManifest, String> {
    let media_rows: Vec<(String, String, i64, i64)> = sqlx::query_as(
        "SELECT hash, file_type, COALESCE(size, 0), COALESCE(uploader_id, 0)
         FROM media WHERE hash IS NOT NULL AND hash != ''",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    let mut media = Vec::new();
    for (hash, file_type, size, uploader_id) in media_rows {
        // Rows whose canonical path cannot be derived are unservable anyway
        // (the /sync/media endpoint reconstructs the same path), so skip them.
        if let Some(canonical) = canonical_media_url(&hash, &file_type, uploader_id, media_dir) {
            let path = Path::new(&canonical)
                .strip_prefix(media_dir)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or(canonical);
            media.push(MediaEntry {
                hash,
                file_type,
                size,
                path,
            });
        }
    }

    let project_ids: Vec<(i64,)> =
        sqlx::query_as("SELECT id FROM projects ORDER BY id").fetch_all(pool).await.map_err(|e| e.to_string())?;
    let game_ids: Vec<(i64,)> =
        sqlx::query_as("SELECT id FROM games ORDER BY id").fetch_all(pool).await.map_err(|e| e.to_string())?;

    let walk_projects = {
        let demos_dir = demos_dir.to_path_buf();
        let ids: Vec<i64> = project_ids.iter().map(|(id,)| *id).collect();
        tokio::task::spawn_blocking(move || {
            ids.into_iter()
                .map(|id| {
                    Ok::<_, std::io::Error>(DemoDir {
                        id,
                        files: walk_demo_dir(&demos_dir.join(id.to_string()))?,
                    })
                })
                .filter(|dir| dir.as_ref().map(|d| !d.files.is_empty()).unwrap_or(false))
                .collect::<Result<Vec<DemoDir>, std::io::Error>>()
        })
        .await
        .map_err(|e| e.to_string())?
    };

    let walk_games = {
        let demos_dir = demos_dir.to_path_buf();
        let ids: Vec<i64> = game_ids.iter().map(|(id,)| *id).collect();
        tokio::task::spawn_blocking(move || {
            ids.into_iter()
                .map(|id| {
                    Ok::<_, std::io::Error>(DemoDir {
                        id,
                        files: walk_demo_dir(&demos_dir.join(format!("game-{id}")))?,
                    })
                })
                .filter(|dir| dir.as_ref().map(|d| !d.files.is_empty()).unwrap_or(false))
                .collect::<Result<Vec<DemoDir>, std::io::Error>>()
        })
        .await
        .map_err(|e| e.to_string())?
    };

    let project_demos = walk_projects.map_err(|e| format!("walk project demos: {e}"))?;
    let game_demos = walk_games.map_err(|e| format!("walk game demos: {e}"))?;

    // v86 artifacts live in the object store (R2 or fs); js-dos bundles are
    // DB-addressed and always on the source's disk, so the DB is authoritative
    // for them.
    let mut artifacts: Vec<ArtifactEntry> = storage
        .list_prefix("v86")
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|(key, _)| !key.starts_with("v86/tmp/"))
        .map(|(key, size)| ArtifactEntry { key, size })
        .collect();
    let jsdos_rows: Vec<(String, i64)> =
        sqlx::query_as("SELECT storage_key, size_bytes FROM game_jsdos_bundles")
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;
    for (key, size) in jsdos_rows {
        artifacts.push(ArtifactEntry {
            key,
            size: size.max(0) as u64,
        });
    }

    let database_size_bytes = tokio::fs::metadata(db_path)
        .await
        .map(|m| m.len())
        .unwrap_or(0);

    Ok(SyncManifest {
        generated_at: chrono::Utc::now().to_rfc3339(),
        storage_backend: match storage {
            ObjectStore::R2(_) => "r2".to_string(),
            ObjectStore::Fs(_) => "fs".to_string(),
        },
        database_size_bytes,
        media,
        project_demos,
        game_demos,
        artifacts,
    })
}

/// Shape check for an artifact key: backend keys are root-relative with no
/// traversal. Applied before the DB membership query.
