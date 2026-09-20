// Post-import database fix pass: rewrites stored URLs and paths to the
// target environment's layout.
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use sqlx::Row;

use crate::domain::entities::media::MediaType;
use crate::infrastructure::storage::ObjectStore;

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

