use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};

use crate::infrastructure::{persistence::redirects, web::server::AppState};

async fn resolve(
    State(state): State<Arc<AppState>>,
    Path((kind, slug)): Path<(String, String)>,
) -> Response {
    match redirects::resolve(&state.post_service.pool, &kind, &slug).await {
        Ok(Some(target)) => (StatusCode::OK, Json(target)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("redirect lookup failed for {kind}/{slug}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

// Public: it only answers where a published entity moved to, which is the
// same information the public page already exposes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/{kind}/{slug}", get(resolve))
}
