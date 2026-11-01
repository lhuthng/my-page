use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub enum NotificationError {
    NotFound,
    InternalError(String),
}

impl From<sqlx::Error> for NotificationError {
    fn from(err: sqlx::Error) -> Self {
        NotificationError::InternalError(err.to_string())
    }
}

impl IntoResponse for NotificationError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            NotificationError::NotFound => {
                (StatusCode::NOT_FOUND, "Notification not found".to_string())
            }
            NotificationError::InternalError(msg) => {
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
