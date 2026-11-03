use std::sync::Arc;
use std::time::Duration;

use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use cookie::time::Duration as CookieDuration;

use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    application::{
        commands::auth::{
            LoginCommand, LogoutCommand, RefreshAccessTokenCommand, RequestPasswordResetCommand,
            ResendVerificationCommand, ResetPasswordCommand,
            VerifyEmailCommand,
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
        web::{api::security, server::AppState},
    },
};

const LOGIN_IP_LIMIT: (usize, Duration) = (10, Duration::from_secs(10 * 60));
const LOGIN_IDENTIFIER_LIMIT: (usize, Duration) = (5, Duration::from_secs(15 * 60));
const REGISTER_IP_LIMIT: (usize, Duration) = (5, Duration::from_secs(60 * 60));
const REGISTER_IDENTIFIER_LIMIT: (usize, Duration) = (3, Duration::from_secs(60 * 60));
const RESEND_IP_LIMIT: (usize, Duration) = (5, Duration::from_secs(10 * 60));
const RESEND_IDENTIFIER_LIMIT: (usize, Duration) = (3, Duration::from_secs(10 * 60));
const FORGOT_IP_LIMIT: (usize, Duration) = (5, Duration::from_secs(60 * 60));
const FORGOT_IDENTIFIER_LIMIT: (usize, Duration) = (3, Duration::from_secs(60 * 60));
const RESET_IP_LIMIT: (usize, Duration) = (10, Duration::from_secs(60 * 60));
const RESET_TOKEN_LIMIT: (usize, Duration) = (5, Duration::from_secs(60 * 60));
const VERIFY_IP_LIMIT: (usize, Duration) = (20, Duration::from_secs(10 * 60));

/// Verify the Turnstile token on this request. Development without a secret
/// skips it; a release build without one fails closed.
async fn verify_captcha(
    state: &AppState,
    headers: &HeaderMap,
    token: Option<&str>,
) -> Result<(), AuthError> {
    let remote_ip = security::client_ip(headers);
    security::verify_turnstile(
        &state.http_client,
        state.config.security.turnstile_secret.as_deref(),
        remote_ip.as_deref(),
        token,
    )
    .await
}

fn enforce(
    state: &AppState,
    key: String,
    (max_requests, window): (usize, Duration),
) -> Result<(), AuthError> {
    state.auth_rate_limiter.check(&key, max_requests, window)
}

/// A persistent cookie only when "remember me" was chosen; otherwise the
/// cookie is a session cookie and dies with the browser.
fn refresh_cookie(token: String, max_age: Option<CookieDuration>) -> Cookie<'static> {
    let mut builder = Cookie::build(("refresh-token", token))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .path("/");

    if let Some(max_age) = max_age {
        builder = builder.max_age(max_age);
    }

    builder.build()
}

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

#[derive(Debug, Deserialize)]
pub struct LoginPayload {
    username: String,
    password: String,
    #[serde(default)]
    remember: bool,
    #[serde(default, alias = "captchaToken")]
    captcha_token: Option<String>,
}

