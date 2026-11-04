use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Serialize;

use crate::infrastructure::{persistence::content_links, web::server::AppState};

#[derive(Serialize)]
struct LinksResponse {
    links: Vec<content_links::LinkState>,
}

async fn health(State(state): State<Arc<AppState>>, Path(slug): Path<String>) -> Response {
    match content_links::health_for(&state.post_service.pool, &slug).await {
        Ok(Some(links)) => (StatusCode::OK, Json(LinksResponse { links })).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("link health lookup failed for {slug}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

// Public: it reports which published targets a published body points at.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/s/{slug}", get(health))
}
