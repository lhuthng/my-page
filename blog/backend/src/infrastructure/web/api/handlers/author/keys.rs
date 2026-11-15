//! Key management. Session-guarded, because you cannot need a key to get a key.
//!
//! The sync-key feature has three handlers for this in `handlers/sync.rs` that
//! were never mounted on any router. These routes are wired in `mod.rs` and
//! covered by a reachability test.
use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{Duration, Utc};
use sqlx::Row;

use crate::{
    domain::{entities::secret::Claims, errors::author::AuthorError},
    infrastructure::{
        author_keys::{author_key_hint, generate_author_key, hash_author_key},
        web::{
            api::handlers::author::{
                dto::CreateKeyPayload,
                response::{CreatedKeyResponse, KeyInfoResponse, KeyListResponse},
                shared::{assert_book_owner, caller_id, pool},
            },
            server::AppState,
        },
    },
};

const MAX_TTL_HOURS: u64 = 5 * 365 * 24;

pub async fn issue_key(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(audiobook_id): Path<i64>,
    Json(payload): Json<CreateKeyPayload>,
) -> Result<Json<CreatedKeyResponse>, AuthorError> {
    let user_id = caller_id(&claims)?;
    let db = pool(&state);
    assert_book_owner(db, audiobook_id, user_id).await?;

    // Issuing a key for a book whose API is off would mint a credential that
    // the guard immediately refuses — confusing at best. Make the author flip
    // the switch first.
    let enabled: i64 = sqlx::query_scalar("SELECT author_api_enabled FROM audiobooks WHERE id = ?")
        .bind(audiobook_id)
        .fetch_one(db)
        .await?;
    if enabled == 0 {
        return Err(AuthorError::Validation(
            "Turn the author API on for this book before issuing a key.".to_string(),
        ));
    }

    let label: String = payload
        .label
        .unwrap_or_default()
        .trim()
        .chars()
        .take(100)
        .collect();
    let scopes = payload.scopes.unwrap_or_else(|| "read,write".to_string());
    if scopes != "read" && scopes != "read,write" {
        return Err(AuthorError::Validation(
            "scopes must be 'read' or 'read,write'.".to_string(),
        ));
    }

    // NULL means "until revoked" and is the default. A supplied TTL is clamped
    // so a typo cannot mint a key that outlives the service by a millennium.
    let expires_at = payload
        .ttl_hours
        .map(|hours| hours.clamp(1, MAX_TTL_HOURS))
        .map(|hours| (Utc::now() + Duration::hours(hours as i64)).to_rfc3339());

    let key = generate_author_key();
    let hint = author_key_hint(&key);
    let result = sqlx::query(
        "INSERT INTO author_api_keys
         (audiobook_id, user_id, token_hash, label, hint, scopes, expires_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(audiobook_id)
    .bind(user_id)
    .bind(hash_author_key(&key))
    .bind(&label)
    .bind(&hint)
    .bind(&scopes)
    .bind(&expires_at)
    .execute(db)
    .await?;

    tracing::info!(
        key_id = result.last_insert_rowid(),
        audiobook_id,
        "author key issued"
    );
    Ok(Json(CreatedKeyResponse {
        id: result.last_insert_rowid(),
        label,
        hint,
        scopes,
        expires_at,
        key,
    }))
}

pub async fn list_keys(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(audiobook_id): Path<i64>,
) -> Result<Json<KeyListResponse>, AuthorError> {
    let user_id = caller_id(&claims)?;
    let db = pool(&state);
    assert_book_owner(db, audiobook_id, user_id).await?;

    let rows = sqlx::query(
        "SELECT id, label, hint, scopes, created_at, expires_at, revoked_at, last_used_at
         FROM author_api_keys WHERE audiobook_id = ? ORDER BY id DESC",
    )
    .bind(audiobook_id)
    .fetch_all(db)
    .await?;

    Ok(Json(KeyListResponse {
        keys: rows
            .iter()
            .map(|row| KeyInfoResponse {
                id: row.get("id"),
                label: row.get("label"),
                hint: row.get("hint"),
                scopes: row.get("scopes"),
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
                revoked_at: row.get("revoked_at"),
                last_used_at: row.get("last_used_at"),
            })
            .collect(),
    }))
}

pub async fn revoke_key(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path((audiobook_id, key_id)): Path<(i64, i64)>,
) -> Result<StatusCode, AuthorError> {
    let user_id = caller_id(&claims)?;
    let db = pool(&state);
    assert_book_owner(db, audiobook_id, user_id).await?;

    let result = sqlx::query(
        "UPDATE author_api_keys SET revoked_at = CURRENT_TIMESTAMP
         WHERE id = ? AND audiobook_id = ? AND revoked_at IS NULL",
    )
    .bind(key_id)
    .bind(audiobook_id)
    .execute(db)
    .await?;

    if result.rows_affected() == 0 {
        let exists: Option<i64> =
            sqlx::query_scalar("SELECT id FROM author_api_keys WHERE id = ? AND audiobook_id = ?")
                .bind(key_id)
                .bind(audiobook_id)
                .fetch_optional(db)
                .await?;
        return match exists {
            Some(_) => Ok(StatusCode::NO_CONTENT), // already revoked
            None => Err(AuthorError::NotFound),
        };
    }
    tracing::info!(key_id, audiobook_id, "author key revoked");
    Ok(StatusCode::NO_CONTENT)
}
