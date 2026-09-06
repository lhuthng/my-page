use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub enum SyncError {
    /// Missing, malformed, revoked or unknown sync key.
    InvalidKey,
    /// The key exists but its validity window has passed.
    KeyExpired,
    /// The key's mode does not allow this operation (pull-only for now).
    ForbiddenMode,
    NotFound,
    InvalidData(String),
    IoError(std::io::Error),
    InternalError(String),
}

impl From<std::io::Error> for SyncError {
    fn from(e: std::io::Error) -> Self {
        SyncError::IoError(e)
    }
}

impl From<sqlx::Error> for SyncError {
    fn from(e: sqlx::Error) -> Self {
        SyncError::InternalError(e.to_string())
    }
}

impl IntoResponse for SyncError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            SyncError::InvalidKey => (
                StatusCode::UNAUTHORIZED,
                "Invalid or revoked sync key.".to_string(),
            ),
            SyncError::KeyExpired => (
                StatusCode::FORBIDDEN,
                "This sync key has expired; issue a new one.".to_string(),
            ),
            SyncError::ForbiddenMode => (
                StatusCode::FORBIDDEN,
                "This sync key does not allow the requested operation.".to_string(),
            ),
            SyncError::NotFound => (StatusCode::NOT_FOUND, "Not found.".to_string()),
            SyncError::InvalidData(msg) => (StatusCode::BAD_REQUEST, msg),
            SyncError::IoError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("IO error: {}", e),
            ),
            SyncError::InternalError(msg) => {
                if cfg!(debug_assertions) {
                    (StatusCode::INTERNAL_SERVER_ERROR, msg)
                } else {
                    tracing::error!("sync internal error: {msg}");
                    (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error".to_string())
                }
            }
        };
        (status, body).into_response()
    }
}
