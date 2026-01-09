use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::error;

use crate::domain::errors::media::MediaError;

#[derive(Debug)]
pub enum SeriesError {
    Duplication,
    PermissionDenied,
    InternalError(String),
    ExposedInternalError(String),
    Media(MediaError),
}

impl From<String> for SeriesError {
    fn from(s: String) -> Self {
        SeriesError::InternalError(s)
    }
}

impl From<std::io::Error> for SeriesError {
    fn from(err: std::io::Error) -> Self {
        SeriesError::InternalError(err.to_string())
    }
}

impl IntoResponse for SeriesError {
    fn into_response(self) -> Response {
        match self {
            SeriesError::Media(inner) => inner.into_response(),
            _ => {
                let (status, body) = match self {
                    SeriesError::Duplication => {
                        (StatusCode::CONFLICT, "Duplication detected.".to_string())
                    }

                    SeriesError::PermissionDenied => (
                        StatusCode::FORBIDDEN,
                        "You do not have permission to perform this action".to_string(),
                    ),
                    SeriesError::InternalError(msg) => {
                        if cfg!(debug_assertions) {
                            (StatusCode::INTERNAL_SERVER_ERROR, msg)
                        } else {
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Internal Server Error".to_string(),
                            )
                        }
                    }
                    SeriesError::ExposedInternalError(msg) => {
                        error!("Internal media error: {}", msg);
                        (StatusCode::INTERNAL_SERVER_ERROR, msg.to_string())
                    }
                    SeriesError::Media(_) => unreachable!(),
                };

                (status, body).into_response()
            }
        }
    }
}

impl From<sqlx::Error> for SeriesError {
    fn from(err: sqlx::Error) -> Self {
        SeriesError::InternalError(err.to_string())
    }
}

impl From<MediaError> for SeriesError {
    fn from(err: MediaError) -> Self {
        SeriesError::Media(err)
    }
}
