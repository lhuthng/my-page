use std::sync::Arc;

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::IntoResponse,
};
use chrono::Utc;

use crate::{
    domain::errors::auth::AuthError,
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

    let token = match access_token {
        Ok(token) => token,
        Err(error) => return Err(error),
    };

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
