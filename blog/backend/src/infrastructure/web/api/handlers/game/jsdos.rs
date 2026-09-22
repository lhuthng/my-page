// The js-dos launcher: chunked bundle upload, validation, and public serving.
use std::{
    fs,
    io::{Read, Seek},
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use axum::{
    Extension, Json,
    body::Bytes,
    extract::{Path as AxumPath, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;
use uuid::Uuid;
use zip::ZipArchive;

use crate::{
    domain::{entities::secret::Claims, errors::game::GameError},
    infrastructure::web::{
        api::handlers::game::dto::{
            CompleteJsDosUploadResponse, JsDosUploadResponse, StartJsDosUploadRequest,
            StartJsDosUploadResponse,
        },
        api::support::ownership::require_owner,
        server::AppState,
    },
};

fn validate_jsdos_bundle(path: &Path, max_files: usize) -> Result<(u64, String), GameError> {
    let mut file = fs::File::open(path)?;
    let size = file.metadata()?.len();
    let mut has_manifest = false;
    let mut archive = ZipArchive::new(file.try_clone()?)
        .map_err(|e| GameError::InvalidDemo(format!("Invalid js-dos bundle: {e}")))?;
    if archive.is_empty() || archive.len() > max_files {
        return Err(GameError::InvalidDemo(
            "Invalid js-dos bundle file count.".to_string(),
        ));
    }
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|e| GameError::InvalidDemo(e.to_string()))?;
        if entry.name() == ".jsdos/jsdos.json" {
            has_manifest = true;
        }
        if entry.enclosed_name().is_none() {
            return Err(GameError::InvalidDemo(
                "js-dos bundle contains an unsafe path.".to_string(),
            ));
        }
        #[cfg(unix)]
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(GameError::InvalidDemo(
                "js-dos bundle cannot contain symlinks.".to_string(),
            ));
        }
    }
    if !has_manifest {
        return Err(GameError::InvalidDemo(
            "js-dos bundle must contain .jsdos/jsdos.json.".to_string(),
        ));
    }

    file.rewind()?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| GameError::InvalidDemo(e.to_string()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok((size, hex::encode(hasher.finalize())))
}

fn jsdos_temp_path(state: &AppState, upload_id: &str) -> PathBuf {
    state
        .project_demo_config
        .dir
        .join(".uploads")
        .join("jsdos")
        .join(format!("{upload_id}.part"))
}

fn jsdos_storage_key(game_id: i64, sha256: &str) -> String {
    format!("jsdos/{game_id}/{sha256}.jsdos")
}

pub async fn start_jsdos_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(game_id): AxumPath<i64>,
    Json(request): Json<StartJsDosUploadRequest>,
) -> Result<impl IntoResponse, GameError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse user id".to_string()))?;
    require_owner(&state.game_service.pool, "games", game_id, user_id).await?;
    if !request.file_name.to_ascii_lowercase().ends_with(".jsdos") {
        return Err(GameError::InvalidDemo(
            "Only .jsdos bundles are accepted.".to_string(),
        ));
    }
    if request.size_bytes == 0 || request.size_bytes > state.project_demo_config.max_jsdos_size {
        return Err(GameError::InvalidDemo(format!(
            "js-dos bundles must be between 1 byte and {} bytes.",
            state.project_demo_config.max_jsdos_size
        )));
    }

    let upload_id = Uuid::new_v4().to_string();
    let temp_path = jsdos_temp_path(&state, &upload_id);
    if let Some(parent) = temp_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::File::create(&temp_path).await?;
    let ttl_hours = state.project_demo_config.upload_session_ttl_hours;
    sqlx::query(
        r#"INSERT INTO game_jsdos_upload_sessions
           (id, game_id, uploader_id, original_file_name, expected_size_bytes,
            chunk_size_bytes, temp_storage_key, expires_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now', ?))"#,
    )
    .bind(&upload_id)
    .bind(game_id)
    .bind(user_id)
    .bind(&request.file_name)
    .bind(request.size_bytes as i64)
    .bind(state.project_demo_config.jsdos_chunk_size as i64)
    .bind(temp_path.to_string_lossy().to_string())
    .bind(format!("+{ttl_hours} hours"))
    .execute(&state.game_service.pool)
    .await?;

    Ok(Json(StartJsDosUploadResponse {
        upload_id,
        chunk_size_bytes: state.project_demo_config.jsdos_chunk_size,
        next_chunk_index: 0,
        expected_size_bytes: request.size_bytes,
    }))
}

pub async fn append_jsdos_chunk(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath((_game_id, upload_id, chunk_index)): AxumPath<(i64, String, u64)>,
    body: Bytes,
) -> Result<impl IntoResponse, GameError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse user id".to_string()))?;
    let row: Option<(i64, i64, i64, i64, i64, String, String)> = sqlx::query_as(
        "SELECT game_id, uploader_id, expected_size_bytes, received_size_bytes, next_chunk_index, temp_storage_key, status FROM game_jsdos_upload_sessions WHERE id = ?",
    )
    .bind(&upload_id)
    .fetch_optional(&state.game_service.pool)
    .await?;
    let (game_id, uploader_id, expected, received, next, temp_key, status) =
        row.ok_or(GameError::GameNotFound)?;
    if uploader_id != user_id || game_id != _game_id {
        return Err(GameError::Forbidden);
    }
    if status != "active" || chunk_index != next as u64 {
        return Err(GameError::InvalidDemo(
            "Invalid or out-of-order js-dos upload chunk.".to_string(),
        ));
    }
    let chunk_size = state.project_demo_config.jsdos_chunk_size;
    if body.is_empty()
        || body.len() as u64 > chunk_size
        || received as u64 + body.len() as u64 > expected as u64
    {
        return Err(GameError::InvalidDemo(
            "Invalid js-dos upload chunk size.".to_string(),
        ));
    }
    let mut file = tokio::fs::OpenOptions::new()
        .append(true)
        .open(&temp_key)
        .await?;
    file.write_all(&body).await?;
    file.flush().await?;
    let received_size = received as u64 + body.len() as u64;
    sqlx::query(
        "UPDATE game_jsdos_upload_sessions SET received_size_bytes = ?, next_chunk_index = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(received_size as i64)
    .bind((chunk_index + 1) as i64)
    .bind(&upload_id)
    .execute(&state.game_service.pool)
    .await?;
    Ok(Json(JsDosUploadResponse {
        received_size_bytes: received_size,
        next_chunk_index: chunk_index + 1,
    }))
}

