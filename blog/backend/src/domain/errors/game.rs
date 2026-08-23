use std::fmt::{self, Display};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::error;

use crate::domain::errors::{media::MediaError, post::PostError, project::ProjectError};

#[derive(Debug)]
pub enum GameError {
    GameNotFound,
    SaveNotFound,
    Forbidden,
    UploadFailed(String),
    InvalidDemo(String),
    Conflict(String),
    InternalError(String),
    Media(MediaError),
    Post(PostError),
    Project(ProjectError),
}

impl Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameError::GameNotFound => write!(f, "Game not found"),
            GameError::SaveNotFound => write!(f, "No save exists for this game yet"),
            GameError::Forbidden => write!(f, "Forbidden"),
            GameError::UploadFailed(msg) => write!(f, "Upload failed: {msg}"),
            GameError::InvalidDemo(msg) => write!(f, "Invalid demo: {msg}"),
            GameError::Conflict(msg) => write!(f, "Conflict: {msg}"),
            GameError::InternalError(msg) => write!(f, "Internal error: {msg}"),
            GameError::Media(_) => write!(f, "Media error"),
            GameError::Post(_) => write!(f, "Post error"),
            GameError::Project(inner) => write!(f, "{inner}"),
        }
    }
}

impl IntoResponse for GameError {
    fn into_response(self) -> Response {
        match self {
            GameError::Media(inner) => inner.into_response(),
            GameError::Post(inner) => inner.into_response(),
            GameError::Project(inner) => inner.into_response(),
            GameError::GameNotFound => {
                (StatusCode::NOT_FOUND, "Game not found".to_string()).into_response()
            }
            GameError::SaveNotFound => {
                (StatusCode::NOT_FOUND, "No save exists for this game yet".to_string())
                    .into_response()
            }
            GameError::Forbidden => (
                StatusCode::FORBIDDEN,
                "You do not have permission to perform this action".to_string(),
            )
                .into_response(),
            GameError::UploadFailed(msg) | GameError::InvalidDemo(msg) => {
                (StatusCode::BAD_REQUEST, msg).into_response()
            }
            GameError::Conflict(msg) => (StatusCode::CONFLICT, msg).into_response(),
            GameError::InternalError(msg) => {
                error!("Internal game error: {}", msg);
                if cfg!(debug_assertions) {
                    (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
                } else {
                    (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
                }
            }
        }
    }
}

impl From<sqlx::Error> for GameError {
    fn from(err: sqlx::Error) -> Self {
        GameError::InternalError(err.to_string())
    }
}

impl From<std::io::Error> for GameError {
    fn from(err: std::io::Error) -> Self {
        GameError::InternalError(err.to_string())
    }
}

impl From<MediaError> for GameError {
    fn from(err: MediaError) -> Self {
        GameError::Media(err)
    }
}

impl From<PostError> for GameError {
    fn from(err: PostError) -> Self {
        GameError::Post(err)
    }
}

impl From<ProjectError> for GameError {
    fn from(err: ProjectError) -> Self {
        GameError::Project(err)
    }
}