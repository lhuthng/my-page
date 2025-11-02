use std::sync::Arc;

use axum::{Router, routing::post};

use crate::infrastructure::web::{api::handlers::auth, server::AppState};

// This MUST retuns Router<()> instead of Router<AppState>
pub fn create(state: Arc<AppState>) -> Router<()> {
    let router = Router::new()
        .route("/auth/login", post(auth::login))
        .route("/auth/register", post(auth::register))
        .route("/auth/refresh-token", post(auth::refresh_token))
        .with_state(state);

    router
}
