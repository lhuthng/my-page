// Finalising, consuming, and aborting game build upload sessions, including
// the transactional attach into `game_v86_games` performed by the game writer.
use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path as AxumPath, State},
    http::StatusCode,
};
use sqlx::{Row, Sqlite, Transaction};

use crate::domain::{entities::secret::Claims, errors::project::ProjectError};
use crate::infrastructure::{storage::ObjectStore, web::server::AppState};

use super::dto::UploadStatusResponse;
use super::shared::{ensure_upload_not_expired, user_id};
use super::system_uploads::chunk_progress_map;

/// The stored artifact of a project, used to resolve the manifest-only fast
/// path (reuse the stored disk) and to validate the expected revision.
pub(super) struct StoredGameArtifact {
    pub(super) artifact_revision: i64,
    pub(super) disk_sha256: Option<String>,
    pub(super) disk_size_bytes: Option<i64>,
    pub(super) chunk_count: i64,
}

pub(super) async fn fetch_stored_game_artifact(
    pool: &sqlx::SqlitePool,
    game_id: i64,
) -> Result<Option<StoredGameArtifact>, String> {
    let row = sqlx::query(
        r#"SELECT artifact_revision, disk_sha256, disk_size_bytes, chunk_count
           FROM game_v86_games WHERE game_id = ?"#,
    )
    .bind(game_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(row.map(|row| StoredGameArtifact {
        artifact_revision: row.get("artifact_revision"),
        disk_sha256: row.get("disk_sha256"),
        disk_size_bytes: row.get("disk_size_bytes"),
        chunk_count: row.get("chunk_count"),
    }))
}

/// Deletes the content-addressed artifacts this session uploaded, but never
/// shared/reused objects: the parts live under the disk sha prefix only when
/// the client actually uploaded them, and reused variant CDs are skipped.
async fn delete_uploaded_game_artifacts(
    storage: &ObjectStore,
    pool: &sqlx::SqlitePool,
    upload_id: &str,
    disk_storage_key: Option<&str>,
    disk_reuse: bool,
) {
    if !disk_reuse && let Some(key) = disk_storage_key {
        let _ = storage.delete_prefix(key).await;
    }
    let uploaded_isos: Vec<String> = sqlx::query_scalar(
        "SELECT iso_storage_key FROM project_v86_staged_variants WHERE upload_id = ? AND reuse = 0",
    )
    .bind(upload_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    for key in uploaded_isos {
        let _ = storage.delete_object(&format!("{key}/full.iso")).await;
    }
}

pub async fn complete_game_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(upload_id): AxumPath<String>,
) -> Result<StatusCode, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let row = sqlx::query(
        r#"SELECT staged_disk_chunk_count, disk_reuse, status, expires_at
           FROM project_v86_upload_sessions WHERE id = ? AND uploader_id = ?"#,
    )
    .bind(&upload_id)
    .bind(uploader_id)
    .fetch_optional(&state.project_service.pool)
    .await?
    .ok_or(ProjectError::ProjectNotFound)?;
    ensure_upload_not_expired(row.get::<String, _>("expires_at").as_str())?;
    if row.get::<String, _>("status") != "active" {
        return Err(ProjectError::InvalidDemo(
            "The v86 game upload is no longer active.".to_string(),
        ));
    }

    // Every non-reused artifact must have arrived before the session can be
    // finalized. The client already built, hashed, and self-checked the disk
    // and CD images against this plan (see upload-controller.js), so the
    // server only records arrival — no decompression or re-hashing. Parts are
    // tracked atomically, and a part's R2 object is written before its
    // tracking row, so a complete session has all its content in R2.
    let disk_reuse: i64 = row.get("disk_reuse");
    if disk_reuse == 0 {
        let chunk_count: i64 = row.get("staged_disk_chunk_count");
        let received: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM project_v86_received_disk_parts WHERE upload_id = ?",
        )
        .bind(&upload_id)
        .fetch_one(&state.project_service.pool)
        .await?;
        if received != chunk_count {
            return Err(ProjectError::InvalidDemo(
                "The game disk upload is incomplete.".to_string(),
            ));
        }
    }
    let missing_isos: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM project_v86_staged_variants
           WHERE upload_id = ? AND reuse = 0 AND received = 0"#,
    )
    .bind(&upload_id)
    .fetch_one(&state.project_service.pool)
    .await?;
    if missing_isos > 0 {
        return Err(ProjectError::InvalidDemo(
            "The v86 game upload is incomplete.".to_string(),
        ));
    }

    sqlx::query(
        "UPDATE project_v86_upload_sessions SET status = 'ready', updated_at = CURRENT_TIMESTAMP WHERE id = ? AND status = 'active'",
    )
    .bind(&upload_id)
    .execute(&state.project_service.pool)
    .await?;
    Ok(StatusCode::OK)
}

