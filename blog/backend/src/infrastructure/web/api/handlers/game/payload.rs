// The payload launchers ('shimkit', 'wasmshell'): chunked bundle upload,
// validation, unpacking. A bundle is one zip whose root carries the game
// host's manifest (bundle.json, format 1, see the win32-shim-kit repo's
// docs/web-bundle-contract.md); serving lives in payload_serve.rs.
use std::{
    fs,
    io::{Read, Seek},
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::{
    Extension, Json,
    body::Bytes,
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;
use zip::ZipArchive;

use crate::{
    domain::{entities::secret::Claims, errors::game::GameError},
    infrastructure::web::{
        api::handlers::game::dto::{
            CompletePayloadUploadResponse, PayloadUploadResponse, StartPayloadUploadRequest,
            StartPayloadUploadResponse,
        },
        api::support::ownership::require_owner,
        server::AppState,
    },
};

/// The manifest every bundle must carry at its root, and the kinds it may name.
const BUNDLE_MANIFEST: &str = "bundle.json";
const BUNDLE_FORMAT: f64 = 1.0;

fn payload_temp_path(state: &AppState, upload_id: &str) -> PathBuf {
    state
        .project_demo_config
        .dir
        .join(".uploads")
        .join("payloads")
        .join(format!("{upload_id}.part"))
}

fn payload_storage_key(game_id: i64, sha256: &str) -> String {
    // Content-addressed: the same zip re-uploaded unpacks into the same
    // directory, and a new bundle unpacks beside the old one so the flip to it
    // is one UPDATE.
    format!("payloads/{game_id}/{}", &sha256[..16.min(sha256.len())])
}

pub async fn start_payload_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(game_id): AxumPath<i64>,
    Json(request): Json<StartPayloadUploadRequest>,
) -> Result<impl IntoResponse, GameError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse user id".to_string()))?;
    require_owner(&state.game_service.pool, "games", game_id, user_id).await?;
    if !request.file_name.to_ascii_lowercase().ends_with(".zip") {
        return Err(GameError::InvalidDemo(
            "Only .zip bundles are accepted.".to_string(),
        ));
    }
    if request.size_bytes == 0 || request.size_bytes > state.project_demo_config.max_payload_size {
        return Err(GameError::InvalidDemo(format!(
            "Payload bundles must be between 1 byte and {} bytes.",
            state.project_demo_config.max_payload_size
        )));
    }

    let upload_id = Uuid::new_v4().to_string();
    let temp_path = payload_temp_path(&state, &upload_id);
    if let Some(parent) = temp_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::File::create(&temp_path).await?;
    let ttl_hours = state.project_demo_config.upload_session_ttl_hours;
    sqlx::query(
        r#"INSERT INTO game_payload_upload_sessions
           (id, game_id, uploader_id, original_file_name, expected_size_bytes,
            chunk_size_bytes, temp_storage_key, expires_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now', ?))"#,
    )
    .bind(&upload_id)
    .bind(game_id)
    .bind(user_id)
    .bind(&request.file_name)
    .bind(request.size_bytes as i64)
    .bind(state.project_demo_config.payload_chunk_size as i64)
    .bind(temp_path.to_string_lossy().to_string())
    .bind(format!("+{ttl_hours} hours"))
    .execute(&state.game_service.pool)
    .await?;

    Ok(Json(StartPayloadUploadResponse {
        upload_id,
        chunk_size_bytes: state.project_demo_config.payload_chunk_size,
        next_chunk_index: 0,
        expected_size_bytes: request.size_bytes,
    }))
}

pub async fn append_payload_chunk(
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
        "SELECT game_id, uploader_id, expected_size_bytes, received_size_bytes, next_chunk_index, temp_storage_key, status FROM game_payload_upload_sessions WHERE id = ?",
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
            "Invalid or out-of-order payload upload chunk.".to_string(),
        ));
    }
    let chunk_size = state.project_demo_config.payload_chunk_size;
    if body.is_empty()
        || body.len() as u64 > chunk_size
        || received as u64 + body.len() as u64 > expected as u64
    {
        return Err(GameError::InvalidDemo(
            "Invalid payload upload chunk size.".to_string(),
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
        "UPDATE game_payload_upload_sessions SET received_size_bytes = ?, next_chunk_index = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(received_size as i64)
    .bind((chunk_index + 1) as i64)
    .bind(&upload_id)
    .execute(&state.game_service.pool)
    .await?;
    Ok(Json(PayloadUploadResponse {
        received_size_bytes: received_size,
        next_chunk_index: chunk_index + 1,
    }))
}

pub async fn abort_payload_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath((game_id, upload_id)): AxumPath<(i64, String)>,
) -> Result<StatusCode, GameError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse user id".to_string()))?;
    let temp_key: Option<String> = sqlx::query_scalar(
        "SELECT temp_storage_key FROM game_payload_upload_sessions WHERE id = ? AND game_id = ? AND uploader_id = ? AND status = 'active'",
    )
    .bind(&upload_id)
    .bind(game_id)
    .bind(user_id)
    .fetch_optional(&state.game_service.pool)
    .await?;
    if let Some(temp_key) = temp_key {
        sqlx::query("UPDATE game_payload_upload_sessions SET status = 'aborted', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(&upload_id)
            .execute(&state.game_service.pool)
            .await?;
        tokio::fs::remove_file(temp_key).await.ok();
    }
    Ok(StatusCode::NO_CONTENT)
}

/// What a bundle is, read out of its own manifest: the only fields the backend
/// needs to store a bundle and answer about it.
struct BundleManifest {
    kind: String,
    slug: String,
    title: String,
    text: String,
}

fn read_bundle_manifest(archive: &mut ZipArchive<fs::File>) -> Result<BundleManifest, GameError> {
    let mut manifest = archive
        .by_name(BUNDLE_MANIFEST)
        .map_err(|_| GameError::InvalidDemo("bundle.json must be at the zip root.".to_string()))?;
    let mut text = String::new();
    manifest
        .read_to_string(&mut text)
        .map_err(|e| GameError::InvalidDemo(format!("bundle.json is not valid UTF-8: {e}")))?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|e| GameError::InvalidDemo(format!("bundle.json does not parse: {e}")))?;
    let format = value.get("format").and_then(Value::as_f64).unwrap_or(0.0);
    if format != BUNDLE_FORMAT {
        return Err(GameError::InvalidDemo(format!(
            "bundle.json format {format} is not supported (expected 1)."
        )));
    }
    let kind = value
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if kind != "shimkit" && kind != "wasmshell" {
        return Err(GameError::InvalidDemo(
            "bundle.json kind must be 'shimkit' or 'wasmshell'.".to_string(),
        ));
    }
    Ok(BundleManifest {
        slug: value
            .get("slug")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        title: value
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        kind,
        text,
    })
}

