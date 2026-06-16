use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[allow(dead_code)]
pub enum BackupError {
    IoError(std::io::Error),
    ZipError(zip::result::ZipError),
    InternalError(String),
}

impl From<std::io::Error> for BackupError {
    fn from(e: std::io::Error) -> Self {
        BackupError::IoError(e)
    }
}

impl From<zip::result::ZipError> for BackupError {
    fn from(e: zip::result::ZipError) -> Self {
        BackupError::ZipError(e)
    }
}

impl IntoResponse for BackupError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            BackupError::IoError(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("IO error: {}", e))
            }
            BackupError::ZipError(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Zip error: {}", e))
            }
            BackupError::InternalError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            }
        };
        (status, body).into_response()
    }
}