pub async fn complete_jsdos_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath((game_id, upload_id)): AxumPath<(i64, String)>,
) -> Result<impl IntoResponse, GameError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse user id".to_string()))?;
    require_owner(&state.game_service.pool, "games", game_id, user_id).await?;
    let row: Option<(String, i64, i64, String, String)> = sqlx::query_as(
        "SELECT original_file_name, expected_size_bytes, received_size_bytes, temp_storage_key, status FROM game_jsdos_upload_sessions WHERE id = ? AND game_id = ? AND uploader_id = ?",
    )
    .bind(&upload_id)
    .bind(game_id)
    .bind(user_id)
    .fetch_optional(&state.game_service.pool)
    .await?;
    let (file_name, expected, received, temp_key, status) = row.ok_or(GameError::GameNotFound)?;
    if status != "active" || expected != received {
        return Err(GameError::InvalidDemo(
            "js-dos upload is incomplete.".to_string(),
        ));
    }
    let temp_path = PathBuf::from(&temp_key);
    let max_files = state.project_demo_config.max_files;
    let (size, sha256) =
        tokio::task::spawn_blocking(move || validate_jsdos_bundle(&temp_path, max_files))
            .await
            .map_err(|e| GameError::InternalError(e.to_string()))??;
    let storage_key = jsdos_storage_key(game_id, &sha256);
    let final_path = state.project_demo_config.dir.join(&storage_key);
    if let Some(parent) = final_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::rename(&temp_key, &final_path).await?;

    let mut tx = state.game_service.pool.begin().await?;
    sqlx::query(
        "INSERT INTO game_jsdos_bundles (game_id, storage_key, original_file_name, size_bytes, sha256) VALUES (?, ?, ?, ?, ?) ON CONFLICT(game_id) DO UPDATE SET storage_key = excluded.storage_key, original_file_name = excluded.original_file_name, size_bytes = excluded.size_bytes, sha256 = excluded.sha256, updated_at = CURRENT_TIMESTAMP",
    )
    .bind(game_id)
    .bind(&storage_key)
    .bind(&file_name)
    .bind(size as i64)
    .bind(&sha256)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE games SET launcher_type = 'jsdos', demo_url = NULL, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(game_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_jsdos_upload_sessions SET status = 'completed', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&upload_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(Json(CompleteJsDosUploadResponse {
        game_id,
        file_name,
        size_bytes: size,
        sha256,
        bundle_url: format!(
            "games/s/{}/jsdos",
            game_slug(&state, game_id).await?.unwrap_or_default()
        ),
    }))
}

async fn game_slug(state: &AppState, game_id: i64) -> Result<Option<String>, GameError> {
    Ok(sqlx::query_scalar(
        "SELECT posts.slug FROM games JOIN posts ON posts.id = games.post_id WHERE games.id = ?",
    )
    .bind(game_id)
    .fetch_optional(&state.game_service.pool)
    .await?)
}

pub async fn abort_jsdos_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath((game_id, upload_id)): AxumPath<(i64, String)>,
) -> Result<StatusCode, GameError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse user id".to_string()))?;
    let temp_key: Option<String> = sqlx::query_scalar(
        "SELECT temp_storage_key FROM game_jsdos_upload_sessions WHERE id = ? AND game_id = ? AND uploader_id = ? AND status = 'active'",
    )
    .bind(&upload_id)
    .bind(game_id)
    .bind(user_id)
    .fetch_optional(&state.game_service.pool)
    .await?;
    if let Some(temp_key) = temp_key {
        sqlx::query("UPDATE game_jsdos_upload_sessions SET status = 'aborted', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(&upload_id)
            .execute(&state.game_service.pool)
            .await?;
        tokio::fs::remove_file(temp_key).await.ok();
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_jsdos_bundle(
    State(state): State<Arc<AppState>>,
    AxumPath(slug): AxumPath<String>,
) -> Result<Response, GameError> {
    let storage_key: Option<String> = sqlx::query_scalar(
        "SELECT b.storage_key FROM game_jsdos_bundles b JOIN games g ON g.id = b.game_id JOIN posts ON posts.id = g.post_id WHERE posts.slug = ? AND posts.status = 'published' AND g.launcher_type = 'jsdos'",
    )
    .bind(&slug)
    .fetch_optional(&state.game_service.pool)
    .await?;
    let storage_key = storage_key.ok_or(GameError::GameNotFound)?;
    if Path::new(&storage_key).components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(GameError::InternalError(
            "Invalid js-dos storage key".to_string(),
        ));
    }
    let path = state.project_demo_config.dir.join(&storage_key);
    let file = tokio::fs::File::open(&path)
        .await
        .map_err(|_| GameError::GameNotFound)?;
    let metadata = file.metadata().await?;
    let mut response = Response::new(axum::body::Body::from_stream(ReaderStream::new(file)));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response.headers_mut().insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&metadata.len().to_string()).unwrap(),
    );
    Ok(response)
}
