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

    let user_routes = Router::new()
        .merge(
            Router::new()
                .route("/{username}", get(handlers::user::get_user))
                .route("/{username}/posts", get(handlers::user::get_posts)),
        )
        // protected route
        .merge(
            Router::new()
                .route("/me", get(handlers::user::me))
                .route("/me", patch(handlers::user::change_details))
                .route("/me/check-mod", get(handlers::user::check_mod))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                )),
        );

    let media_routes = Router::new()
        // protected route
        .merge(
            Router::new()
                .route("/upload", post(handlers::media::upload))
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
                .layer(middleware::from_fn(middlewares::auth::mod_check))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                )),
        )
        // public route
        .merge(
            Router::new()
                .route("/s/{short_name}", get(handlers::media::get_link))
                .route("/", get(handlers::media::search)),
        )
        .fallback_service(get_service(ServeDir::new(&state.media_config.dir)));

    let post_routes = Router::new()
        // optional
        .merge(
            Router::new()
                .route("/{post_id}/comments", post(handlers::post::new_comment))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::optional_user_guard,
                )),
        )
        // user protected
        .merge(
            Router::new()
                .route("/new", post(handlers::post::new_post))
                .route("/id/{post_id}", patch(handlers::post::publish))
                .route("/id/{post_id}", get(handlers::post::get_post_details))
                .layer(middleware::from_fn(middlewares::auth::mod_check))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                )),
        )
        // public
        .merge(
            Router::new()
                .route("/{post_slug}", get(handlers::post::get_post_by_slug))
                // .route("/{post_id}/")
                .route("/featured", get(handlers::post::get_featured_posts))
                .route("/latest", get(handlers::post::get_latest_posts))
                .route("/check", get(handlers::post::check_post))
                .route("/categories", get(handlers::post::get_categories)),
        );

    Router::new()
        .nest("/media", media_routes)
        .nest("/auth", auth_routes)
        .nest("/users", user_routes)
        .nest("/posts", post_routes)
        .with_state(state)
}
