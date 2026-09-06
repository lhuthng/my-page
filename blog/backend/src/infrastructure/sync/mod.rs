//! Prod -> dev sync support: manifest generation, sync-key handling, and the
//! post-import database fix pass. Shared between the `/sync` API (prod side)
//! and the `sync-pull` binary (dev side).
//!
//! Direction policy: this flow is deliberately pull-only. The `sync_keys`
//! migration constrains `mode` to `'pull'`, and nothing here can write to the
//! source environment — a future push flow needs its own mode, key scoping and
//! confirmation steps.

use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::str::FromStr;

use crate::domain::entities::media::MediaType;
use crate::infrastructure::storage::ObjectStore;

/// Prefix of a sync key secret. The rest is 32 random bytes hex.
pub const SYNC_KEY_PREFIX: &str = "bsk_";

// ── Keys ─────────────────────────────────────────────────────────────────────

/// Generates a new `bsk_<64 hex>` secret. Only its SHA-256 hash is persisted.
pub fn generate_sync_key() -> Result<String, String> {
    let bytes = {
        use rand::RngCore;
        let mut buf = [0u8; 32];
        rand::rng().fill_bytes(&mut buf);
        buf
    };
    Ok(format!("{SYNC_KEY_PREFIX}{}", hex::encode(bytes)))
}

pub fn hash_sync_key(secret: &str) -> String {
    hex::encode(Sha256::digest(secret.as_bytes()))
}

