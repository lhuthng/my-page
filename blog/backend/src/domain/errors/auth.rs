use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tracing::error;

pub enum AuthError {
    UserAlreadyExists,
    InvalidCredentials,
    InvalidToken,
    ExpiredToken,
    EmailNotVerified { email_sent: bool },
    ResendTooSoon,
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
        let (status, body) = match self {
            AuthError::UserAlreadyExists => {
                (StatusCode::CONFLICT, "User already exists".to_string())
            }
            AuthError::InvalidCredentials => {
                (StatusCode::UNAUTHORIZED, "Invalid credentials".to_string())
            }
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid tokens".to_string()),
            AuthError::ExpiredToken => (StatusCode::UNAUTHORIZED, "Expired tokens".to_string()),
            AuthError::EmailNotVerified { email_sent } => (
                StatusCode::FORBIDDEN,
                if email_sent {
                    "Email not verified. A fresh verification link has been sent.".to_string()
                } else {
                    "Email not verified. Check your inbox or request a new verification link."
                        .to_string()
                },
            ),
            AuthError::ResendTooSoon => (
                StatusCode::TOO_MANY_REQUESTS,
                "Please wait at least one minute before requesting another verification email."
                    .to_string(),
            ),
            AuthError::Validation(msg) => (StatusCode::BAD_REQUEST, msg.to_string()),
            AuthError::InternalError(msg) => {
                error!("Internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                )
            }
        };

        (status, body).into_response()
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
