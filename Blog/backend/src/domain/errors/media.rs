use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::error;

#[derive(Debug)]
pub enum MediaError {
    InvalidFileType,
    FileTooLarge,
    Duplication,
    UploadFailed(String),
    FileNotFound,
    PermissionDenied,
    InternalError(String),
    ExposedInternalError(String),
}

impl From<String> for MediaError {
    fn from(s: String) -> Self {
        MediaError::InternalError(s)
    }
}

impl From<std::io::Error> for MediaError {
    fn from(err: std::io::Error) -> Self {
        MediaError::InternalError(err.to_string())
    }
}

impl IntoResponse for MediaError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            MediaError::InvalidFileType => (
                StatusCode::BAD_REQUEST,
                "Unsupported or invalid file type".to_string(),
            ),
            MediaError::FileTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "Uploaded file exceeds size limit".to_string(),
            ),
            MediaError::Duplication => (StatusCode::CONFLICT, "Duplication detected.".to_string()),
            MediaError::UploadFailed(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            MediaError::FileNotFound => (
                StatusCode::NOT_FOUND,
                "Requested file not found".to_string(),
            ),
            MediaError::PermissionDenied => (
                StatusCode::FORBIDDEN,
                "You do not have permission to perform this action".to_string(),
            ),
            MediaError::InternalError(msg) => {
                if cfg!(debug_assertions) {
                    (StatusCode::INTERNAL_SERVER_ERROR, msg)
                } else {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Internal Server Error".to_string(),
                    )
                }
            }
            MediaError::ExposedInternalError(msg) => {
                error!("Internal media error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, msg.to_string())
            }
        };

        (status, body).into_response()
    }
}

impl From<sqlx::Error> for MediaError {
    fn from(err: sqlx::Error) -> Self {
        MediaError::InternalError(err.to_string())
    }
}
