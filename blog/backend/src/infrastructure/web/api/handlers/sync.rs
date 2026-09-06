use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    body::{Body, Bytes},
    extract::{Extension, Path, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use chrono::{Duration, Utc};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

use crate::{
    domain::{
        entities::secret::Claims,
        errors::sync::SyncError,
    },
    infrastructure::{
        sync::{
            artifact_key_exists, build_manifest, canonical_media_url, generate_sync_key,
            hash_sync_key,
        },
        web::{
            api::middlewares::auth::SyncKeyAuth,
            server::{AppState, DatabaseSource},
        },
    },
};

pub const MAX_SYNC_KEY_TTL_HOURS: u64 = 168;

// ── Admin: key management (JWT, admin only) ──────────────────────────────────

#[derive(Deserialize)]
pub struct CreateSyncKeyRequest {
    pub label: Option<String>,
    /// Validity window in hours. Clamped to 1..=168 (one week).
    pub ttl_hours: Option<u64>,
}

#[derive(Serialize)]
pub struct CreatedSyncKey {
    pub id: i64,
    pub label: String,
    pub expires_at: String,
    /// The full `bsk_…` secret. Returned exactly once and never stored.
    pub key: String,
}

pub async fn create_sync_key(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<CreateSyncKeyRequest>,
) -> Result<Json<CreatedSyncKey>, SyncError> {
    let created_by: i64 = claims
        .user_id
        .parse()
        .map_err(|_| SyncError::InternalError("Cannot parse id".to_string()))?;
    let ttl = request.ttl_hours.unwrap_or(24).clamp(1, MAX_SYNC_KEY_TTL_HOURS);
    let label = request.label.unwrap_or_default().trim().to_string();
    let label = if label.len() > 100 {
        label[..100].to_string()
    } else {
        label
    };

    let key = generate_sync_key().map_err(SyncError::InternalError)?;
    let expires_at = (Utc::now() + Duration::hours(ttl as i64)).to_rfc3339();

    let result = sqlx::query(
        "INSERT INTO sync_keys (token_hash, label, mode, created_by, expires_at) VALUES (?, ?, 'pull', ?, ?)",
    )
    .bind(hash_sync_key(&key))
    .bind(&label)
    .bind(created_by)
    .bind(&expires_at)
    .execute(&state.project_service.pool)
    .await?;

    tracing::info!(key_id = result.last_insert_rowid(), "sync key issued");
    Ok(Json(CreatedSyncKey {
        id: result.last_insert_rowid(),
        label,
        expires_at,
        key,
    }))
}

#[derive(Serialize)]
pub struct SyncKeyInfo {
    pub id: i64,
    pub label: String,
    pub mode: String,
    pub created_at: String,
    pub expires_at: String,
    pub revoked_at: Option<String>,
    pub last_used_at: Option<String>,
}

pub async fn list_sync_keys(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<SyncKeyInfo>>, SyncError> {
    let rows = sqlx::query(
        "SELECT id, label, mode, created_at, expires_at, revoked_at, last_used_at
         FROM sync_keys ORDER BY id DESC",
    )
    .fetch_all(&state.project_service.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|row| SyncKeyInfo {
                id: row.get("id"),
                label: row.get("label"),
                mode: row.get("mode"),
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
                revoked_at: row.get("revoked_at"),
                last_used_at: row.get("last_used_at"),
            })
            .collect(),
    ))
}

pub async fn revoke_sync_key(
    State(state): State<Arc<AppState>>,
    Path(key_id): Path<i64>,
) -> Result<StatusCode, SyncError> {
    let result = sqlx::query("UPDATE sync_keys SET revoked_at = CURRENT_TIMESTAMP WHERE id = ? AND revoked_at IS NULL")
        .bind(key_id)
        .execute(&state.project_service.pool)
        .await?;
    if result.rows_affected() == 0 {
        let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM sync_keys WHERE id = ?")
            .bind(key_id)
            .fetch_optional(&state.project_service.pool)
            .await?;
        return match exists {
            Some(_) => Ok(StatusCode::NO_CONTENT), // already revoked
            None => Err(SyncError::NotFound),
        };
    }
    tracing::info!(key_id, "sync key revoked");
    Ok(StatusCode::NO_CONTENT)
}

// ── Key-authenticated pull endpoints ─────────────────────────────────────────

pub async fn get_manifest(
    State(state): State<Arc<AppState>>,
    Extension(_auth): Extension<SyncKeyAuth>,
) -> Result<Json<crate::infrastructure::sync::SyncManifest>, SyncError> {
    let db_path = match &state.config.database_source {
        DatabaseSource::Sqlite { path } => path.clone(),
    };
    let manifest = build_manifest(
        &state.project_service.pool,
        &state.media_config.dir,
        &state.project_demo_config.dir,
        &state.storage,
        &db_path,
    )
    .await
    .map_err(SyncError::InternalError)?;
    Ok(Json(manifest))
}

/// Streams a consistent, compacted snapshot of the SQLite database. VACUUM
/// INTO captures WAL content too, unlike a raw file copy.
pub async fn get_database(
    State(state): State<Arc<AppState>>,
    Extension(_auth): Extension<SyncKeyAuth>,
) -> Result<Response, SyncError> {
    let db_path = match &state.config.database_source {
        DatabaseSource::Sqlite { path } => path.clone(),
    };
    let temp_path = std::env::temp_dir().join(format!("sync-db-{}-{}.db", Utc::now().timestamp(), Uuid::new_v4()));
    let escaped = temp_path.to_string_lossy().replace('\'', "''");
    sqlx::query(&format!("VACUUM INTO '{escaped}'"))
        .execute(&state.project_service.pool)
        .await
        .map_err(|e| SyncError::InternalError(format!("database snapshot failed: {e}")))?;

    stream_temp_file(temp_path, &db_path, "application/octet-stream").await
}

