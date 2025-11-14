use std::{path::PathBuf, sync::Arc};

use axum::{
    Router, middleware,
    routing::{get, get_service, post},
};
use tower_http::services::ServeDir;

use crate::infrastructure::web::{
    api::{handlers, middlewares, services},
    server::AppState,
};

// This MUST retuns Router<()> instead of Router<AppState>
pub fn create(state: Arc<AppState>) -> Router<()> {
    let static_media_service = get_service(ServeDir::new(&state.media_config.dir));

    let auth_routes = Router::new()
        .route("/login", post(handlers::auth::login))
        .route("/register", post(handlers::auth::register))
        .route("/refresh", post(handlers::auth::refresh_token));

    let user_routes =
        Router::new()
            .route("/me", get(handlers::user::me))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                middlewares::auth::user_guard,
            ));

    let media_routes = Router::new()
        .route("/", post(handlers::media::upload))
        .layer(middleware::from_fn(middlewares::auth::mod_check))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middlewares::auth::user_guard,
        ));

    Router::new()
        .nest_service("/media/static", static_media_service)
        .nest("/media", media_routes)
        .nest("/auth", auth_routes)
        .nest("/user", user_routes)
        .with_state(state)
}
