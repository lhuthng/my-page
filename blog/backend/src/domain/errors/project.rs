use std::fmt::{self, Display};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::error;

use crate::domain::errors::{media::MediaError, post::PostError};

#[derive(Debug)]
pub enum ProjectError {
    ProjectNotFound,
    SaveNotFound,
    Forbidden,
    UploadFailed(String),
    InvalidDemo(String),
    Conflict(String),
    InternalError(String),
    Media(MediaError),
    Post(PostError),
}

impl Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProjectError::ProjectNotFound => write!(f, "Project not found"),
            ProjectError::SaveNotFound => write!(f, "No save exists for this game yet"),
            ProjectError::Forbidden => write!(f, "Forbidden"),
            ProjectError::UploadFailed(msg) => write!(f, "Upload failed: {msg}"),
            ProjectError::InvalidDemo(msg) => write!(f, "Invalid demo: {msg}"),
            ProjectError::Conflict(msg) => write!(f, "Conflict: {msg}"),
            ProjectError::InternalError(msg) => write!(f, "Internal error: {msg}"),
            ProjectError::Media(_) => write!(f, "Media error"),
            ProjectError::Post(_) => write!(f, "Post error"),
        }
    }
}

impl IntoResponse for ProjectError {
    fn into_response(self) -> Response {
        match self {
            ProjectError::Media(inner) => inner.into_response(),
            ProjectError::Post(inner) => inner.into_response(),
            ProjectError::ProjectNotFound => {
                (StatusCode::NOT_FOUND, "Project not found".to_string()).into_response()
            }
            ProjectError::SaveNotFound => {
                (StatusCode::NOT_FOUND, "No save exists for this game yet".to_string())
                    .into_response()
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
