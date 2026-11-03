use axum::http::{HeaderValue, StatusCode, header::RETRY_AFTER};
use axum::response::{IntoResponse, Response};
use tracing::error;

use crate::domain::errors::media::MediaError;

#[derive(Debug)]
pub enum PostError {
    PostNotFound,
    TagNotFound,
    Forbidden,
    /// The client's `expected_updated_at` did not match the stored row: someone
    /// else saved in the meantime. Carries the current `updated_at` so the
    /// client can show the conflict and offer to reload or overwrite.
    Conflict(String),
    // InvalidPostContent,
    Validation(String),
    UploadFailed(String),
    RateLimited {
        retry_after_secs: u64,
    },
    InternalError(String),
    Media(MediaError),
}

impl From<String> for PostError {
    fn from(s: String) -> Self {
        PostError::InternalError(s)
    }
}

impl IntoResponse for PostError {
    fn into_response(self) -> Response {
        match self {
            PostError::Media(inner) => inner.into_response(),
            PostError::RateLimited { retry_after_secs } => {
                let mut response = (
                    StatusCode::TOO_MANY_REQUESTS,
                    format!("Too many comments. Try again in {retry_after_secs} seconds."),
                )
                    .into_response();
                if let Ok(value) = HeaderValue::from_str(&retry_after_secs.to_string()) {
                    response.headers_mut().insert(RETRY_AFTER, value);
                }
                response
            }
            _ => {
                let (status, body) = match self {
                    PostError::PostNotFound => {
                        (StatusCode::NOT_FOUND, "Post not found".to_string())
                    }
                    PostError::TagNotFound => (StatusCode::NOT_FOUND, "Tag not found".to_string()),
                    PostError::Forbidden => (
                        StatusCode::FORBIDDEN,
                        "You do not have permission to perform this action".to_string(),
                    ),
                    PostError::Conflict(current_updated_at) => {
                        (StatusCode::CONFLICT, current_updated_at)
                    }
                    PostError::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
                    // PostError::InvalidPostContent => {
                    //     (StatusCode::BAD_REQUEST, "Invalid post content.".to_string())
                    // }
                    PostError::UploadFailed(msg) => (StatusCode::BAD_REQUEST, msg),
                    PostError::InternalError(msg) => {
                        error!("Internal post error: {}", msg);
                        if cfg!(debug_assertions) {
                            (StatusCode::INTERNAL_SERVER_ERROR, msg)
                        } else {
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Internal Server Error".to_string(),
                            )
                        }
                    }
                    PostError::Media(_) => unreachable!(),
                    PostError::RateLimited { .. } => unreachable!(),
                };

                (status, body).into_response()
            }
        }
    }
}

impl From<sqlx::Error> for PostError {
    fn from(err: sqlx::Error) -> Self {
        PostError::InternalError(err.to_string())
    }
}

impl From<MediaError> for PostError {
    fn from(err: MediaError) -> Self {
        PostError::Media(err)
    }
}
