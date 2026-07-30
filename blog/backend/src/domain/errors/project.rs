use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::error;

use crate::domain::errors::{media::MediaError, post::PostError};

pub enum ProjectError {
    ProjectNotFound,
    Forbidden,
    UploadFailed(String),
    InvalidDemo(String),
    Conflict(String),
    InternalError(String),
    Media(MediaError),
    Post(PostError),
}

impl IntoResponse for ProjectError {
    fn into_response(self) -> Response {
        match self {
            ProjectError::Media(inner) => inner.into_response(),
            ProjectError::Post(inner) => inner.into_response(),
            ProjectError::ProjectNotFound => {
                (StatusCode::NOT_FOUND, "Project not found".to_string()).into_response()
            }
            ProjectError::Forbidden => (
                StatusCode::FORBIDDEN,
                "You do not have permission to perform this action".to_string(),
            )
                .into_response(),
            ProjectError::UploadFailed(msg) | ProjectError::InvalidDemo(msg) => {
                (StatusCode::BAD_REQUEST, msg).into_response()
            }
            ProjectError::Conflict(msg) => (StatusCode::CONFLICT, msg).into_response(),
            ProjectError::InternalError(msg) => {
                error!("Internal project error: {}", msg);
                if cfg!(debug_assertions) {
                    (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
                } else {
                    (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
                }
            }
        }
    }
}

impl From<sqlx::Error> for ProjectError {
    fn from(err: sqlx::Error) -> Self {
        ProjectError::InternalError(err.to_string())
    }
}

impl From<std::io::Error> for ProjectError {
    fn from(err: std::io::Error) -> Self {
        ProjectError::InternalError(err.to_string())
    }
}

impl From<MediaError> for ProjectError {
    fn from(err: MediaError) -> Self {
        ProjectError::Media(err)
    }
}

impl From<PostError> for ProjectError {
    fn from(err: PostError) -> Self {
        ProjectError::Post(err)
    }
}
