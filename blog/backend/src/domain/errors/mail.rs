use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::error;

#[derive(Debug)]
pub enum MailError {
    InvalidFileType,
    FileTooLarge,
    Duplication,
    UploadFailed(String),
    FileNotFound,
    PermissionDenied,
    InternalError(String),
    ExposedInternalError(String),
}

impl From<String> for MailError {
    fn from(s: String) -> Self {
        MailError::InternalError(s)
    }
}

impl From<std::io::Error> for MailError {
    fn from(err: std::io::Error) -> Self {
        MailError::InternalError(err.to_string())
    }
}

impl IntoResponse for MailError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            MailError::InvalidFileType => (
                StatusCode::BAD_REQUEST,
                "Unsupported or invalid file type".to_string(),
            ),
            MailError::FileTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "Uploaded file exceeds size limit".to_string(),
            ),
            MailError::Duplication => (StatusCode::CONFLICT, "Duplication detected.".to_string()),
            MailError::UploadFailed(msg) => (StatusCode::BAD_REQUEST, msg),
            MailError::FileNotFound => (
                StatusCode::BAD_REQUEST,
                "Requested file not found".to_string(),
            ),
            MailError::PermissionDenied => (
                StatusCode::FORBIDDEN,
                "You do not have permission to perform this action".to_string(),
            ),
            MailError::InternalError(msg) => {
                if cfg!(debug_assertions) {
                    (StatusCode::INTERNAL_SERVER_ERROR, msg)
                } else {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Internal Server Error".to_string(),
                    )
                }
            }
            MailError::ExposedInternalError(msg) => {
                error!("Internal media error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, msg.to_string())
            }
        };

        (status, body).into_response()
    }
}

impl From<sqlx::Error> for MailError {
    fn from(err: sqlx::Error) -> Self {
        MailError::InternalError(err.to_string())
    }
}
