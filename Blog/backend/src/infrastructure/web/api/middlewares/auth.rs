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
    let access_token = request
        .headers()
        .get("Authorization")
        .and_then(|header_value| header_value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(AuthError::InvalidToken);

    let token = access_token?;

    let claims = decode_from_jwt_token(
        token.to_string(),
        &state.config.auth.algorithm,
        &state.config.auth.decoding_key,
    )
    .await?;

    if Utc::now().timestamp() as usize > claims.exp {
        return Err(AuthError::ExpiredToken);
    }

    request.extensions_mut().insert(claims);
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