pub async fn attach_ready_game_tx(
    tx: &mut Transaction<'_, Sqlite>,
    game_id: i64,
    uploader_id: i64,
    upload_id: &str,
    chunk_size: u64,
) -> Result<i64, ProjectError> {
    let row = sqlx::query(
        r#"SELECT source_project_id, system_version_id, expected_artifact_revision,
                  manifest_text, manifest_sha256,
                  staged_disk_storage_key, staged_disk_sha256, staged_disk_size_bytes,
                  staged_disk_chunk_count,
                  staged_iso_storage_key, staged_iso_sha256, staged_iso_size_bytes,
                  status, expires_at
           FROM project_v86_upload_sessions
           WHERE id = ? AND uploader_id = ?"#,
    )
    .bind(upload_id)
    .bind(uploader_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(ProjectError::ProjectNotFound)?;
    ensure_upload_not_expired(row.get::<String, _>("expires_at").as_str())?;
    if row.get::<String, _>("status") != "ready" {
        return Err(ProjectError::InvalidDemo(
            "The v86 game package is not ready.".to_string(),
        ));
    }
    let source_project: Option<i64> = row.get("source_project_id");
    if source_project.is_some() && source_project != Some(game_id) {
        return Err(ProjectError::Forbidden);
    }
    let expected: i64 = row.get("expected_artifact_revision");
    let current: i64 = sqlx::query_scalar(
        "SELECT COALESCE((SELECT artifact_revision FROM game_v86_games WHERE game_id = ?), 0)",
    )
    .bind(game_id)
    .fetch_one(&mut **tx)
    .await?;
    if current != expected {
        return Err(ProjectError::Conflict(
            "The v86 artifact changed while this package was building.".to_string(),
        ));
    }
    let revision = current + 1;
    let artifact_change = sqlx::query(
        r#"INSERT INTO game_v86_games
           (game_id, system_version_id, manifest_text, manifest_sha256,
            launcher_config_sha256, game_config_sha256,
            disk_storage_key, disk_size_bytes, disk_sha256,
            iso_storage_key, iso_size_bytes, iso_sha256, chunk_size_bytes,
            chunk_count, artifact_revision)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
           ON CONFLICT(game_id) DO UPDATE SET
             system_version_id = excluded.system_version_id,
             manifest_text = excluded.manifest_text,
             manifest_sha256 = excluded.manifest_sha256,
             launcher_config_sha256 = excluded.launcher_config_sha256,
             game_config_sha256 = excluded.game_config_sha256,
             disk_storage_key = excluded.disk_storage_key,
             disk_size_bytes = excluded.disk_size_bytes,
             disk_sha256 = excluded.disk_sha256,
             iso_storage_key = excluded.iso_storage_key,
             iso_size_bytes = excluded.iso_size_bytes,
             iso_sha256 = excluded.iso_sha256,
             chunk_size_bytes = excluded.chunk_size_bytes,
             chunk_count = excluded.chunk_count,
             artifact_revision = excluded.artifact_revision,
             updated_at = CURRENT_TIMESTAMP
           WHERE game_v86_games.artifact_revision = ?"#,
    )
    .bind(game_id)
    .bind(row.get::<i64, _>("system_version_id"))
    .bind(row.get::<String, _>("manifest_text"))
    .bind(row.get::<String, _>("manifest_sha256"))
    .bind(row.get::<String, _>("manifest_sha256"))
    .bind(row.get::<String, _>("manifest_sha256"))
    .bind(row.get::<Option<String>, _>("staged_disk_storage_key"))
    .bind(row.get::<Option<i64>, _>("staged_disk_size_bytes"))
    .bind(row.get::<Option<String>, _>("staged_disk_sha256"))
    .bind(row.get::<String, _>("staged_iso_storage_key"))
    .bind(row.get::<i64, _>("staged_iso_size_bytes"))
    .bind(row.get::<String, _>("staged_iso_sha256"))
    .bind(chunk_size as i64)
    .bind(row.get::<i64, _>("staged_disk_chunk_count"))
    .bind(revision)
    .bind(expected)
    .execute(&mut **tx)
    .await?;
    if artifact_change.rows_affected() != 1 {
        return Err(ProjectError::Conflict(
            "The v86 artifact changed while this package was building.".to_string(),
        ));
    }
    // Replace the game's variant CDs with the newly staged set. Variant 1
    // mirrors the iso_* columns on game_v86_games (kept for compatibility).
    sqlx::query("DELETE FROM game_v86_variants WHERE game_id = ?")
        .bind(game_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query(
        r#"INSERT INTO game_v86_variants
           (game_id, variant_index, name, exe, args, iso_storage_key, iso_size_bytes, iso_sha256)
           SELECT ?, variant_index, name, exe, args, iso_storage_key, iso_size_bytes, iso_sha256
           FROM project_v86_staged_variants WHERE upload_id = ?"#,
    )
    .bind(game_id)
    .bind(upload_id)
    .execute(&mut **tx)
    .await?;
    let changed = sqlx::query(
        "UPDATE project_v86_upload_sessions SET status = 'consumed', updated_at = CURRENT_TIMESTAMP WHERE id = ? AND status = 'ready'",
    )
    .bind(upload_id)
    .execute(&mut **tx)
    .await?;
    if changed.rows_affected() != 1 {
        return Err(ProjectError::Conflict(
            "The staged v86 package was already consumed.".to_string(),
        ));
    }
    Ok(revision)
}

pub async fn abort_game_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(upload_id): AxumPath<String>,
) -> Result<StatusCode, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let row = sqlx::query(
        "SELECT staged_disk_storage_key, disk_reuse, status FROM project_v86_upload_sessions WHERE id = ? AND uploader_id = ?",
    )
    .bind(&upload_id)
    .bind(uploader_id)
    .fetch_optional(&state.project_service.pool)
    .await?
    .ok_or(ProjectError::ProjectNotFound)?;
    let status: String = row.get("status");
    if status == "consumed" {
        return Err(ProjectError::Conflict(
            "A consumed upload cannot be aborted.".to_string(),
        ));
    }
    sqlx::query(
        "UPDATE project_v86_upload_sessions SET status = 'aborted', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(&upload_id)
    .execute(&state.project_service.pool)
    .await?;
    // Remove only the content this session uploaded. Shared/reused artifacts
    // are left untouched.
    let disk_key: Option<String> = row.get("staged_disk_storage_key");
    let disk_reuse: i64 = row.get("disk_reuse");
    delete_uploaded_game_artifacts(
        &state.storage,
        &state.project_service.pool,
        &upload_id,
        disk_key.as_deref(),
        disk_reuse != 0,
    )
    .await;
    chunk_progress_map().lock().unwrap().remove(&upload_id);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_game_upload_status(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(upload_id): AxumPath<String>,
) -> Result<Json<UploadStatusResponse>, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let row = sqlx::query(
        "SELECT status, error_message FROM project_v86_upload_sessions WHERE id = ? AND uploader_id = ?",
    )
    .bind(&upload_id)
    .bind(uploader_id)
    .fetch_optional(&state.project_service.pool)
    .await?
    .ok_or(ProjectError::ProjectNotFound)?;
    let progress = chunk_progress_map()
        .lock()
        .unwrap()
        .get(&upload_id)
        .cloned();
    let active_uploads = chunk_progress_map()
        .lock()
        .unwrap()
        .values()
        .cloned()
        .collect();
    Ok(Json(UploadStatusResponse {
        status: row.get("status"),
        error_message: row.get("error_message"),
        chunk_progress: progress,
        active_uploads,
    }))
}
