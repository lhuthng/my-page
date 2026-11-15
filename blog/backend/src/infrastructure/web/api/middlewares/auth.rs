use std::sync::Arc;

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::IntoResponse,
};
use chrono::Utc;

use crate::{
    domain::{
        entities::{secret::Claims, user::UserRole},
        errors::{auth::AuthError, user::UserError},
    },
    infrastructure::web::{api::secrets::decode_from_jwt_token, server::AppState},
};

#[axum::debug_middleware]
pub async fn user_guard(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<impl IntoResponse, AuthError> {
    let token = request
        .headers()
        .get("Authorization")
        .and_then(|header_value| header_value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(AuthError::InvalidToken)?;

    let claims = decode_from_jwt_token(
        token.to_string(),
        &state.config.auth.algorithm,
        &state.config.auth.decoding_key,
    )
    .await?;

    if Utc::now().timestamp() as usize > claims.exp {
        return Err(AuthError::ExpiredToken);
    }

    if !claims.email_verified {
        return Err(AuthError::EmailNotVerified { email_sent: false });
    }

    request.extensions_mut().insert(claims);
    Ok(next.run(request).await)
}

#[axum::debug_middleware]
pub async fn optional_user_guard(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<impl IntoResponse, AuthError> {
    let mut opt_claims: Option<Claims> = None;
    if let Some(header_value) = request.headers().get("Authorization") {
        if let Some(token) = header_value
            .to_str()
            .ok()
            .and_then(|value| value.strip_prefix("Bearer "))
        {
            let claims = decode_from_jwt_token(
                token.to_string(),
                &state.config.auth.algorithm,
                &state.config.auth.decoding_key,
            )
            .await?;

            if Utc::now().timestamp() as usize > claims.exp {
                return Err(AuthError::ExpiredToken);
            }
            if !claims.email_verified {
                return Err(AuthError::EmailNotVerified { email_sent: false });
            }
            opt_claims = Some(claims)
        } else {
            return Err(AuthError::InvalidToken);
        }
    }

    request.extensions_mut().insert(opt_claims);

    Ok(next.run(request).await)
}

#[axum::debug_middleware]
pub async fn mod_check(request: Request, next: Next) -> Result<impl IntoResponse, UserError> {
    let claims = request
        .extensions()
        .get::<Claims>()
        .ok_or(UserError::InternalError("No claims found.".to_string()))?;
    let role = UserRole::try_from(claims.role.clone())
        .map_err(|_| UserError::InvalidData("Invalid role".to_string()))?;
    if UserRole::Moderator.include(&role) {
        Ok(next.run(request).await)
    } else {
        Err(UserError::Unauthorized)
    }
}

#[axum::debug_middleware]
pub async fn admin_check(request: Request, next: Next) -> Result<impl IntoResponse, UserError> {
    let claims = request
        .extensions()
        .get::<Claims>()
        .ok_or(UserError::InternalError("No claims found.".to_string()))?;
    let role = UserRole::try_from(claims.role.clone())
        .map_err(|_| UserError::InvalidData("Invalid role".to_string()))?;
    if UserRole::Admin.include(&role) {
        Ok(next.run(request).await)
    } else {
        Err(UserError::Unauthorized)
    }
}

/// Authenticates /sync requests via a sync key (`Authorization: Bearer bsk_…`)
/// instead of a JWT. Keys are pull-only by design; expired and revoked keys
/// are rejected and the last-use timestamp is recorded for auditing.
#[axum::debug_middleware]
pub async fn sync_key_guard(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<impl IntoResponse, crate::domain::errors::sync::SyncError> {
    use sqlx::Row;

    let token = request
        .headers()
        .get("Authorization")
        .and_then(|header_value| header_value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(crate::domain::errors::sync::SyncError::InvalidKey)?;

    let token_hash = crate::infrastructure::sync::hash_sync_key(token);
    let row =
        sqlx::query("SELECT id, mode, expires_at, revoked_at FROM sync_keys WHERE token_hash = ?")
            .bind(&token_hash)
            .fetch_optional(&state.project_service.pool)
            .await
            .map_err(|e| crate::domain::errors::sync::SyncError::InternalError(e.to_string()))?
            .ok_or(crate::domain::errors::sync::SyncError::InvalidKey)?;

    if row.get::<String, _>("mode") != "pull" {
        return Err(crate::domain::errors::sync::SyncError::ForbiddenMode);
    }
    if row.get::<Option<String>, _>("revoked_at").is_some() {
        return Err(crate::domain::errors::sync::SyncError::InvalidKey);
    }
    let expires_at = row.get::<String, _>("expires_at");
    let expires = chrono::DateTime::parse_from_rfc3339(&expires_at)
        .map(|parsed| parsed.with_timezone(&Utc))
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(&expires_at, "%Y-%m-%d %H:%M:%S")
                .map(|naive| chrono::DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
        })
        .map_err(|_| {
            crate::domain::errors::sync::SyncError::InternalError(
                "Unparseable key expiry.".to_string(),
            )
        })?;
    if expires < Utc::now() {
        return Err(crate::domain::errors::sync::SyncError::KeyExpired);
    }

    let _ = sqlx::query("UPDATE sync_keys SET last_used_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(row.get::<i64, _>("id"))
        .execute(&state.project_service.pool)
        .await;

    request.extensions_mut().insert(SyncKeyAuth {
        key_id: row.get::<i64, _>("id"),
    });
    Ok(next.run(request).await)
}

/// Marker inserted by [`sync_key_guard`] once a key has been validated.
#[derive(Clone, Copy)]
pub struct SyncKeyAuth {
    pub key_id: i64,
}

/// Identity carried by a validated `abk_…` author key. The book id is part of
/// the identity, not a value the handler is free to choose: see
/// `handlers::author::shared::assert_key_scope`.
#[derive(Clone)]
pub struct AuthorKeyAuth {
    pub key_id: i64,
    pub audiobook_id: i64,
    pub user_id: i64,
    pub role: String,
    pub scopes: String,
}

/// Authenticates `/author` requests with a book-scoped key.
///
/// One query joins the key, its book and its owner. That single read is also
/// where the four scope checks live: revoked, still owned by the issuer,
/// authoring role, and read-vs-write. A key is deliberately *narrower* than
/// the session that issued it — role and ownership are re-read on every
/// request, so a demotion or a reassignment takes effect immediately rather
/// than at key expiry.
#[axum::debug_middleware]
pub async fn author_key_guard(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<impl IntoResponse, crate::domain::errors::author::AuthorError> {
    use crate::domain::errors::author::AuthorError;
    use crate::infrastructure::author_keys::hash_author_key;
    use axum::http::Method;
    use chrono::{DateTime, NaiveDateTime};
    use sqlx::Row;

    let token = request
        .headers()
        .get("Authorization")
        .and_then(|header_value| header_value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(AuthorError::InvalidKey)?;

    let row = sqlx::query(
        r#"
        SELECT k.id, k.audiobook_id, k.user_id, k.scopes, k.expires_at, k.revoked_at,
               a.user_id AS book_owner, a.author_api_enabled, u.role
        FROM author_api_keys k
        JOIN audiobooks a ON a.id = k.audiobook_id
        JOIN users u ON u.id = k.user_id
        WHERE k.token_hash = ?
        "#,
    )
    .bind(hash_author_key(token))
    .fetch_optional(&state.audiobook_service.pool)
    .await?
    .ok_or(AuthorError::InvalidKey)?;

    if row.get::<Option<String>, _>("revoked_at").is_some() {
        return Err(AuthorError::InvalidKey);
    }
    if row.get::<i64, _>("book_owner") != row.get::<i64, _>("user_id") {
        return Err(AuthorError::InvalidKey);
    }
    // The per-book switch. A book whose API is off is indistinguishable from a
    // book that does not exist, so a key cannot be used to discover which books
    // have the API enabled.
    if row.get::<i64, _>("author_api_enabled") == 0 {
        return Err(AuthorError::NotFound);
    }

    let role: String = row.get("role");
    if role != "moderator" && role != "admin" {
        return Err(AuthorError::Forbidden(
            "Authoring requires a moderator or admin account.".to_string(),
        ));
    }

    // NULL means "until revoked". Accept both the RFC3339 spelling we write
    // and SQLite's CURRENT_TIMESTAMP, as sync_key_guard does.
    if let Some(expires_at) = row.get::<Option<String>, _>("expires_at") {
        let expires = DateTime::parse_from_rfc3339(&expires_at)
            .map(|parsed| parsed.with_timezone(&Utc))
            .or_else(|_| {
                NaiveDateTime::parse_from_str(&expires_at, "%Y-%m-%d %H:%M:%S")
                    .map(|naive| DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
            })
            .map_err(|_| AuthorError::InternalError("Unparseable key expiry.".to_string()))?;
        if expires < Utc::now() {
            return Err(AuthorError::KeyExpired);
        }
    }

    let scopes: String = row.get("scopes");
    let method = request.method().clone();
    let read_only = method == Method::GET || method == Method::HEAD;
    if !read_only && scopes != "read,write" {
        return Err(AuthorError::Forbidden("This key is read-only.".to_string()));
    }

    let key_id = row.get::<i64, _>("id");
    let _ = sqlx::query("UPDATE author_api_keys SET last_used_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(key_id)
        .execute(&state.audiobook_service.pool)
        .await;

    request.extensions_mut().insert(AuthorKeyAuth {
        key_id,
        audiobook_id: row.get("audiobook_id"),
        user_id: row.get("user_id"),
        role,
        scopes,
    });
    Ok(next.run(request).await)
}
