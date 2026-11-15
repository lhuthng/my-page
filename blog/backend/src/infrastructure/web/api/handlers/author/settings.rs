//! The per-book switch that exposes (or hides) this audiobook's API.
//!
//! Session-guarded and owner-only, like key issuance. Turning it off does not
//! delete keys — it makes them inert, so switching back on restores the same
//! credentials instead of forcing every script to be reconfigured.
use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, State},
};
use sqlx::Row;

use crate::{
    domain::{entities::secret::Claims, errors::author::AuthorError},
    infrastructure::web::{
        api::handlers::author::{
            dto::TogglePayload,
            response::SettingsResponse,
            shared::{assert_book_owner, caller_id, pool},
        },
        server::AppState,
    },
};

async fn read_settings(
    pool: &sqlx::SqlitePool,
    audiobook_id: i64,
) -> Result<SettingsResponse, AuthorError> {
    let row = sqlx::query("SELECT author_api_enabled FROM audiobooks WHERE id = ?")
        .bind(audiobook_id)
        .fetch_optional(pool)
        .await?
        .ok_or(AuthorError::NotFound)?;

    let key_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM author_api_keys WHERE audiobook_id = ?")
            .bind(audiobook_id)
            .fetch_one(pool)
            .await?;
    let active_key_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM author_api_keys
         WHERE audiobook_id = ? AND revoked_at IS NULL
           AND (expires_at IS NULL OR expires_at > CURRENT_TIMESTAMP)",
    )
    .bind(audiobook_id)
    .fetch_one(pool)
    .await?;

    Ok(SettingsResponse {
        enabled: row.get::<i64, _>("author_api_enabled") != 0,
        key_count,
        active_key_count,
    })
}

pub async fn get_settings(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(audiobook_id): Path<i64>,
) -> Result<Json<SettingsResponse>, AuthorError> {
    let db = pool(&state);
    assert_book_owner(db, audiobook_id, caller_id(&claims)?).await?;
    Ok(Json(read_settings(db, audiobook_id).await?))
}

pub async fn put_settings(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(audiobook_id): Path<i64>,
    Json(payload): Json<TogglePayload>,
) -> Result<Json<SettingsResponse>, AuthorError> {
    let db = pool(&state);
    assert_book_owner(db, audiobook_id, caller_id(&claims)?).await?;

    sqlx::query("UPDATE audiobooks SET author_api_enabled = ? WHERE id = ?")
        .bind(i64::from(payload.enabled))
        .bind(audiobook_id)
        .execute(db)
        .await?;

    tracing::info!(
        audiobook_id,
        enabled = payload.enabled,
        "author API toggled"
    );
    Ok(Json(read_settings(db, audiobook_id).await?))
}
