// Streaming v86 artifacts to browsers: snapshot blobs, system and game disk
// chunks, and launcher CDs, from either object-store backend.
use std::{
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use axum::{
    Extension,
    extract::{Path as AxumPath, State},
    http::{HeaderValue, header},
    response::Response,
};
use axum_extra::extract::cookie::CookieJar;
use tokio_util::io::ReaderStream;

use crate::domain::entities::secret::Claims;
use crate::domain::errors::project::ProjectError;
use crate::infrastructure::{storage::ObjectStore, web::server::AppState};

use super::shared::storage_error;
use super::snapshot_uploads::validate_sha256_hex;
use crate::infrastructure::web::api::handlers::game::access::{
    entitled, entitled_with, game_access_by_slug,
};

/// The invited-only gate for the slug-addressed game routes. Everything the
/// gate reads lives behind the game handlers' resolver; the error mapping is
/// the only thing that differs here (ProjectError, not GameError).
async fn game_gate(
    state: &AppState,
    slug: &str,
    viewer: Option<&Claims>,
    jar: &CookieJar,
) -> Result<(), ProjectError> {
    let access = game_access_by_slug(&state.game_service.pool, slug)
        .await
        .map_err(|e| ProjectError::InternalError(e.to_string()))?;
    let allowed = match &access {
        Some(access) => entitled(state, access, viewer, jar)
            .await
            .map_err(|e| ProjectError::InternalError(e.to_string()))?,
        None => false,
    };
    if allowed {
        Ok(())
    } else {
        Err(ProjectError::ProjectNotFound)
    }
}

/// Public serve for the local-storage fallback (no R2 public domain). With R2
/// configured the descriptor points straight at the bucket instead.
pub async fn get_snapshot_blob(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    jar: CookieJar,
    AxumPath((sha256, part)): AxumPath<(String, String)>,
) -> Result<Response, ProjectError> {
    if part != "state.zst" {
        return Err(ProjectError::ProjectNotFound);
    }
    validate_sha256_hex(&sha256).map_err(|_| ProjectError::ProjectNotFound)?;
    // Only serve digests referenced by a published project, matching the disk
    // and ISO routes. A snapshot is a fully booted machine with the game on
    // it, so a draft's state must not be reachable before the post goes live.
    let row: Option<(String, i64, String, i64)> = sqlx::query_as(
        r#"SELECT s.storage_key, gm.id, gm.access_mode, posts.user_id
           FROM game_v86_snapshots s
           JOIN games gm ON gm.id = s.game_id
           JOIN posts ON posts.id = gm.post_id
           WHERE s.sha256 = ? AND posts.status = 'published' AND gm.launcher_type = 'v86'
           LIMIT 1"#,
    )
    .bind(&sha256)
    .fetch_optional(&state.project_service.pool)
    .await?;
    let (storage_key, game_id, access_mode, owner_user_id) =
        row.ok_or(ProjectError::ProjectNotFound)?;
    // A snapshot carries the whole game disk, so invited-only games gate it
    // exactly like the disk routes do.
    let allowed = entitled_with(
        &state,
        game_id,
        &access_mode,
        owner_user_id,
        opt_claims.as_ref(),
        &jar,
    )
    .await
    .map_err(|e| ProjectError::InternalError(e.to_string()))?;
    if !allowed {
        return Err(ProjectError::ProjectNotFound);
    }
    match &state.storage {
        ObjectStore::R2(_) => {
            streamed_object(
                &state.storage,
                &storage_key,
                "application/octet-stream",
                IMMUTABLE_CACHE_CONTROL,
            )
            .await
        }
        ObjectStore::Fs(_) => {
            streamed_fs_file(
                state.project_demo_config.dir.join(&storage_key),
                "application/octet-stream",
                IMMUTABLE_CACHE_CONTROL,
            )
            .await
        }
    }
}

const IMMUTABLE_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

fn streamed_response(
    body: axum::body::Body,
    size: u64,
    content_type: &'static str,
    cache_control: &'static str,
) -> Response {
    let mut response = Response::new(body);
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(cache_control),
    );
    if let Ok(value) = HeaderValue::from_str(&size.to_string()) {
        response.headers_mut().insert(header::CONTENT_LENGTH, value);
    }
    response
}

