// Unified per-user save files for the payload launchers ('shimkit',
// 'wasmshell'). One guest-written file is one row, keyed by its name in the
// game's own save set; the browser always keeps its own copy, so this is the
// account's mirror in the cloud, and guests never reach it. The v86 floppies
// keep their own single-blob shape beside this (handlers/v86/saves.rs).
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use axum::{
    Extension,
    body::Bytes,
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::{
    domain::{entities::secret::Claims, errors::game::GameError},
    infrastructure::{
        storage::ObjectStore,
        web::{
            api::handlers::{
                game::dto::{GameSaveEntry, GameSavesResponse},
                v86::{streamed_fs_file, streamed_object},
            },
            server::AppState,
        },
    },
};

/// A save file's size cap. Vice City's slots are ~200 KB; a shim-kit game's
/// writes are smaller still. Anything bigger is not a save, it is a backup.
const MAX_SAVE_BYTES: usize = 2 * 1024 * 1024;
/// One save per user per game per moment: the player's autosave loop retries,
/// and without this gate a stuck loop becomes a write storm.
const SAVE_RATE_LIMIT_MS: i64 = 2000;

/// A save key is a file name inside the game's save set, never a path: letters,
/// digits, dot, dash, underscore, space. Everything else is rejected before it
/// can become a storage key or a traversal.
fn valid_save_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 128
        && key != "."
        && key != ".."
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ' '))
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
        && now - last < SAVE_RATE_LIMIT_MS
    {
        return true;
    }
    map.insert(key.to_string(), now);
    false
}

async fn save_game_id(state: &AppState, slug: &str) -> Result<i64, GameError> {
    sqlx::query_scalar(
        r#"SELECT g.id FROM games g
           JOIN posts ON posts.id = g.post_id
           WHERE posts.slug = ? AND posts.status = 'published'
             AND g.launcher_type IN ('shimkit', 'wasmshell')"#,
    )
    .bind(slug)
    .fetch_optional(&state.game_service.pool)
    .await?
    .ok_or(GameError::GameNotFound)
}

fn user_id(claims: &Claims) -> Result<i64, GameError> {
    claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse user id".to_string()))
}

/// An anonymous caller and a save that does not exist look the same from the
/// outside: nothing is leaked about either.
fn require_user(opt_claims: Option<Claims>) -> Result<i64, GameError> {
    user_id(&opt_claims.ok_or(GameError::GameNotFound)?)
}

pub async fn list_game_saves(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    AxumPath(slug): AxumPath<String>,
) -> Result<impl IntoResponse, GameError> {
    let user_id = require_user(opt_claims)?;
    let game_id = save_game_id(&state, &slug).await?;
    let rows: Vec<(String, i64, String, String)> = sqlx::query_as(
        "SELECT save_key, size_bytes, sha256, updated_at FROM game_saves WHERE game_id = ? AND user_id = ? ORDER BY save_key",
    )
    .bind(game_id)
    .bind(user_id)
    .fetch_all(&state.game_service.pool)
    .await?;
    Ok(axum::Json(GameSavesResponse {
        saves: rows
            .into_iter()
            .map(|(key, size_bytes, sha256, updated_at)| GameSaveEntry {
                key,
                size_bytes,
                sha256,
                updated_at,
            })
            .collect(),
    }))
}

pub async fn get_game_save(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    AxumPath((slug, key)): AxumPath<(String, String)>,
) -> Result<Response, GameError> {
    let user_id = require_user(opt_claims)?;
    let game_id = save_game_id(&state, &slug).await?;
    if !valid_save_key(&key) {
        return Err(GameError::GameNotFound);
    }
    let storage_key: Option<String> = sqlx::query_scalar(
        "SELECT storage_key FROM game_saves WHERE game_id = ? AND user_id = ? AND save_key = ?",
    )
    .bind(game_id)
    .bind(user_id)
    .bind(&key)
    .fetch_optional(&state.game_service.pool)
    .await?;
    let Some(storage_key) = storage_key else {
        return Err(GameError::SaveNotFound);
    };
    // The helpers are ProjectError-typed because the v86 handlers own them;
    // GameError::Project is the bridge this codebase already has for exactly
    // that, and it maps a not-found to the same 404 either way.
    match &state.storage {
        ObjectStore::R2(_) => streamed_object(
            &state.storage,
            &storage_key,
            "application/octet-stream",
            "no-store",
        )
        .await
        .map_err(GameError::Project),
        ObjectStore::Fs(_) => streamed_fs_file(
            state.project_demo_config.dir.join(&storage_key),
            "application/octet-stream",
            "no-store",
        )
        .await
        .map_err(GameError::Project),
    }
}

