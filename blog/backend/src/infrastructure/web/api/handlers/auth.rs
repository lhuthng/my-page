use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};

use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    application::{
        commands::auth::{
            LoginCommand, RefreshAccessTokenCommand, RegisterCommand, RequestPasswordResetCommand,
            ResendVerificationCommand, ResetPasswordCommand, VerifyEmailCommand,
        },
        services::auth::AuthService,
    },
    domain::{
        entities::auth::{
            LoginResult, RegisterCredentials, RequestPasswordResetResult, ResendVerificationResult,
        },
        errors::auth::AuthError,
    },
    infrastructure::{
        mail::{send_password_reset_email, send_verification_email},
        web::server::AppState,
    },
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
        Ok(LoginResult::Authenticated(auth_tokens)) => {
            let jar = jar.add(
                Cookie::build(("refresh-token", auth_tokens.refresh_token))
                    .http_only(true)
                    .secure(true)
                    .same_site(SameSite::Lax)
                    .path("/")
                    .build(),
            );
            let body = Json(LoginResponse::new(auth_tokens.access_token));
            Ok((jar, (StatusCode::OK, body)))
        }
        Ok(LoginResult::VerificationRequired { verification_mail }) => {
            if let Some(mail) = verification_mail {
                let mail_config = state.config.mail.as_ref().ok_or_else(|| {
                    AuthError::InternalError("Mail is not configured on the server".to_string())
                })?;
                send_verification_email(mail_config, &state.config.app_base_url, &mail)
                    .await
                    .map_err(AuthError::InternalError)?;
                Err(AuthError::EmailNotVerified { email_sent: true })
            } else {
                Err(AuthError::EmailNotVerified { email_sent: false })
            }
        }
        Err(e) => Err(e),
    }
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    message: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyEmailResponse {
    message: String,
}

#[derive(Debug, Serialize)]
pub struct PasswordResetResponse {
    message: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyEmailQuery {
    token: String,
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
        Ok(result) => {
            let mail_config = state.config.mail.as_ref().ok_or_else(|| {
                AuthError::InternalError("Mail is not configured on the server".to_string())
            })?;

            send_verification_email(
                mail_config,
                &state.config.app_base_url,
                &result.verification_mail,
            )
            .await
            .map_err(AuthError::InternalError)?;

            Ok(Json(RegisterResponse {
                message: "User registered. A verification link has been sent to your email."
                    .to_string(),
            }))
        }
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

    let cmd = RefreshAccessTokenCommand { refresh_token };

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
                    .same_site(SameSite::Lax)
                    .path("/")
                    .build(),
            );
            let body = Json(LoginResponse::new(auth_tokens.access_token));
            Ok((jar, (StatusCode::OK, body)))
        }
        Err(e) => Err(e),
    }
}

#[axum::debug_handler]
pub async fn verify_email(
    State(state): State<Arc<AppState>>,
    Query(query): Query<VerifyEmailQuery>,
) -> Result<Json<VerifyEmailResponse>, AuthError> {
    state
        .auth_service
        .verify_email(VerifyEmailCommand { token: query.token })
        .await?;

    Ok(Json(VerifyEmailResponse {
        message: "Email verified. You can log in now.".to_string(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct ResendVerificationPayload {
    identifier: String,
}

#[axum::debug_handler]
pub async fn resend_verification(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ResendVerificationPayload>,
) -> Result<Json<VerifyEmailResponse>, AuthError> {
    let result = state
        .auth_service
        .resend_verification(ResendVerificationCommand {
            identifier: payload.identifier,
        })
        .await?;

    match result {
        ResendVerificationResult::VerificationMailQueued(mail) => {
            let mail_config = state.config.mail.as_ref().ok_or_else(|| {
                AuthError::InternalError("Mail is not configured on the server".to_string())
            })?;
            send_verification_email(mail_config, &state.config.app_base_url, &mail)
                .await
                .map_err(AuthError::InternalError)?;
            Ok(Json(VerifyEmailResponse {
                message: "A fresh verification email has been sent.".to_string(),
            }))
        }
        ResendVerificationResult::AlreadyVerified => Ok(Json(VerifyEmailResponse {
            message: "This account is already verified.".to_string(),
        })),
        ResendVerificationResult::UserNotFound => Ok(Json(VerifyEmailResponse {
            message: "If the account exists, a new verification email is on the way.".to_string(),
        })),
    }
}

#[derive(Debug, Deserialize)]
pub struct RequestPasswordResetPayload {
    username: String,
    email: String,
}

#[axum::debug_handler]
pub async fn request_password_reset(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RequestPasswordResetPayload>,
) -> Result<Json<PasswordResetResponse>, AuthError> {
    let result = state
        .auth_service
        .request_password_reset(RequestPasswordResetCommand {
            username: payload.username,
            email: payload.email,
        })
        .await?;

    if let RequestPasswordResetResult::ResetMailQueued(mail) = result {
        let mail_config = state.config.mail.as_ref().ok_or_else(|| {
            AuthError::InternalError("Mail is not configured on the server".to_string())
        })?;
        send_password_reset_email(mail_config, &state.config.app_base_url, &mail)
            .await
            .map_err(AuthError::InternalError)?;
    }

    Ok(Json(PasswordResetResponse {
        message: "If the username and email match an account, a reset link has been sent."
            .to_string(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordPayload {
    token: String,
    password: String,
}

#[axum::debug_handler]
pub async fn reset_password(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ResetPasswordPayload>,
) -> Result<Json<PasswordResetResponse>, AuthError> {
    state
        .auth_service
        .reset_password(ResetPasswordCommand {
            token: payload.token,
            password: payload.password,
        })
        .await?;

    Ok(Json(PasswordResetResponse {
        message: "Password changed. You can log in with the new password now.".to_string(),
    }))
}
