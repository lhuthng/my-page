use std::sync::Arc;

use axum::{
    Router, middleware,
    routing::{delete, get, get_service, patch, post},
};
use tower_http::services::ServeDir;

use crate::infrastructure::web::{
    api::{handlers, middlewares},
    server::AppState,
};

// This MUST retuns Router<()> instead of Router<AppState>
pub fn build_router(state: Arc<AppState>) -> Router<()> {
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
        // protected route
        .merge(
            Router::new()
                .route("/", post(handlers::media::upload))
                .layer(middleware::from_fn(middlewares::auth::mod_check))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                )),
        )
        // public route
        .merge(
            Router::new()
                .route("/d/{short_name}", get(handlers::media::get_details))
                .route("/d/{short_name}", patch(handlers::media::change_details))
                .route("/d/{short_name}/aliases", get(handlers::media::get_aliases))
                .route("/d/{short_name}/aliases", post(handlers::media::add_alias))
                .route(
                    "/d/{short_name}/aliases/{alias}",
                    patch(handlers::media::change_alias),
                )
                .route(
                    "/d/{short_name}/aliases/{alias}",
                    delete(handlers::media::delete_alias),
                )
                .route("/s/{short_name}", get(handlers::media::get_link))
                .route("/", get(handlers::media::search)),
        )
        .fallback_service(get_service(ServeDir::new(&state.media_config.dir)));

    Router::new()
        .nest("/media", media_routes)
        .nest("/auth", auth_routes)
        .nest("/user", user_routes)
        .with_state(state)
}