#[axum::debug_handler]
pub async fn login(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(payload): Json<LoginPayload>,
) -> Result<impl IntoResponse, AuthError> {
    verify_captcha(&state, &headers, payload.captcha_token.as_deref()).await?;

    enforce(&state, security::client_key("login", &headers), LOGIN_IP_LIMIT)?;
    enforce(
        &state,
        security::identifier_key("login", &payload.username),
        LOGIN_IDENTIFIER_LIMIT,
    )?;

    let remember = payload.remember;
    let command = LoginCommand {
        username: payload.username,
        password: payload.password,
        remember,
    };

    match state
        .auth_service
        .login(command, state.config.auth.clone())
        .await
    {
        Ok(LoginResult::Authenticated(auth_tokens)) => {
            let max_age = remember
                .then(|| CookieDuration::hours(state.config.auth.remember_expire_hours));
            let jar = jar.add(refresh_cookie(auth_tokens.refresh_token, max_age));
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

#[derive(Debug, Deserialize)]
pub struct RegisterPayload {
    username: String,
    password: String,
    email: String,
    #[serde(default, alias = "captchaToken")]
    captcha_token: Option<String>,
}

#[axum::debug_handler]
pub async fn register(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<RegisterPayload>,
) -> Result<Json<RegisterResponse>, AuthError> {
    verify_captcha(&state, &headers, payload.captcha_token.as_deref()).await?;

    enforce(&state, security::client_key("register", &headers), REGISTER_IP_LIMIT)?;
    enforce(
        &state,
        security::identifier_key("register", &payload.email),
        REGISTER_IDENTIFIER_LIMIT,
    )?;

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
            // The browser already holds this cookie with whatever Max-Age the
            // login chose. Re-setting it here (without Max-Age) would turn a
            // "remember me" cookie back into a session cookie on first refresh.
            let body = Json(LoginResponse::new(auth_tokens.access_token));
            Ok(body)
        }
        Err(e) => Err(e),
    }
}

#[axum::debug_handler]
pub async fn logout(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AuthError> {
    // The refresh token is the session's credential, so possession of it is
    // what authorises ending the session. This runs before the cookie is
    // cleared and works even when the access token has already expired.
    if let Some(token) = jar.get("refresh-token") {
        state
            .auth_service
            .logout(LogoutCommand {
                refresh_token: token.value().to_string(),
            })
            .await?;
    }

    let jar = jar.remove(Cookie::build(("refresh-token", "")).path("/").build());
    Ok((jar, StatusCode::NO_CONTENT))
}

#[axum::debug_handler]
pub async fn verify_email(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<VerifyEmailQuery>,
) -> Result<Json<VerifyEmailResponse>, AuthError> {
    enforce(&state, security::client_key("verify", &headers), VERIFY_IP_LIMIT)?;

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
    #[serde(default, alias = "captchaToken")]
    captcha_token: Option<String>,
}

#[axum::debug_handler]
pub async fn resend_verification(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<ResendVerificationPayload>,
) -> Result<Json<VerifyEmailResponse>, AuthError> {
    verify_captcha(&state, &headers, payload.captcha_token.as_deref()).await?;

    enforce(&state, security::client_key("resend", &headers), RESEND_IP_LIMIT)?;
    enforce(
        &state,
        security::identifier_key("resend", &payload.identifier),
        RESEND_IDENTIFIER_LIMIT,
    )?;

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
    #[serde(default, alias = "captchaToken")]
    captcha_token: Option<String>,
}

#[axum::debug_handler]
pub async fn request_password_reset(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<RequestPasswordResetPayload>,
) -> Result<Json<PasswordResetResponse>, AuthError> {
    verify_captcha(&state, &headers, payload.captcha_token.as_deref()).await?;

    enforce(&state, security::client_key("forgot", &headers), FORGOT_IP_LIMIT)?;
    enforce(
        &state,
        security::identifier_key("forgot", &format!("{}|{}", payload.username, payload.email)),
        FORGOT_IDENTIFIER_LIMIT,
    )?;

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
    headers: HeaderMap,
    Json(payload): Json<ResetPasswordPayload>,
) -> Result<Json<PasswordResetResponse>, AuthError> {
    enforce(&state, security::client_key("reset", &headers), RESET_IP_LIMIT)?;
    enforce(
        &state,
        security::identifier_key("reset", &payload.token),
        RESET_TOKEN_LIMIT,
    )?;

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

// ---------------------------------------------------------------------------
// Route table (public only; the login flow mints its own tokens)
use axum::{
    Router,
    routing::{get, post},
};

pub fn routes(_state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/login", post(login))
        .route("/register", post(register))
        .route("/verify-email", get(verify_email))
        .route("/forgot-password", post(request_password_reset))
        .route("/reset-password", post(reset_password))
        .route("/resend-verification", post(resend_verification))
        .route("/refresh", post(refresh_token))
        .route("/logout", post(logout))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_login_keeps_a_session_cookie() {
        let cookie = refresh_cookie("token-value".to_string(), None).to_string();
        assert!(cookie.starts_with("refresh-token=token-value"));
        assert!(!cookie.contains("Max-Age"));
    }

    #[test]
    fn remember_me_sets_a_persistent_cookie() {
        let cookie =
            refresh_cookie("token-value".to_string(), Some(CookieDuration::hours(2160)))
                .to_string();
        assert!(cookie.contains("Max-Age=7776000"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("Secure"));
        assert!(cookie.contains("SameSite=Lax"));
    }
}
