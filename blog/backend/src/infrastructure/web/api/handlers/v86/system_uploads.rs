// System base-image upload sessions: start, per-part PUT, abort, and the
// in-memory chunk-progress registry the status endpoints read.
use std::{collections::HashMap, sync::{Arc, Mutex, OnceLock}};

use axum::{
    Extension, Json,
    body::Bytes,
    extract::{Path as AxumPath, State},
    http::StatusCode,
};
use chrono::{Duration, Utc};
use sqlx::Row;
use uuid::Uuid;

use crate::domain::{entities::secret::Claims, errors::project::ProjectError};
use crate::infrastructure::web::server::AppState;

use super::dto::{
    ChunkProgress, ServerStatusResponse, StartSystemUploadRequest, StartSystemUploadResponse,
    UploadStatusResponse,
};
use super::shared::{ensure_upload_not_expired, storage_error, user_id};
use super::systems::validate_memory_size_mb;

pub async fn start_system_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<StartSystemUploadRequest>,
) -> Result<Json<StartSystemUploadResponse>, ProjectError> {
    let uploader_id = user_id(&claims)?;
    // windows9x covers 95/98/ME; windowsxp boots too (needs memory_size_mb
    // >= 256 to be usable). The player config does not branch on the key.
    if !matches!(request.platform_key.as_str(), "windows9x" | "windowsxp") {
        return Err(ProjectError::InvalidDemo(
            "Only the windows9x and windowsxp v86 platforms are currently supported.".to_string(),
        ));
    }
    if let Some(memory_size_mb) = request.memory_size_mb {
        validate_memory_size_mb(memory_size_mb)?;
    }
    // .raw and .img are both flat disk dumps — identical to the emulator;
    // container formats (VHD/VDI/...) are not supported.
    if !matches!(request.file_name.to_ascii_lowercase().rsplit('.').next(), Some("img" | "raw")) {
        return Err(ProjectError::InvalidDemo(
            "Expected an .img or .raw disk image.".to_string(),
        ));
    }
    let name = request.name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err(ProjectError::InvalidDemo(
            "A system name between 1 and 100 characters is required.".to_string(),
        ));
    }
    if request.size_bytes == 0 || request.size_bytes > state.project_demo_config.max_v86_base_size {
        return Err(ProjectError::InvalidDemo(
            "The base IMG exceeds the configured limit.".to_string(),
        ));
    }
    let existing: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM v86_systems WHERE name = ? AND id != COALESCE(?, 0)",
    )
    .bind(name)
    .bind(request.system_id)
    .fetch_one(&state.project_service.pool)
    .await?;
    if existing > 0 {
        return Err(ProjectError::InvalidDemo(
            "A system with this name already exists.".to_string(),
        ));
    }

    let chunk_size = state.project_demo_config.v86_download_chunk_size;
    let chunk_count = request.size_bytes.div_ceil(chunk_size);
    let storage_key = format!("v86/assets/systems/{}", request.sha256);

    // Content-addressed dedup: if a version with this exact sha already exists,
    // skip the upload entirely.
    let existing_version: Option<String> = sqlx::query_scalar(
        "SELECT storage_key FROM v86_system_versions WHERE sha256 = ? LIMIT 1",
    )
    .bind(&request.sha256)
    .fetch_optional(&state.project_service.pool)
    .await?;

    if let Some(existing_key) = existing_version {
        let upload_id = Uuid::new_v4().to_string();
        let expires_at =
            Utc::now() + Duration::hours(state.project_demo_config.upload_session_ttl_hours as i64);
        sqlx::query(
            r#"INSERT INTO v86_system_upload_sessions
               (id, uploader_id, system_id, name, platform_key, expected_current_version,
                original_file_name, expected_size_bytes, staged_storage_key, staged_sha256,
                staged_chunk_count, memory_size_mb, reuse, status, expires_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, 'active', ?)"#,
        )
        .bind(&upload_id)
        .bind(uploader_id)
        .bind(request.system_id)
        .bind(name)
        .bind(&request.platform_key)
        .bind(request.expected_current_version.unwrap_or(0))
        .bind(&request.file_name)
        .bind(request.size_bytes as i64)
        .bind(&existing_key)
        .bind(&request.sha256)
        .bind(chunk_count as i64)
        .bind(request.memory_size_mb)
        .bind(expires_at.to_rfc3339())
        .execute(&state.project_service.pool)
        .await?;
        return Ok(Json(StartSystemUploadResponse {
            upload_id,
            reuse: true,
            chunk_size_bytes: chunk_size,
            chunk_count,
            storage_key: Some(existing_key),
        }));
    }

    let upload_id = Uuid::new_v4().to_string();
    let expires_at =
        Utc::now() + Duration::hours(state.project_demo_config.upload_session_ttl_hours as i64);
    sqlx::query(
        r#"INSERT INTO v86_system_upload_sessions
           (id, uploader_id, system_id, name, platform_key, expected_current_version,
            original_file_name, expected_size_bytes, staged_storage_key, staged_sha256,
            staged_chunk_count, memory_size_mb, reuse, expires_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?)"#,
    )
    .bind(&upload_id)
    .bind(uploader_id)
    .bind(request.system_id)
    .bind(name)
    .bind(&request.platform_key)
    .bind(request.expected_current_version.unwrap_or(0))
    .bind(&request.file_name)
    .bind(request.size_bytes as i64)
    .bind(&storage_key)
    .bind(&request.sha256)
    .bind(chunk_count as i64)
    .bind(request.memory_size_mb)
    .bind(expires_at.to_rfc3339())
    .execute(&state.project_service.pool)
    .await?;
    Ok(Json(StartSystemUploadResponse {
        upload_id,
        reuse: false,
        chunk_size_bytes: chunk_size,
        chunk_count,
        storage_key: None,
    }))
}
pub async fn upload_system_part(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath((upload_id, part_index)): AxumPath<(String, u64)>,
    bytes: Bytes,
) -> Result<StatusCode, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let row = sqlx::query(
        r#"SELECT status, expected_size_bytes, staged_storage_key,
                  staged_chunk_count, reuse, expires_at
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
    if row.get::<i64, _>("reuse") != 0 {
        return Err(ProjectError::Conflict(
            "The base image already exists; no upload is expected.".to_string(),
        ));
    }
    let chunk_count: i64 = row.get("staged_chunk_count");
    if part_index >= chunk_count as u64 {
        return Err(ProjectError::InvalidDemo(
            "Part index exceeds the expected chunk count.".to_string(),
        ));
    }

    let storage_key: String = row.get("staged_storage_key");
    let chunk_size: u64 = state.project_demo_config.v86_download_chunk_size;
    let offset = part_index * chunk_size;
    let end = (offset + chunk_size).min(row.get::<i64, _>("expected_size_bytes") as u64);
    let part_name = format!("{storage_key}/{offset}-{end}.img.zst");

    let storage = &state.storage;
    storage
        .put_object_bytes(&part_name, bytes.to_vec())
        .await
        .map_err(storage_error)?;

    // Record the part atomically. INSERT is concurrency-safe, unlike the old
    // read-modify-write of a received_parts JSON column.
    let changed = sqlx::query(
        "INSERT INTO v86_system_upload_parts (upload_id, part_index) VALUES (?, ?)",
    )
    .bind(&upload_id)
    .bind(part_index as i64)
    .execute(&state.project_service.pool)
    .await;
    match changed {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(sqlx::Error::Database(db_err))
            if db_err.is_unique_violation() =>
        {
            Err(ProjectError::Conflict(
                "This part was already uploaded.".to_string(),
            ))
        }
        Err(e) => Err(e.into()),
    }
}
pub async fn abort_system_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(upload_id): AxumPath<String>,
) -> Result<StatusCode, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let row = sqlx::query(
        "SELECT staged_storage_key, reuse, status FROM v86_system_upload_sessions WHERE id = ? AND uploader_id = ?",
    )
    .bind(&upload_id)
    .bind(uploader_id)
    .fetch_optional(&state.project_service.pool)
    .await?
    .ok_or(ProjectError::ProjectNotFound)?;
    if row.get::<String, _>("status") == "consumed" {
        return Err(ProjectError::Conflict(
            "A consumed upload cannot be aborted.".to_string(),
        ));
    }
    sqlx::query(
        "UPDATE v86_system_upload_sessions SET status = 'aborted', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(&upload_id)
    .execute(&state.project_service.pool)
    .await?;
    let reuse: i64 = row.get("reuse");
    if reuse == 0 {
        if let Some(key) = row.get::<Option<String>, _>("staged_storage_key") {
            let _ = state.storage.delete_prefix(&key).await;
        }
    }
    Ok(StatusCode::NO_CONTENT)
}
pub(super) fn chunk_progress_map() -> &'static Mutex<HashMap<String, ChunkProgress>> {
    static MAP: OnceLock<Mutex<HashMap<String, ChunkProgress>>> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

pub async fn get_system_upload_status(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(upload_id): AxumPath<String>,
) -> Result<Json<UploadStatusResponse>, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let row = sqlx::query(
        "SELECT status, error_message FROM v86_system_upload_sessions WHERE id = ? AND uploader_id = ?",
    )
    .bind(&upload_id)
    .bind(uploader_id)
    .fetch_optional(&state.project_service.pool)
    .await?
    .ok_or(ProjectError::ProjectNotFound)?;
    let progress = chunk_progress_map().lock().unwrap().get(&upload_id).cloned();
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

pub async fn get_server_status(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<ServerStatusResponse>, ProjectError> {
    let active_uploads = chunk_progress_map()
        .lock()
        .unwrap()
        .values()
        .cloned()
        .collect();
    Ok(Json(ServerStatusResponse {
        ok: true,
        active_uploads,
    }))
}

