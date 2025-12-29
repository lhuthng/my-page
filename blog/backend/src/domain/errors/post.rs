use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tracing::error;

pub enum PostError {
    PostNotFound,
    Forbidden,
    InvalidPostContent,
    UploadFailed(String),
    InternalError(String),
}

impl From<String> for PostError {
    fn from(s: String) -> Self {
        PostError::InternalError(s)
    }
}

impl IntoResponse for PostError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            PostError::PostNotFound => (StatusCode::NOT_FOUND, "Post not found".to_string()),
            PostError::Forbidden => (
                StatusCode::FORBIDDEN,
                "You do not have permission to perform this action".to_string(),
            ),
            PostError::InvalidPostContent => {
                (StatusCode::BAD_REQUEST, "Invalid post content.".to_string())
            }
            PostError::UploadFailed(msg) => (StatusCode::BAD_REQUEST, msg),
            PostError::InternalError(msg) => {
                if cfg!(debug_assertions) {
                    (StatusCode::INTERNAL_SERVER_ERROR, msg)
                } else {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Internal Server Error".to_string(),
                    )
                }
            }
        };

        (status, body).into_response()
    }
}

impl From<sqlx::Error> for PostError {
    fn from(err: sqlx::Error) -> Self {
        PostError::InternalError(err.to_string())
    }
}
