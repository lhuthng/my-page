use axum::http::{HeaderValue, StatusCode, header::RETRY_AFTER};
use axum::response::{IntoResponse, Response};
use tracing::error;

pub enum AuthError {
    UserAlreadyExists,
    InvalidCredentials,
    InvalidToken,
    ExpiredToken,
    EmailNotVerified { email_sent: bool },
    ResendTooSoon,
    CaptchaRequired,
    CaptchaFailed,
    CaptchaUnavailable,
    RateLimited { retry_after_secs: u64 },
    Validation(String),
    InternalError(String),
}

impl From<String> for AuthError {
    fn from(s: String) -> Self {
        AuthError::InternalError(s.to_string())
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, body, retry_after) = match self {
            AuthError::UserAlreadyExists => (
                StatusCode::CONFLICT,
                "User already exists".to_string(),
                None,
            ),
            AuthError::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "Invalid credentials".to_string(),
                None,
            ),
            AuthError::InvalidToken => {
                (StatusCode::UNAUTHORIZED, "Invalid tokens".to_string(), None)
            }
            AuthError::ExpiredToken => {
                (StatusCode::UNAUTHORIZED, "Expired tokens".to_string(), None)
            }
            // 403 is reserved for "credentials are right, email is not verified"
            // so the login form can tell it apart by status alone.
            AuthError::EmailNotVerified { email_sent } => (
                StatusCode::FORBIDDEN,
                if email_sent {
                    "Email not verified. A fresh verification link has been sent.".to_string()
                } else {
                    "Email not verified. Check your inbox or request a new verification link."
                        .to_string()
                },
                None,
            ),
            AuthError::ResendTooSoon => (
                StatusCode::TOO_MANY_REQUESTS,
                "Please wait at least one minute before requesting another verification email."
                    .to_string(),
                Some(60),
            ),
            AuthError::CaptchaRequired => (
                StatusCode::BAD_REQUEST,
                "Captcha is required.".to_string(),
                None,
            ),
            AuthError::CaptchaFailed => (
                StatusCode::BAD_REQUEST,
                "Captcha verification failed. Please try again.".to_string(),
                None,
            ),
            AuthError::CaptchaUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Captcha verification is unavailable. Please try again later.".to_string(),
                None,
            ),
            AuthError::RateLimited { retry_after_secs } => (
                StatusCode::TOO_MANY_REQUESTS,
                format!("Too many attempts. Try again in {retry_after_secs} seconds."),
                Some(retry_after_secs),
            ),
            AuthError::Validation(msg) => (StatusCode::BAD_REQUEST, msg.to_string(), None),
            AuthError::InternalError(msg) => {
                error!("Internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                    None,
                )
            }
        };

        let mut response = (status, body).into_response();
        if let Some(seconds) = retry_after
            && let Ok(value) = HeaderValue::from_str(&seconds.to_string())
        {
            response.headers_mut().insert(RETRY_AFTER, value);
        }
        response
    }
}

impl From<sqlx::Error> for AuthError {
    fn from(err: sqlx::Error) -> Self {
        AuthError::InternalError(err.to_string())
    }
}

impl From<bcrypt::BcryptError> for AuthError {
    fn from(err: bcrypt::BcryptError) -> Self {
        AuthError::InternalError(err.to_string())
    }
}
