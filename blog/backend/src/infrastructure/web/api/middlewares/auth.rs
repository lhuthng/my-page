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
    let row = sqlx::query(
        "SELECT id, mode, expires_at, revoked_at FROM sync_keys WHERE token_hash = ?",
    )
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
        .map_err(|_| crate::domain::errors::sync::SyncError::InternalError("Unparseable key expiry.".to_string()))?;
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