/// One pass over the archive: validate every entry, then unpack into
/// `target`. Anything unsafe, too numerous, or too large fails the whole
/// bundle and leaves nothing behind.
fn unpack_payload_bundle(
    zip_path: &Path,
    target: &Path,
    max_files: usize,
    max_extracted_bytes: u64,
) -> Result<(u64, String, usize, u64, BundleManifest), GameError> {
    let file = fs::File::open(zip_path)?;
    let zip_size = file.metadata()?.len();
    let mut archive = ZipArchive::new(file)
        .map_err(|e| GameError::InvalidDemo(format!("Invalid payload bundle: {e}")))?;
    let manifest = read_bundle_manifest(&mut archive)?;

    let mut extracted_bytes: u64 = 0;
    let mut extracted_files: usize = 0;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|e| GameError::InvalidDemo(format!("Unreadable bundle entry: {e}")))?;
        if entry.is_dir() {
            continue;
        }
        let Some(relative) = entry.enclosed_name() else {
            return Err(GameError::InvalidDemo(
                "Payload bundle contains an unsafe path.".to_string(),
            ));
        };
        #[cfg(unix)]
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(GameError::InvalidDemo(
                "Payload bundle cannot contain symlinks.".to_string(),
            ));
        }
        if entry.name().ends_with('/') {
            continue;
        }
        extracted_files += 1;
        extracted_bytes += entry.size();
        if extracted_files > max_files {
            return Err(GameError::InvalidDemo(format!(
                "Payload bundle holds more than {max_files} files."
            )));
        }
        if extracted_bytes > max_extracted_bytes {
            return Err(GameError::InvalidDemo(
                "Payload bundle unpacks beyond the allowed size.".to_string(),
            ));
        }
        let out_path = target.join(relative);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = fs::File::create(&out_path)?;
        std::io::copy(&mut entry, &mut out)?;
    }
    if extracted_files == 0 {
        return Err(GameError::InvalidDemo(
            "Payload bundle is empty.".to_string(),
        ));
    }

    // The digest is of the zip itself, not the tree: it is what identifies the
    // upload, and the storage key carries a slice of it.
    let mut file = fs::File::open(zip_path)?;
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
    Ok((
        zip_size,
        hex::encode(hasher.finalize()),
        extracted_files,
        extracted_bytes,
        manifest,
    ))
}

