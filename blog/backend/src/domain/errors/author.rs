//! Errors for the author API.
//!
//! Unlike the dashboard aggregates (which answer plain text), these responses
//! are JSON with a `message` field: the callers are scripts, and the frontend
//! API client already reads `data.message` when a request fails.
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug)]
pub enum AuthorError {
    /// Missing, malformed, revoked or unknown key.
    InvalidKey,
    /// The key had a validity window and it has passed.
    KeyExpired,
    /// Right key, wrong book, insufficient scope, or the owner lost the
    /// authoring role. Deliberately a 404 in the wrong-book case so a key
    /// cannot enumerate which other books exist.
    NotFound,
    /// The key authenticated but may not perform this operation.
    Forbidden(String),
    Validation(String),
    /// `X-Content-SHA256` did not match the bytes actually received.
    HashMismatch {
        expected: String,
        actual: String,
    },
    /// A batch archive would exceed the configured byte cap.
    ArchiveTooLarge(String),
    InternalError(String),
}

impl From<sqlx::Error> for AuthorError {
    fn from(error: sqlx::Error) -> Self {
        AuthorError::InternalError(error.to_string())
    }
}

impl From<std::io::Error> for AuthorError {
    fn from(error: std::io::Error) -> Self {
        AuthorError::InternalError(error.to_string())
    }
}

impl AuthorError {
    fn parts(self) -> (StatusCode, String) {
        match self {
            AuthorError::InvalidKey => (
                StatusCode::UNAUTHORIZED,
                "Invalid or revoked author key.".to_string(),
            ),
            AuthorError::KeyExpired => (
                StatusCode::UNAUTHORIZED,
                "This author key has expired; issue a new one.".to_string(),
            ),
            AuthorError::NotFound => (StatusCode::NOT_FOUND, "Not found.".to_string()),
            AuthorError::Forbidden(message) => (StatusCode::FORBIDDEN, message),
            AuthorError::Validation(message) => (StatusCode::BAD_REQUEST, message),
            AuthorError::HashMismatch { expected, actual } => (
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("Hash mismatch: expected {expected}, received {actual}."),
            ),
            AuthorError::ArchiveTooLarge(message) => (StatusCode::PAYLOAD_TOO_LARGE, message),
            AuthorError::InternalError(message) => {
                tracing::error!("author API internal error: {message}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                )
            }
        }
    }
}

impl IntoResponse for AuthorError {
    fn into_response(self) -> Response {
        let (status, message) = self.parts();
        (status, Json(json!({ "message": message }))).into_response()
    }
}
