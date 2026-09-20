// Per-user cloud saves, transported as a zstd-compressed 1.44 MB floppy image.
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use axum::{
    Extension,
    body::Bytes,
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::Response,
};
use chrono::Utc;
use sha2::{Digest, Sha256};
use sqlx::Row;

use crate::domain::{entities::secret::Claims, errors::project::ProjectError};
use crate::infrastructure::{storage::ObjectStore, web::server::AppState};

use super::constants::{V86_SAVE_FLOPPY_BYTES, V86_SAVE_MAX_UPLOAD_BYTES, V86_SAVE_RATE_LIMIT_MS};
use super::serving::{streamed_fs_file, streamed_object};
use super::shared::{storage_error, user_id};

fn zstd_compress(data: &[u8]) -> Result<Vec<u8>, ProjectError> {
    let mut encoder = zstd::stream::write::Encoder::new(Vec::new(), 19)
        .map_err(|e| ProjectError::InternalError(format!("zstd encode start: {e}")))?;
    std::io::Write::write_all(&mut encoder, data)
        .map_err(|e| ProjectError::InternalError(format!("zstd encode: {e}")))?;
    encoder
        .finish()
        .map_err(|e| ProjectError::InternalError(format!("zstd encode finish: {e}")))
}

#[allow(dead_code)]
fn zstd_decode(data: &[u8]) -> Result<Vec<u8>, ProjectError> {
    use std::io::Read;
    let mut output = Vec::with_capacity(V86_SAVE_FLOPPY_BYTES);
    zstd::stream::read::Decoder::new(data)
        .map_err(|e| ProjectError::InternalError(format!("zstd decode start: {e}")))?
        .read_to_end(&mut output)
        .map_err(|e| ProjectError::InternalError(format!("zstd decode: {e}")))?;
    Ok(output)
}

fn save_rate_limit_key(user_id: i64, game_id: i64) -> String {
    format!("{user_id}:{game_id}")
}

fn save_rate_limited(key: &str) -> bool {
    let now = Utc::now().timestamp_millis();
    static MAP: OnceLock<Mutex<HashMap<String, i64>>> = OnceLock::new();
    let map = MAP.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = map.lock().unwrap();
    if let Some(&last) = map.get(key)
        && now - last < V86_SAVE_RATE_LIMIT_MS as i64
    {
        return true;
    }
    map.insert(key.to_string(), now);
    false
}

async fn save_game_id(
    state: &AppState,
    slug: &str,
) -> Result<i64, ProjectError> {
    sqlx::query_scalar(
        r#"SELECT gm.id FROM games gm
           JOIN posts ON posts.id = gm.post_id
           WHERE posts.slug = ? AND posts.status = 'published' AND gm.launcher_type = 'v86'"#,
    )
    .bind(slug)
    .fetch_optional(&state.project_service.pool)
    .await?
    .ok_or(ProjectError::ProjectNotFound)
}

pub async fn get_game_save(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    AxumPath(slug): AxumPath<String>,
) -> Result<Response, ProjectError> {
    let Some(claims) = opt_claims else {
        return Err(ProjectError::ProjectNotFound);
    };
    let user_id = user_id(&claims)?;
    let game_id = save_game_id(&state, &slug).await?;
    let row = sqlx::query(
        "SELECT storage_key, size_bytes FROM game_v86_saves WHERE game_id = ? AND user_id = ?",
    )
    .bind(game_id)
    .bind(user_id)
    .fetch_optional(&state.project_service.pool)
    .await?;
    let (storage_key, _size) = match row {
        Some(row) => (row.get::<String, _>("storage_key"), row.get::<i64, _>("size_bytes")),
        None => return Err(ProjectError::SaveNotFound),
    };
    match &state.storage {
        ObjectStore::R2(_) => {
            streamed_object(
                &state.storage,
                &storage_key,
                "application/octet-stream",
                "no-store",
            )
            .await
        }
        ObjectStore::Fs(_) => {
            streamed_fs_file(
                state.project_demo_config.dir.join(storage_key),
                "application/octet-stream",
                "no-store",
            )
            .await
        }
    }
}

pub async fn put_game_save(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    AxumPath(slug): AxumPath<String>,
    bytes: Bytes,
) -> Result<StatusCode, ProjectError> {
    let user_id = user_id(&opt_claims.ok_or(ProjectError::Forbidden)?)?;
    let game_id = save_game_id(&state, &slug).await?;
    if bytes.is_empty() || bytes.len() > V86_SAVE_MAX_UPLOAD_BYTES {
        return Err(ProjectError::InvalidDemo(
            "The save image exceeds the allowed size.".to_string(),
        ));
    }
    let key = save_rate_limit_key(user_id, game_id);
    if save_rate_limited(&key) {
        return Err(ProjectError::Conflict(
            "Please wait before saving again.".to_string(),
        ));
    }
    // Level-19 zstd over up to 2 MB is seconds of CPU; run it with the hash
    // on the blocking pool.
    let (compressed, sha) = tokio::task::spawn_blocking(move || -> Result<_, ProjectError> {
        let compressed = zstd_compress(&bytes)?;
        let sha = hex::encode(Sha256::digest(&compressed));
        Ok((compressed, sha))
    })
    .await
    .map_err(|e| ProjectError::InternalError(e.to_string()))??;
    let storage_key = format!("v86/saves/{user_id}/{game_id}/save.zst");
    let size_bytes = compressed.len();
    state
        .storage
        .put_object_bytes(&storage_key, compressed)
        .await
        .map_err(storage_error)?;
    sqlx::query(
        r#"INSERT INTO game_v86_saves (game_id, user_id, storage_key, size_bytes, sha256)
           VALUES (?, ?, ?, ?, ?)
           ON CONFLICT(game_id, user_id) DO UPDATE SET
             storage_key = excluded.storage_key,
             size_bytes = excluded.size_bytes,
             sha256 = excluded.sha256,
             updated_at = CURRENT_TIMESTAMP"#,
    )
    .bind(game_id)
    .bind(user_id)
    .bind(&storage_key)
    .bind(size_bytes as i64)
    .bind(&sha)
    .execute(&state.project_service.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_game_save(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    AxumPath(slug): AxumPath<String>,
) -> Result<StatusCode, ProjectError> {
    let user_id = user_id(&opt_claims.ok_or(ProjectError::Forbidden)?)?;
    let game_id = save_game_id(&state, &slug).await?;
    let row = sqlx::query(
        "SELECT storage_key FROM game_v86_saves WHERE game_id = ? AND user_id = ?",
    )
    .bind(game_id)
    .bind(user_id)
    .fetch_optional(&state.project_service.pool)
    .await?;
    if let Some(row) = row {
        let storage_key: String = row.get("storage_key");
        let _ = state.storage.delete_object(&storage_key).await;
        sqlx::query("DELETE FROM game_v86_saves WHERE game_id = ? AND user_id = ?")
            .bind(game_id)
            .bind(user_id)
            .execute(&state.project_service.pool)
            .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

