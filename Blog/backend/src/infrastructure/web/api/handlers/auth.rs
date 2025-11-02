use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use axum_extra::extract::cookie::{Cookie, CookieJar};

use serde::Serialize;

use crate::{
    application::{
        commands::auth::{LoginCommand, RefreshTokenCommand, RegisterCommand},
        services::auth::AuthService,
    },
    domain::errors::auth::AuthError,
    infrastructure::web::server::AppState,
};

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    access_token: String,
    token_type: String,
}

impl LoginResponse {
    fn new(access_token: String) -> Self {
        Self {
            access_token,
            token_type: "Bearer".to_string(),
        }
    }
}

#[axum::debug_handler]
pub async fn login(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Json(payload): Json<LoginCommand>,
) -> Result<impl IntoResponse, AuthError> {
    match state
        .auth_service
        .login(payload, state.config.auth.clone())
        .await
    {
        Ok(auth_tokens) => {
            let jar = jar.add(
                Cookie::build(("refresh_token", auth_tokens.refresh_token))
                    .http_only(true)
                    .secure(true)
                    .path("/")
                    .build(),
            );
            let body = Json(LoginResponse::new(auth_tokens.access_token));
            Ok((jar, (StatusCode::OK, body)))
        }
        Err(e) => Err(e),
    }
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    message: String,
}

#[axum::debug_handler]
pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterCommand>,
) -> Result<Json<RegisterResponse>, AuthError> {
    match state.auth_service.register(payload).await {
        Ok(_) => Ok(Json(RegisterResponse {
            message: "User registered.".to_string(),
        })),
        Err(e) => Err(e),
    }
}

#[derive(Debug, Serialize)]
pub struct RefreshTokenResponse {
    access_token: String,
    refresh_token: String,
    token_type: String,
}

impl RefreshTokenResponse {
    fn new(access_token: String, refresh_token: String) -> Self {
        Self {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
        }
    }
}

#[axum::debug_handler]
pub async fn refresh_token(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Json(payload): Json<RefreshTokenCommand>,
) -> Result<impl IntoResponse, AuthError> {
    let refresh_token = jar
        .get("refresh_token")
        .ok_or(AuthError::InvalidToken)?
        .value()
        .to_string();

    match state
        .auth_service
        .refresh_token(payload, refresh_token, state.config.auth.clone())
        .await
    {
        Ok(auth_tokens) => {
            let jar = jar.add(
                Cookie::build(("refresh_token", auth_tokens.refresh_token))
                    .http_only(true)
                    .secure(true)
                    .path("/")
                    .build(),
            );
            let body = Json(LoginResponse::new(auth_tokens.access_token));
            Ok((jar, (StatusCode::OK, body)))
        }
        Err(e) => Err(e),
    }
}