pub async fn get_media_by_hash(
    State(state): State<Arc<AppState>>,
    Extension(_auth): Extension<SyncKeyAuth>,
    Path(hash): Path<String>,
) -> Result<Response, SyncError> {
    let row = sqlx::query(
        "SELECT hash, file_type, COALESCE(uploader_id, 0) AS uploader_id FROM media WHERE hash = ?",
    )
    .bind(&hash)
    .fetch_optional(&state.project_service.pool)
    .await?
    .ok_or(SyncError::NotFound)?;
    let file_type: String = row.get("file_type");
    let uploader_id: i64 = row.get("uploader_id");
    let canonical = canonical_media_url(&hash, &file_type, uploader_id, &state.media_config.dir)
        .ok_or(SyncError::NotFound)?;
    let file = tokio::fs::File::open(PathBuf::from(&canonical))
        .await
        .map_err(|_| SyncError::NotFound)?;
    let size = file.metadata().await?.len();
    Ok(streamed_response(
        Body::from_stream(ReaderStream::new(file)),
        size,
        "application/octet-stream",
    ))
}

/// Serves one file of an extracted demo (project or game). The id must exist
/// in the database and the path may not escape the demo directory.
pub async fn get_demo_file(
    State(state): State<Arc<AppState>>,
    Extension(_auth): Extension<SyncKeyAuth>,
    Path((kind, id, relative)): Path<(String, i64, String)>,
) -> Result<Response, SyncError> {
    let dir_name = match kind.as_str() {
        "project" => {
            let exists: Option<i64> =
                sqlx::query_scalar("SELECT id FROM projects WHERE id = ?")
                    .bind(id)
                    .fetch_optional(&state.project_service.pool)
                    .await?;
            exists.ok_or(SyncError::NotFound)?;
            id.to_string()
        }
        "game" => {
            let exists: Option<i64> =
                sqlx::query_scalar("SELECT id FROM games WHERE id = ?")
                    .bind(id)
                    .fetch_optional(&state.project_service.pool)
                    .await?;
            exists.ok_or(SyncError::NotFound)?;
            format!("game-{id}")
        }
        _ => return Err(SyncError::InvalidData("kind must be project or game".to_string())),
    };
    if !is_safe_relative_path(&relative) {
        return Err(SyncError::NotFound);
    }
    let path = state
        .project_demo_config
        .dir
        .join(dir_name)
        .join(&relative);
    let file = tokio::fs::File::open(&path).await.map_err(|_| SyncError::NotFound)?;
    let size = file.metadata().await?.len();
    Ok(streamed_response(
        Body::from_stream(ReaderStream::new(file)),
        size,
        "application/octet-stream",
    ))
}

/// Serves one artifact (js-dos bundle, v86 system chunk, game disk part, ISO,
/// snapshot or save) from the source environment's object store — R2 or fs,
/// whichever the backend runs on. The key must be recorded in the database.
pub async fn get_artifact(
    State(state): State<Arc<AppState>>,
    Extension(_auth): Extension<SyncKeyAuth>,
    Path(key): Path<String>,
) -> Result<Response, SyncError> {
    if !artifact_key_exists(&state.project_service.pool, &key)
        .await
        .map_err(SyncError::InternalError)?
    {
        return Err(SyncError::NotFound);
    }
    let size = state
        .storage
        .object_size(&key)
        .await
        .map_err(|e| SyncError::InternalError(e.to_string()))?
        .ok_or(SyncError::NotFound)?;
    let reader = state
        .storage
        .get_object_reader(&key)
        .await
        .map_err(|e| SyncError::InternalError(e.to_string()))?;
    Ok(streamed_response(
        Body::from_stream(ReaderStream::new(reader)),
        size,
        "application/octet-stream",
    ))
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Rejects traversal, absolute paths, backslashes and hidden/transient files.
fn is_safe_relative_path(relative: &str) -> bool {
    !relative.is_empty()
        && !relative.starts_with('/')
        && !relative.contains('\\')
        && relative
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != ".." && !segment.starts_with('.'))
}

fn streamed_response(body: Body, size: u64, content_type: &'static str) -> Response {
    let mut response = Response::new(body);
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, header::HeaderValue::from_static(content_type));
    if let Ok(value) = header::HeaderValue::from_str(&size.to_string()) {
        response.headers_mut().insert(header::CONTENT_LENGTH, value);
    }
    response
}

/// Streams a temp file, then deletes it whether the download completed or the
/// client disconnected (same cleanup pattern the old backup download used).
async fn stream_temp_file(
    temp_path: PathBuf,
    source_db_path: &std::path::Path,
    content_type: &'static str,
) -> Result<Response, SyncError> {
    let file = tokio::fs::File::open(&temp_path).await?;
    let size = file.metadata().await?.len();
    let file_name = source_db_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("blog.db")
        .to_string();

    let cleanup_path = temp_path.clone();
    let cleanup = futures::stream::once(async move {
        let _ = tokio::fs::remove_file(&cleanup_path).await;
        Ok::<Bytes, std::io::Error>(Bytes::new())
    });
    let stream = ReaderStream::with_capacity(file, 64 * 1024).chain(cleanup);

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, size)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{file_name}\""),
        )
        .body(Body::from_stream(stream))
        .unwrap())
}