pub async fn put_game_save(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    AxumPath((slug, key)): AxumPath<(String, String)>,
    bytes: Bytes,
) -> Result<StatusCode, GameError> {
    let user_id = user_id(&opt_claims.ok_or(GameError::Forbidden)?)?;
    let game_id = save_game_id(&state, &slug).await?;
    if !valid_save_key(&key) {
        return Err(GameError::InvalidDemo(
            "Save names are plain file names.".to_string(),
        ));
    }
    if bytes.is_empty() || bytes.len() > MAX_SAVE_BYTES {
        return Err(GameError::InvalidDemo(
            "The save file exceeds the allowed size.".to_string(),
        ));
    }
    if save_rate_limited(&save_rate_limit_key(user_id, game_id)) {
        return Err(GameError::Conflict(
            "Please wait before saving again.".to_string(),
        ));
    }
    // Level-19 zstd over up to 2 MB is seconds of CPU; run it with the hash on
    // the blocking pool, exactly as the v86 saves do.
    let (compressed, sha) = tokio::task::spawn_blocking(move || -> Result<_, GameError> {
        let mut encoder = zstd::stream::write::Encoder::new(Vec::new(), 19)
            .map_err(|e| GameError::InternalError(format!("zstd encode start: {e}")))?;
        std::io::Write::write_all(&mut encoder, &bytes)
            .map_err(|e| GameError::InternalError(format!("zstd encode: {e}")))?;
        let compressed = encoder
            .finish()
            .map_err(|e| GameError::InternalError(format!("zstd encode finish: {e}")))?;
        let sha = hex::encode(Sha256::digest(&compressed));
        Ok((compressed, sha))
    })
    .await
    .map_err(|e| GameError::InternalError(e.to_string()))??;
    // Content-addressed, so re-saving identical bytes rewrites one object and
    // the row's digest is the truth about what the cloud holds.
    let storage_key = format!("games/saves/{user_id}/{game_id}/{sha}.zst");
    let size_bytes = compressed.len();
    state
        .storage
        .put_object_bytes(&storage_key, compressed)
        .await
        .map_err(|error| GameError::InternalError(format!("save storage: {error}")))?;
    sqlx::query(
        r#"INSERT INTO game_saves (game_id, user_id, save_key, storage_key, size_bytes, sha256)
           VALUES (?, ?, ?, ?, ?, ?)
           ON CONFLICT(game_id, user_id, save_key) DO UPDATE SET
             storage_key = excluded.storage_key,
             size_bytes = excluded.size_bytes,
             sha256 = excluded.sha256,
             updated_at = CURRENT_TIMESTAMP"#,
    )
    .bind(game_id)
    .bind(user_id)
    .bind(&key)
    .bind(&storage_key)
    .bind(size_bytes as i64)
    .bind(&sha)
    .execute(&state.game_service.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_game_save(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    AxumPath((slug, key)): AxumPath<(String, String)>,
) -> Result<StatusCode, GameError> {
    let user_id = require_user(opt_claims)?;
    let game_id = save_game_id(&state, &slug).await?;
    if !valid_save_key(&key) {
        return Err(GameError::GameNotFound);
    }
    let storage_key: Option<String> = sqlx::query_scalar(
        "SELECT storage_key FROM game_saves WHERE game_id = ? AND user_id = ? AND save_key = ?",
    )
    .bind(game_id)
    .bind(user_id)
    .bind(&key)
    .fetch_optional(&state.game_service.pool)
    .await?;
    if let Some(storage_key) = storage_key {
        let _ = state.storage.delete_object(&storage_key).await;
        sqlx::query("DELETE FROM game_saves WHERE game_id = ? AND user_id = ? AND save_key = ?")
            .bind(game_id)
            .bind(user_id)
            .bind(&key)
            .execute(&state.game_service.pool)
            .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}