// ── Manifest ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
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
fn walk_demo_dir(base: &Path) -> Result<Vec<DemoFile>, std::io::Error> {
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
pub fn is_valid_artifact_key_shape(key: &str) -> bool {
    !key.is_empty()
        && !key.starts_with('/')
        && !key.contains('\\')
        && key
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

/// Validates that a requested artifact key belongs to one of the storage keys
/// recorded in the database (the key itself or a child object of it), so the
/// `/sync/artifact` endpoint can never serve arbitrary paths.
pub async fn artifact_key_exists(pool: &sqlx::SqlitePool, key: &str) -> Result<bool, String> {
    if !is_valid_artifact_key_shape(key) {
        return Ok(false);
    }
    let hits: (i64,) = sqlx::query_as(
        r#"SELECT
             (SELECT EXISTS(SELECT 1 FROM v86_system_versions
                WHERE ?1 = storage_key OR ?1 LIKE storage_key || '/%'))
           + (SELECT EXISTS(SELECT 1 FROM game_v86_games
                WHERE ?1 = zip_storage_key OR ?1 = iso_storage_key
                   OR ?1 LIKE iso_storage_key || '/%'
                   OR (disk_storage_key IS NOT NULL AND ?1 LIKE disk_storage_key || '/%')))
           + (SELECT EXISTS(SELECT 1 FROM game_v86_variants
                WHERE ?1 = iso_storage_key OR ?1 LIKE iso_storage_key || '/%'))
           + (SELECT EXISTS(SELECT 1 FROM game_v86_snapshots WHERE ?1 = storage_key))
           + (SELECT EXISTS(SELECT 1 FROM game_v86_saves WHERE ?1 = storage_key))
           + (SELECT EXISTS(SELECT 1 FROM game_jsdos_bundles WHERE ?1 = storage_key))"#,
    )
    .bind(key)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(hits.0 > 0)
}

// ── Database fix ─────────────────────────────────────────────────────────────

#[derive(Debug, Default, Serialize)]
pub struct FixSummary {
    pub media_urls_fixed: u64,
    pub project_demo_urls_fixed: u64,
    pub game_demo_urls_fixed: u64,
}

/// Reconstructs the canonical `media.url` value for a row, mirroring
/// `media_path_from_link`'s hash-first layout under the dev media root.
/// Returns None for rows whose file_type cannot be parsed (leave them alone).
pub fn canonical_media_url(
    hash: &str,
    file_type: &str,
    uploader_id: i64,
    media_dir: &Path,
) -> Option<String> {
    let extension = MediaType::from_str(file_type).ok()?.get_extension();
    let path = if hash.starts_with(".post.")
        || hash.starts_with(".avt.")
        || hash.starts_with(".srs.")
    {
        // ".<type>.<id>.<sha256>" — splitn(4, '.') yields ["", type, id, sha].
        // The id part is NOT the on-disk directory; the uploader id is.
        let mut parts = hash.splitn(4, '.');
        let _empty = parts.next()?;
        let type_dir = parts.next()?;
        let _id = parts.next()?;
        let sha = parts.next()?;
        media_dir
            .join(type_dir)
            .join(uploader_id.to_string())
            .join(format!("{sha}{extension}"))
    } else if hash.len() >= 4 && hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        media_dir
            .join(&hash[0..2])
            .join(&hash[2..4])
            .join(format!("{hash}{extension}"))
    } else {
        return None;
    };
    Some(path.to_string_lossy().to_string())
}

/// Rewrites path-valued columns of an imported prod database so they resolve
/// under the dev machine's MEDIA_PATH / PROJECT_DEMOS_PATH. Storage keys
/// (v86/jsdos) are backend-agnostic and stay untouched, as are https
/// embed/download demo URLs.
pub async fn fix_imported_database(
    pool: &sqlx::SqlitePool,
    media_dir: &Path,
    demos_dir: &Path,
) -> Result<FixSummary, String> {
    let mut summary = FixSummary::default();

    let rows: Vec<(i64, String, String, i64, String)> = sqlx::query_as(
        "SELECT id, COALESCE(hash, ''), file_type, COALESCE(uploader_id, 0), COALESCE(url, '') FROM media",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    for (id, hash, file_type, uploader_id, url) in rows {
        if hash.is_empty() {
            continue;
        }
        if let Some(canonical) = canonical_media_url(&hash, &file_type, uploader_id, media_dir)
            && canonical != url
        {
            sqlx::query("UPDATE media SET url = ? WHERE id = ?")
                .bind(&canonical)
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| e.to_string())?;
            summary.media_urls_fixed += 1;
        }
    }

    // html5/webgl zip demos live at {PROJECT_DEMOS_PATH}/{id}/index.html on
    // whatever machine extracted them; embed/download/video keep their https
    // URLs and jsdos/v86 keep NULL.
    let projects: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT id, demo_type, COALESCE(demo_url, '') FROM projects",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    for (id, demo_type, demo_url) in projects {
        if demo_type != "html5" && demo_type != "webgl" {
            continue;
        }
        let canonical = demos_dir.join(id.to_string()).join("index.html");
        let canonical = canonical.to_string_lossy().to_string();
        if demo_url.contains("://") || demo_url == canonical {
            continue;
        }
        sqlx::query("UPDATE projects SET demo_url = ? WHERE id = ?")
            .bind(&canonical)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        summary.project_demo_urls_fixed += 1;
    }

    let games: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT id, launcher_type, COALESCE(demo_url, '') FROM games",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    for (id, launcher_type, demo_url) in games {
        if launcher_type != "html5" && launcher_type != "webgl" {
            continue;
        }
        let canonical = demos_dir.join(format!("game-{id}")).join("index.html");
        let canonical = canonical.to_string_lossy().to_string();
        if demo_url.contains("://") || demo_url == canonical {
            continue;
        }
        sqlx::query("UPDATE games SET demo_url = ? WHERE id = ?")
            .bind(&canonical)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        summary.game_demo_urls_fixed += 1;
    }

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn sync_key_roundtrip() {
        let key = generate_sync_key().unwrap();
        assert!(key.starts_with(SYNC_KEY_PREFIX));
        let secret = key.trim_start_matches(SYNC_KEY_PREFIX);
        assert_eq!(secret.len(), 64);
        assert!(secret.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_eq!(hash_sync_key(&key).len(), 64);
        assert_ne!(generate_sync_key().unwrap(), key);
    }

    #[test]
    fn canonical_media_urls_match_serving_layout() {
        let root = PathBuf::from("./media");
        // Regular media: <root>/<h[0..2]>/<h[2..4]>/<h><ext>
        assert_eq!(
            canonical_media_url("abcdef1234", "image/png", 7, &root).unwrap(),
            "./media/ab/cd/abcdef1234.png"
        );
        // Post cover: directory uses the uploader id, not the post id.
        assert_eq!(
            canonical_media_url(".post.42.abcdef1234", "image/webp", 11, &root).unwrap(),
            "./media/post/11/abcdef1234.webp"
        );
        assert_eq!(
            canonical_media_url(".avt.3.abcdef1234", "image/jpeg", 3, &root).unwrap(),
            "./media/avt/3/abcdef1234.jpeg"
        );
        assert_eq!(
            canonical_media_url(".srs.9.abcdef1234", "video/mp4", 9, &root).unwrap(),
            "./media/srs/9/abcdef1234.mp4"
        );
        // Unparseable file types leave the row untouched.
        assert!(canonical_media_url("abcdef1234", "not/a-type", 1, &root).is_none());
        assert!(canonical_media_url("garbage with spaces", "image/png", 1, &root).is_none());
    }

    #[test]
    fn artifact_key_shape_check() {
        for key in ["", "/abs", "a/../b", "a//b", "a/./b", "a\\b", "."] {
            assert!(!is_valid_artifact_key_shape(key), "'{key}' must be rejected");
        }
        for key in [
            "v86/games/abc/full.iso",
            "v86/assets/systems/abc/0-262144.img.zst",
            "v86/snapshots/abc/state.zst",
            "v86/saves/1/2/save.zst",
            "jsdos/5/sha.jsdos",
        ] {
            assert!(is_valid_artifact_key_shape(key), "'{key}' must be accepted");
        }
    }
}
