// User feature handlers, split by responsibility. This module is the
// public surface: route handlers for the router plus the wire types their
// signatures use.
mod avatar;
mod dto;
mod profile;

pub use avatar::change_avatar;
pub use dto::{
    ChangeDetailsBody, CheckModResponse, GetLatestCommentsQuery, GetLatestCommentsResponse,
    GetPostsQuery, GetPostsResponse, GetUserResponse, MeResponse, SearchUserQuery,
    SearchUserResponse,
};
pub use profile::{change_details, check_mod, get_latest_comments, get_posts, get_user, me, search};

// ---------------------------------------------------------------------------
// Route table
use std::sync::Arc;

use axum::{
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, patch},
    Router,
};

use crate::infrastructure::web::{api::middlewares, server::AppState};

pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // public route
    Router::new()
        .route("/{username}", get(get_user))
        .route("/", get(search))
        // optional-public route
        .merge(
            Router::new()
                .route("/{username}/posts", get(get_posts))
                .route("/{username}/comments", get(get_latest_comments))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::optional_user_guard,
                )),
        )
        // protected route
        .merge(
            Router::new()
                .route("/me", get(me))
                .route("/me/details", patch(change_details))
                .route("/me/avatar", patch(change_avatar))
                .route("/me/check-mod", get(check_mod))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                ))
                .layer(DefaultBodyLimit::max(20 * 1024 * 1024)),
        )
}
