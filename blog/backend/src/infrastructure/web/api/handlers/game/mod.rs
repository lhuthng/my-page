// Game feature handlers, split by responsibility. This module is the public
// surface: route handlers for the router plus the wire types their
// signatures use. Internal helpers stay inside their sub-modules.
mod dto;
mod jsdos;
mod publish;
mod read;
mod response;
mod trash;
mod update;
mod write;

#[cfg(test)]
mod tests;

pub use dto::{
    CheckQuery, CheckResponse, CompleteJsDosUploadResponse, DeleteGameQuery,
    FeaturedGamesQuery, FeaturedGamesResponse, GameCard, GameStats, JsDosUploadResponse,
    LatestGamesQuery, LatestGamesResponse, SetGameFeaturedBody, StartJsDosUploadRequest,
    StartJsDosUploadResponse,
};
pub use jsdos::{
    abort_jsdos_upload, append_jsdos_chunk, complete_jsdos_upload, get_jsdos_bundle,
    start_jsdos_upload,
};
pub use publish::{publish_game, set_game_featured};
pub use read::{
    check_game, get_all_games, get_featured_games, get_game_by_slug, get_game_details,
    get_latest_games,
};
pub use response::{GameResponse, UpdateGameResponse};
pub use trash::{delete_game_draft, purge_game_now, restore_game};
pub use update::update_game;
pub use write::{change_cover, new_game};

// ---------------------------------------------------------------------------
// Route table
use std::sync::Arc;

use axum::{
    extract::DefaultBodyLimit,
    middleware,
    routing::{delete, get, patch, post, put},
    Router,
};

use crate::infrastructure::web::{api::handlers::v86, api::middlewares, server::AppState};

pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // optional-auth routes
    Router::new()
        .route("/s/{game_slug}", get(get_game_by_slug))
        .route(
            "/s/{game_slug}/v86/saves",
            get(v86::get_game_save)
                .put(v86::put_game_save)
                .delete(v86::delete_game_save),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middlewares::auth::optional_user_guard,
        ))
        .layer(DefaultBodyLimit::max(3 * 1024 * 1024))
        // mod-protected routes
        .merge(
            Router::new()
                .route("/new", post(new_game))
                .route("/id/{game_id}/jsdos/upload", post(start_jsdos_upload))
                .route(
                    "/id/{game_id}/jsdos/upload/{upload_id}/chunk/{chunk_index}",
                    put(append_jsdos_chunk),
                )
                .route(
                    "/id/{game_id}/jsdos/upload/{upload_id}/complete",
                    post(complete_jsdos_upload),
                )
                .route(
                    "/id/{game_id}/jsdos/upload/{upload_id}",
                    delete(abort_jsdos_upload),
                )
                .route("/all", get(get_all_games))
                .route("/id/{game_id}", post(publish_game))
                .route("/id/{game_id}", get(get_game_details))
                .route("/id/{game_id}", patch(update_game))
                .route("/id/{game_id}", delete(delete_game_draft))
                .route("/id/{game_id}/restore", post(restore_game))
                .route("/id/{game_id}/cover", patch(change_cover))
                .layer(middleware::from_fn(middlewares::auth::mod_check))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                ))
                .layer(DefaultBodyLimit::max(100 * 1024 * 1024)),
        )
        // admin-protected routes
        .merge(
            Router::new()
                .route("/id/{game_id}/featured", put(set_game_featured))
                .route("/id/{game_id}/purge", delete(purge_game_now))
                .layer(middleware::from_fn(middlewares::auth::admin_check))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                )),
        )
        // public routes
        .merge(
            Router::new()
                .route("/s/{game_slug}/jsdos", get(get_jsdos_bundle))
                .route(
                    "/s/{game_slug}/v86/{sha256}/full.iso",
                    get(v86::get_game_iso),
                )
                .route(
                    "/s/{game_slug}/v86/{sha256}/{part}",
                    get(v86::get_game_chunk),
                )
                .route(
                    "/s/{game_slug}/v86/disk/{sha256}/{part}",
                    get(v86::get_game_disk_chunk),
                )
                .route("/latest", get(get_latest_games))
                .route("/featured", get(get_featured_games))
                .route("/check", get(check_game)),
        )
}
