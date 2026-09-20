// Media feature handlers, split by responsibility. This module is the
// public surface: route handlers for the router plus the wire types their
// signatures use.
mod aliases;
mod dto;
mod manage;
mod serving;
mod upload;

pub use aliases::{add_alias, change_alias, delete_alias, get_aliases};
pub use dto::{
    AddAliasPayload, ChangeAliasPayload, ChangeDetailsPayload, GetAliasesResponse,
    GetLinkResponse, GetMediaDetailsResponse, MediaQuery, SearchResponse,
};
pub use manage::{change_details, get_details};
pub use serving::{get_link, get_media, search};
pub use upload::upload;

// ---------------------------------------------------------------------------
// Route table
use std::sync::Arc;

use axum::{
    extract::DefaultBodyLimit,
    middleware,
    routing::{delete, get, get_service, patch, post},
    Router,
};
use tower_http::services::ServeDir;

use crate::infrastructure::web::{api::middlewares, server::AppState};

pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // protected route
    Router::new()
        .route("/upload", post(upload))
        .route("/d/{short_name}", get(get_details))
        .route("/d/{short_name}", patch(change_details))
        .route("/d/{short_name}/aliases", get(get_aliases))
        .route("/d/{short_name}/aliases", post(add_alias))
        .route("/d/{short_name}/aliases/{alias}", patch(change_alias))
        .route("/d/{short_name}/aliases/{alias}", delete(delete_alias))
        .layer(middleware::from_fn(middlewares::auth::mod_check))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middlewares::auth::user_guard,
        ))
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024))
        // public route
        .merge(
            Router::new()
                .route("/s/{short_name}", get(get_link))
                .route("/i/{short_name}", get(get_media))
                .route("/all", get(search)),
        )
        .fallback_service(get_service(ServeDir::new(&state.media_config.dir)))
}
