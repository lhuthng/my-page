use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::error;

use crate::domain::errors::media::MediaError;

#[derive(Debug)]
#[allow(dead_code)]
pub enum AudiobookError {
    NotFound,
    Duplication,
    PermissionDenied,
    Validation(String),
    InternalError(String),
    ExposedInternalError(String),
    Media(MediaError),
}

impl From<String> for AudiobookError {
    fn from(s: String) -> Self {
        AudiobookError::InternalError(s)
    }
}

impl From<std::io::Error> for AudiobookError {
    fn from(err: std::io::Error) -> Self {
        AudiobookError::InternalError(err.to_string())
    }
}

impl From<sqlx::Error> for AudiobookError {
    fn from(err: sqlx::Error) -> Self {
        AudiobookError::InternalError(err.to_string())
    }
}

impl From<MediaError> for AudiobookError {
    fn from(err: MediaError) -> Self {
        AudiobookError::Media(err)
    }
}

impl IntoResponse for AudiobookError {
    fn into_response(self) -> Response {
        match self {
            AudiobookError::Media(inner) => inner.into_response(),
            _ => {
                let (status, body) = match self {
                    AudiobookError::NotFound => (
                        StatusCode::NOT_FOUND,
                        "Audiobook not found.".to_string(),
                    ),
                    AudiobookError::Duplication => {
                        (StatusCode::CONFLICT, "Duplication detected.".to_string())
                    }
                    AudiobookError::PermissionDenied => (
                        StatusCode::FORBIDDEN,
                        "You do not have permission to perform this action".to_string(),
                    ),
                    AudiobookError::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
                    AudiobookError::InternalError(msg) => {
                        if cfg!(debug_assertions) {
                            (StatusCode::INTERNAL_SERVER_ERROR, msg)
                        } else {
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Internal Server Error".to_string(),
                            )
                        }
                    }
                    AudiobookError::ExposedInternalError(msg) => {
                        error!("Internal audiobook error: {}", msg);
                        (StatusCode::INTERNAL_SERVER_ERROR, msg.to_string())
                    }
                    AudiobookError::Media(_) => unreachable!(),
                };

                (status, body).into_response()
            }
        }
    }
}