// pub(crate): the payload launchers' save serving (handlers/game/saves.rs)
// streams from the same store, and GameError::Project bridges the error type.
pub(crate) async fn streamed_fs_file(
    path: PathBuf,
    content_type: &'static str,
    cache_control: &'static str,
) -> Result<Response, ProjectError> {
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|_| ProjectError::ProjectNotFound)?;
    let size = file
        .metadata()
        .await
        .map_err(|_| ProjectError::ProjectNotFound)?
        .len();
    Ok(streamed_response(
        axum::body::Body::from_stream(ReaderStream::new(file)),
        size,
        content_type,
        cache_control,
    ))
}

/// Streams an object from the store by key. Keys mirror the R2 layout, so the
/// same key resolves in either backend.
pub(crate) async fn streamed_object(
    storage: &ObjectStore,
    key: &str,
    content_type: &'static str,
    cache_control: &'static str,
) -> Result<Response, ProjectError> {
    let size = storage
        .object_size(key)
        .await
        .map_err(storage_error)?
        .ok_or(ProjectError::ProjectNotFound)?;
    let reader = storage
        .get_object_reader(key)
        .await
        .map_err(storage_error)?;
    Ok(streamed_response(
        axum::body::Body::from_stream(ReaderStream::new(reader)),
        size,
        content_type,
        cache_control,
    ))
}

pub async fn get_system_chunk(
    State(state): State<Arc<AppState>>,
    AxumPath((sha256, part)): AxumPath<(String, String)>,
) -> Result<Response, ProjectError> {
    let storage_key: Option<String> = sqlx::query_scalar(
        r#"SELECT v.storage_key FROM v86_system_versions v
           JOIN v86_systems s ON s.id = v.system_id
           WHERE v.sha256 = ?
             AND s.is_active = 1
             AND v.version_number = s.current_version
           LIMIT 1"#,
    )
    .bind(&sha256)
    .fetch_optional(&state.project_service.pool)
    .await?;
    let storage_key = storage_key.ok_or(ProjectError::ProjectNotFound)?;
    if part == ".img"
        || part.contains('/')
        || !(part.ends_with(".img") || part.ends_with(".img.zst"))
    {
        return Err(ProjectError::ProjectNotFound);
    }
    streamed_fs_file(
        state.project_demo_config.dir.join(&storage_key).join(part),
        "application/octet-stream",
        IMMUTABLE_CACHE_CONTROL,
    )
    .await
}

pub async fn get_game_chunk(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    jar: CookieJar,
    AxumPath((slug, sha256, part)): AxumPath<(String, String, String)>,
) -> Result<Response, ProjectError> {
    game_gate(&state, &slug, opt_claims.as_ref(), &jar).await?;
    let storage_key: Option<String> = sqlx::query_scalar(
        r#"SELECT iso_storage_key
           FROM (
                SELECT g.iso_storage_key FROM game_v86_games g
                JOIN games gm ON gm.id = g.game_id
                JOIN posts ON posts.id = gm.post_id
                WHERE posts.slug = ? AND posts.status = 'published'
                  AND gm.launcher_type = 'v86' AND g.iso_sha256 = ?
             UNION
                SELECT v.iso_storage_key FROM game_v86_variants v
                JOIN games gm ON gm.id = v.game_id
                JOIN posts ON posts.id = gm.post_id
                WHERE posts.slug = ? AND posts.status = 'published'
                  AND gm.launcher_type = 'v86' AND v.iso_sha256 = ?
           ) LIMIT 1"#,
    )
    .bind(&slug)
    .bind(&sha256)
    .bind(&slug)
    .bind(&sha256)
    .fetch_optional(&state.project_service.pool)
    .await?;
    let storage_key = storage_key.ok_or(ProjectError::ProjectNotFound)?;
    if part == ".iso"
        || part.contains('/')
        || !(part.ends_with(".iso") || part.ends_with(".iso.zst"))
    {
        return Err(ProjectError::ProjectNotFound);
    }
    streamed_fs_file(
        state
            .project_demo_config
            .dir
            .join(storage_key)
            .join("parts")
            .join(part),
        "application/octet-stream",
        IMMUTABLE_CACHE_CONTROL,
    )
    .await
}

