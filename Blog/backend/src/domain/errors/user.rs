use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tracing::error;

pub enum UserError {
    NotFound,
    AlreadyExists,
    InvalidData(String),
    Unauthorized,
    InternalError(String),
}

impl From<String> for UserError {
    fn from(s: String) -> Self {
        UserError::InternalError(s.to_string())
    }
}

impl IntoResponse for UserError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            UserError::NotFound => (StatusCode::NOT_FOUND, "User not found".to_string()),
            UserError::AlreadyExists => (StatusCode::CONFLICT, "User already exists".to_string()),
            UserError::InvalidData(msg) => (StatusCode::BAD_REQUEST, msg),
            UserError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized role".to_string()),
            UserError::InternalError(msg) => {
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
