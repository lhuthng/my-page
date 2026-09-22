// Promoting an uploaded system base image into a published version, and the
// deletion paths for versions and whole systems.
use std::sync::Arc;

use axum::{
    Extension,
    extract::{Path as AxumPath, State},
    http::StatusCode,
};
use sqlx::Row;

use crate::domain::{entities::secret::Claims, errors::project::ProjectError};
use crate::infrastructure::web::server::AppState;

use super::shared::{ensure_upload_not_expired, user_id};

pub async fn complete_system_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(upload_id): AxumPath<String>,
) -> Result<StatusCode, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let row = sqlx::query(
        r#"SELECT system_id, name, platform_key, expected_current_version, original_file_name,
                  expected_size_bytes, staged_storage_key, staged_sha256, staged_chunk_count,
                  memory_size_mb, reuse, status, expires_at
           FROM v86_system_upload_sessions WHERE id = ? AND uploader_id = ?"#,
    )
    .bind(&upload_id)
    .bind(uploader_id)
    .fetch_optional(&state.project_service.pool)
    .await?
    .ok_or(ProjectError::ProjectNotFound)?;
    ensure_upload_not_expired(row.get::<String, _>("expires_at").as_str())?;
    if row.get::<String, _>("status") != "active" {
        return Err(ProjectError::InvalidDemo(
            "The system upload is no longer active.".to_string(),
        ));
    }

    let reuse: i64 = row.get("reuse");
    if reuse == 0 {
        let chunk_count: i64 = row.get("staged_chunk_count");
        let received: Vec<i64> = sqlx::query_scalar(
            "SELECT part_index FROM v86_system_upload_parts WHERE upload_id = ?",
        )
        .bind(&upload_id)
        .fetch_all(&state.project_service.pool)
        .await?;
        if received.len() as i64 != chunk_count {
            return Err(ProjectError::InvalidDemo(
                "The base IMG upload is incomplete.".to_string(),
            ));
        }
        for index in 0..chunk_count {
            if !received.contains(&index) {
                return Err(ProjectError::InvalidDemo(
                    "The base IMG upload is incomplete.".to_string(),
                ));
            }
        }
    }

    let system_id_opt: Option<i64> = row.get("system_id");
    let expected_version: i64 = row.get("expected_current_version");
    let name: String = row.get("name");
    let platform_key: String = row.get("platform_key");
    let original_file_name: String = row.get("original_file_name");

    let existing: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM v86_systems WHERE name = ? AND id != COALESCE(?, 0)",
    )
    .bind(&name)
    .bind(system_id_opt)
    .fetch_one(&state.project_service.pool)
    .await?;
    if existing > 0 {
        return Err(ProjectError::InvalidDemo(
            "A system with this name already exists.".to_string(),
        ));
    }

    let (system_id, version_number) = {
        let mut tx = state.project_service.pool.begin().await?;
        let (sid, vn) = if let Some(sid) = system_id_opt {
            let updated = sqlx::query(
                "UPDATE v86_systems SET current_version = current_version + 1, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND current_version = ?",
            )
            .bind(sid)
            .bind(expected_version)
            .execute(&mut *tx)
            .await?;
            if updated.rows_affected() != 1 {
                return Err(ProjectError::Conflict(
                    "The system was replaced by another administrator.".to_string(),
                ));
            }
            (sid, expected_version + 1)
        } else {
            let result = sqlx::query(
                "INSERT INTO v86_systems (name, platform_key, memory_size_mb) VALUES (?, ?, COALESCE(?, 64))",
            )
            .bind(&name)
            .bind(&platform_key)
            .bind(row.try_get::<Option<i64>, _>("memory_size_mb").unwrap_or(None))
            .execute(&mut *tx)
            .await?;
            let new_id = result.last_insert_rowid();
            sqlx::query("UPDATE v86_systems SET current_version = 1 WHERE id = ?")
                .bind(new_id)
                .execute(&mut *tx)
                .await?;
            (new_id, 1)
        };
        tx.commit().await?;
        (sid, vn)
    };

    if reuse != 0 {
        // Dedup: image already exists; create version row and mark consumed immediately.
        let storage_key: String = row.get("staged_storage_key");
        let expected_size: i64 = row.get("expected_size_bytes");
        let sha256: String = row.get("staged_sha256");
        let chunk_count: i64 = row.get("staged_chunk_count");
        let chunk_size: i64 = state.project_demo_config.v86_download_chunk_size as i64;
        sqlx::query(
            r#"INSERT INTO v86_system_versions
               (system_id, version_number, original_file_name, storage_key, size_bytes,
                sha256, chunk_size_bytes, chunk_count)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(system_id)
        .bind(version_number)
        .bind(&original_file_name)
        .bind(&storage_key)
        .bind(expected_size)
        .bind(&sha256)
        .bind(chunk_size)
        .bind(chunk_count)
        .execute(&state.project_service.pool)
        .await
        .map_err(|e| ProjectError::InternalError(format!("Failed to create version: {e}")))?;
        sqlx::query(
            "UPDATE v86_system_upload_sessions SET status = 'consumed', system_id = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(system_id)
        .bind(&upload_id)
        .execute(&state.project_service.pool)
        .await?;
        return Ok(StatusCode::OK);
    }

    let chunk_count: i64 = row.get("staged_chunk_count");
    let storage_key: String = row.get("staged_storage_key");
    let sha256: String = row.get("staged_sha256");
    let expected_size: i64 = row.get("expected_size_bytes");
    let chunk_size: i64 = state.project_demo_config.v86_download_chunk_size as i64;

    // The client hashes the IMG and validates its boot sector before uploading,
    // so the server only records arrival — no decompression or re-hashing. The
    // parts are content-addressed under the sha the client reported.
    sqlx::query(
        r#"INSERT INTO v86_system_versions
           (system_id, version_number, original_file_name, storage_key, size_bytes,
            sha256, chunk_size_bytes, chunk_count)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(system_id)
    .bind(version_number)
    .bind(&original_file_name)
    .bind(&storage_key)
    .bind(expected_size)
    .bind(&sha256)
    .bind(chunk_size)
    .bind(chunk_count)
    .execute(&state.project_service.pool)
    .await
    .map_err(|e| ProjectError::InternalError(format!("Failed to create version: {e}")))?;
    sqlx::query(
        "UPDATE v86_system_upload_sessions SET status = 'consumed', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(&upload_id)
    .execute(&state.project_service.pool)
    .await?;
    Ok(StatusCode::OK)
}
pub async fn delete_system_version(
    State(state): State<Arc<AppState>>,
    AxumPath((system_id, version_id)): AxumPath<(i64, i64)>,
) -> Result<StatusCode, ProjectError> {
    let usage: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM game_v86_games WHERE system_version_id = ?")
            .bind(version_id)
            .fetch_one(&state.project_service.pool)
            .await?;
    if usage > 0 {
        return Err(ProjectError::Conflict(format!(
            "This system version is used by {usage} game(s) and cannot be deleted."
        )));
    }
    let row = sqlx::query(
        "SELECT v.storage_key, v.version_number, s.current_version FROM v86_system_versions v JOIN v86_systems s ON s.id = v.system_id WHERE v.id = ? AND v.system_id = ?",
    )
    .bind(version_id)
    .bind(system_id)
    .fetch_optional(&state.project_service.pool)
    .await?
    .ok_or(ProjectError::ProjectNotFound)?;
    if row.get::<i64, _>("version_number") == row.get::<i64, _>("current_version") {
        return Err(ProjectError::Conflict(
            "The current system version cannot be deleted; replace it first.".to_string(),
        ));
    }
    sqlx::query("DELETE FROM project_v86_upload_sessions WHERE system_version_id = ?")
        .bind(version_id)
        .execute(&state.project_service.pool)
        .await?;
    let storage_key: String = row.get("storage_key");
    sqlx::query("DELETE FROM v86_system_versions WHERE id = ?")
        .bind(version_id)
        .execute(&state.project_service.pool)
        .await?;
    // Content-addressed: only delete the prefix if no other version references it.
    let remaining: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM v86_system_versions WHERE storage_key = ?")
            .bind(&storage_key)
            .fetch_one(&state.project_service.pool)
            .await?;
    if remaining == 0 {
        let _ = state.storage.delete_prefix(&storage_key).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_system(
    State(state): State<Arc<AppState>>,
    AxumPath(system_id): AxumPath<i64>,
) -> Result<StatusCode, ProjectError> {
    let usage: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM game_v86_games g
           JOIN v86_system_versions v ON v.id = g.system_version_id
           WHERE v.system_id = ?"#,
    )
    .bind(system_id)
    .fetch_one(&state.project_service.pool)
    .await?;
    if usage > 0 {
        return Err(ProjectError::Conflict(format!(
            "This system is referenced by {usage} game(s); deactivate it instead."
        )));
    }
    let keys: Vec<String> =
        sqlx::query_scalar("SELECT storage_key FROM v86_system_versions WHERE system_id = ?")
            .bind(system_id)
            .fetch_all(&state.project_service.pool)
            .await?;
    sqlx::query(
        r#"DELETE FROM project_v86_upload_sessions
           WHERE system_version_id IN (
             SELECT id FROM v86_system_versions WHERE system_id = ?
           )"#,
    )
    .bind(system_id)
    .execute(&state.project_service.pool)
    .await?;
    let changed = sqlx::query("DELETE FROM v86_systems WHERE id = ?")
        .bind(system_id)
        .execute(&state.project_service.pool)
        .await?;
    if changed.rows_affected() != 1 {
        return Err(ProjectError::ProjectNotFound);
    }
    for key in &keys {
        let remaining: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM v86_system_versions WHERE storage_key = ?")
                .bind(key)
                .fetch_one(&state.project_service.pool)
                .await?;
        if remaining == 0 {
            let _ = state.storage.delete_prefix(key).await;
        }
    }
    Ok(StatusCode::NO_CONTENT)
}