pub async fn get_game_disk_chunk(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    jar: CookieJar,
    AxumPath((slug, sha256, part)): AxumPath<(String, String, String)>,
) -> Result<Response, ProjectError> {
    game_gate(&state, &slug, opt_claims.as_ref(), &jar).await?;
    let storage_key: Option<String> = sqlx::query_scalar(
        r#"SELECT g.disk_storage_key FROM game_v86_games g
           JOIN games gm ON gm.id = g.game_id
           JOIN posts ON posts.id = gm.post_id
           WHERE posts.slug = ? AND posts.status = 'published'
             AND gm.launcher_type = 'v86' AND g.disk_sha256 = ?"#,
    )
    .bind(&slug)
    .bind(&sha256)
    .fetch_optional(&state.project_service.pool)
    .await?;
    let storage_key = storage_key.ok_or(ProjectError::ProjectNotFound)?;
    if part == ".img"
        || part.contains('/')
        || !(part.ends_with(".img") || part.ends_with(".img.zst"))
    {
        return Err(ProjectError::ProjectNotFound);
    }
    match &state.storage {
        ObjectStore::R2(_) => {
            let key = format!("{storage_key}/{part}");
            streamed_object(
                &state.storage,
                &key,
                "application/octet-stream",
                IMMUTABLE_CACHE_CONTROL,
            )
            .await
        }
        ObjectStore::Fs(_) => {
            streamed_fs_file(
                state.project_demo_config.dir.join(storage_key).join(part),
                "application/octet-stream",
                IMMUTABLE_CACHE_CONTROL,
            )
            .await
        }
    }
}

pub async fn get_game_iso(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    jar: CookieJar,
    AxumPath((slug, sha256)): AxumPath<(String, String)>,
) -> Result<Response, ProjectError> {
    game_gate(&state, &slug, opt_claims.as_ref(), &jar).await?;
    let storage_key: Option<String> = sqlx::query_scalar(
        r#"SELECT iso_storage_key
           FROM (
                SELECT g.iso_storage_key FROM game_v86_games g
                JOIN games gm ON gm.id = g.game_id
                JOIN posts ON posts.id = gm.post_id
                WHERE posts.slug = ? AND posts.status = 'published'
                  AND gm.launcher_type = 'v86' AND g.iso_sha256 = ?
             UNION
                SELECT v.iso_storage_key FROM game_v86_variants v
                JOIN games gm ON gm.id = v.game_id
                JOIN posts ON posts.id = gm.post_id
                WHERE posts.slug = ? AND posts.status = 'published'
                  AND gm.launcher_type = 'v86' AND v.iso_sha256 = ?
           ) LIMIT 1"#,
    )
    .bind(&slug)
    .bind(&sha256)
    .bind(&slug)
    .bind(&sha256)
    .fetch_optional(&state.project_service.pool)
    .await?;
    let storage_key = storage_key.ok_or(ProjectError::ProjectNotFound)?;
    if Path::new(&storage_key).components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(ProjectError::InternalError(
            "Invalid v86 game storage key.".to_string(),
        ));
    }

    match &state.storage {
        ObjectStore::R2(_) => {
            let key = format!("v86/games/{sha256}/full.iso");
            let size = state
                .storage
                .object_size(&key)
                .await
                .map_err(storage_error)?
                .ok_or(ProjectError::ProjectNotFound)?;
            let reader = state
                .storage
                .get_object_reader(&key)
                .await
                .map_err(storage_error)?;
            let mut response =
                Response::new(axum::body::Body::from_stream(ReaderStream::new(reader)));
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/octet-stream"),
            );
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            );
            response.headers_mut().insert(
                header::CONTENT_LENGTH,
                HeaderValue::from_str(&size.to_string())
                    .map_err(|error| ProjectError::InternalError(error.to_string()))?,
            );
            response.headers_mut().insert(
                header::ETAG,
                HeaderValue::from_str(&format!("\"{sha256}\""))
                    .map_err(|error| ProjectError::InternalError(error.to_string()))?,
            );
            Ok(response)
        }
        ObjectStore::Fs(_) => {
            // New uploads land at {iso_storage_key}/full.iso; installs that
            // predate the upload pipeline kept the built CD as game.iso.
            let base = state.project_demo_config.dir.join(storage_key);
            let path = if base.join("full.iso").is_file() {
                base.join("full.iso")
            } else {
                base.join("game.iso")
            };
            let file = tokio::fs::File::open(path)
                .await
                .map_err(|_| ProjectError::ProjectNotFound)?;
            let size = file.metadata().await?.len();
            let mut response =
                Response::new(axum::body::Body::from_stream(ReaderStream::new(file)));
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/octet-stream"),
            );
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            );
            response.headers_mut().insert(
                header::CONTENT_LENGTH,
                HeaderValue::from_str(&size.to_string())
                    .map_err(|error| ProjectError::InternalError(error.to_string()))?,
            );
            response.headers_mut().insert(
                header::ETAG,
                HeaderValue::from_str(&format!("\"{sha256}\""))
                    .map_err(|error| ProjectError::InternalError(error.to_string()))?,
            );
            Ok(response)
        }
    }
}
