use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use tracing::error;

/// All newsletter endpoints return `{ "message": "..." }` on both success and
/// error, so the frontend can always parse the body as JSON.
#[derive(Serialize)]
pub struct NewsletterMessageResponse {
    pub message: String,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum NewsletterError {
    AlreadySubscribed,
    AlreadyConfirmed,
    AlreadyUnsubscribed,
    InvalidToken,
    ExpiredToken,
    NotConfigured(String),
    PermissionDenied,
    InternalError(String),
}

impl From<String> for NewsletterError {
    fn from(s: String) -> Self {
        NewsletterError::InternalError(s)
    }
}

impl IntoResponse for NewsletterError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            NewsletterError::AlreadySubscribed => (
                StatusCode::CONFLICT,
                "This email is already subscribed.".to_string(),
            ),
            NewsletterError::AlreadyConfirmed => (
                StatusCode::OK,
                "You're already subscribed.".to_string(),
            ),
            NewsletterError::AlreadyUnsubscribed => (
                StatusCode::OK,
                "This subscription was already removed.".to_string(),
            ),
            NewsletterError::InvalidToken => {
                (StatusCode::UNAUTHORIZED, "Invalid token".to_string())
            }
            NewsletterError::ExpiredToken => {
                (StatusCode::UNAUTHORIZED, "Expired token".to_string())
            }
            NewsletterError::NotConfigured(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg),
            NewsletterError::PermissionDenied => (
                StatusCode::FORBIDDEN,
                "You do not have permission to perform this action".to_string(),
            ),
            NewsletterError::InternalError(msg) => {
                error!("Internal newsletter error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                )
            }
        };

        (status, Json(NewsletterMessageResponse { message: body })).into_response()
    }
}

impl From<sqlx::Error> for NewsletterError {
    fn from(err: sqlx::Error) -> Self {
        NewsletterError::InternalError(err.to_string())
    }
}