pub async fn complete_payload_upload(
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
        "SELECT original_file_name, expected_size_bytes, received_size_bytes, temp_storage_key, status FROM game_payload_upload_sessions WHERE id = ? AND game_id = ? AND uploader_id = ?",
    )
    .bind(&upload_id)
    .bind(game_id)
    .bind(user_id)
    .fetch_optional(&state.game_service.pool)
    .await?;
    let (file_name, expected, received, temp_key, status) = row.ok_or(GameError::GameNotFound)?;
    if status != "active" || expected != received {
        return Err(GameError::InvalidDemo(
            "Payload upload is incomplete.".to_string(),
        ));
    }
    let temp_path = PathBuf::from(&temp_key);
    let max_files = state.project_demo_config.max_payload_files;
    let max_extracted = state.project_demo_config.max_payload_extracted_size;
    let storage_base = state.project_demo_config.dir.clone();
    let (zip_size, sha256, files, bytes, manifest) = tokio::task::spawn_blocking(move || {
        let staging = storage_base
            .join(".uploads")
            .join("payloads")
            .join("unpack");
        let _ = fs::remove_dir_all(&staging);
        fs::create_dir_all(&staging)?;
        let result = unpack_payload_bundle(&temp_path, &staging, max_files, max_extracted);
        match result {
            Ok(good) => Ok(good),
            Err(error) => {
                let _ = fs::remove_dir_all(&staging);
                Err(error)
            }
        }
    })
    .await
    .map_err(|e| GameError::InternalError(e.to_string()))??;

    // Unpack straight into the content-addressed home: the staging pass above
    // validated the tree, so this rename is the only move the bytes get.
    let storage_key = payload_storage_key(game_id, &sha256);
    let final_path = state.project_demo_config.dir.join(&storage_key);
    if let Some(parent) = final_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let old_key: Option<String> =
        sqlx::query_scalar("SELECT storage_key FROM game_payload_bundles WHERE game_id = ?")
            .bind(game_id)
            .fetch_optional(&state.game_service.pool)
            .await?
            .filter(|existing| *existing != storage_key);
    tokio::fs::remove_dir_all(&final_path).await.ok();
    tokio::fs::rename(
        state
            .project_demo_config
            .dir
            .join(".uploads")
            .join("payloads")
            .join("unpack"),
        &final_path,
    )
    .await?;

    let mut tx = state.game_service.pool.begin().await?;
    sqlx::query(
        r#"INSERT INTO game_payload_bundles
           (game_id, kind, storage_key, zip_sha256, zip_size_bytes, manifest_json)
           VALUES (?, ?, ?, ?, ?, ?)
           ON CONFLICT(game_id) DO UPDATE SET
             kind = excluded.kind,
             storage_key = excluded.storage_key,
             zip_sha256 = excluded.zip_sha256,
             zip_size_bytes = excluded.zip_size_bytes,
             manifest_json = excluded.manifest_json,
             artifact_revision = game_payload_bundles.artifact_revision + 1,
             updated_at = CURRENT_TIMESTAMP"#,
    )
    .bind(game_id)
    .bind(&manifest.kind)
    .bind(&storage_key)
    .bind(&sha256)
    .bind(zip_size as i64)
    .bind(&manifest.text)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE games SET launcher_type = ?, demo_url = NULL, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(&manifest.kind)
    .bind(game_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE game_payload_upload_sessions SET status = 'completed', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&upload_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    if let Some(old_key) = old_key {
        tokio::fs::remove_dir_all(state.project_demo_config.dir.join(old_key))
            .await
            .ok();
    }
    tokio::fs::remove_file(&temp_key).await.ok();

    let slug = game_slug(&state, game_id).await?.unwrap_or_default();
    Ok(Json(CompletePayloadUploadResponse {
        game_id,
        file_name,
        kind: manifest.kind,
        slug: manifest.slug,
        title: manifest.title,
        size_bytes: zip_size,
        sha256,
        files,
        extracted_bytes: bytes,
        payload_url: format!("games/s/{slug}/payload/"),
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
