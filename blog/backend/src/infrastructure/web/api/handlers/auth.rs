use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use axum_extra::extract::cookie::{Cookie, CookieJar};

use serde::Serialize;
use validator::Validate;

use crate::{
    application::{
        commands::auth::{LoginCommand, RefreshAccessTokenCommand, RegisterCommand},
        services::auth::AuthService,
    },
    domain::{entities::auth::RegisterCredentials, errors::auth::AuthError},
    infrastructure::web::server::AppState,
};

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    token: String,
    token_type: String,
}

impl LoginResponse {
    fn new(access_token: String) -> Self {
        Self {
            token: access_token,
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
                Cookie::build(("refresh-token", auth_tokens.refresh_token))
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
    if payload.username == "me" {
        return Err(AuthError::Validation(
            "\"me\" cannot be a username".to_string(),
        ));
    }

    let reg_creds = RegisterCredentials {
        username: payload.username,
        password: payload.password,
        email: payload.email,
    };

    if let Err(err) = reg_creds.validate() {
        return Err(AuthError::Validation(err.to_string()));
    }

    match state.auth_service.register(reg_creds).await {
        Ok(_) => Ok(Json(RegisterResponse {
            message: "User registered.".to_string(),
        })),
        Err(e) => Err(e),
    }
}

#[axum::debug_handler]
pub async fn refresh_token(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AuthError> {
    let refresh_token = jar
        .get("refresh-token")
        .ok_or(AuthError::InvalidToken)?
        .value()
        .to_string();

    let cmd = RefreshAccessTokenCommand {
        refresh_token: refresh_token,
    };

    match state
        .auth_service
        .refresh_access_token(cmd, state.config.auth.clone())
        .await
    {
        Ok(auth_tokens) => {
            let jar = jar.add(
                Cookie::build(("refresh-token", auth_tokens.refresh_token))
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
