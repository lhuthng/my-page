// Audiobook feature handlers, split by responsibility. This module is the
// public surface: route handlers for the router plus the wire types their
// signatures use.
mod dto;
mod read;
mod response;
mod shared;
mod tracks;
mod write;

pub use dto::{
    ChangeStatusPayload, ListQuery, ReorderTracksPayload, SlugQuery, UpdateAudiobookPayload,
    UpdateTrackPayload,
};
pub use read::{
    check_slug, get_audiobook_details, get_audiobooks, get_public_audiobook, get_public_audiobooks,
    list_tags,
};
pub use response::{
    AudiobookCreatedResponse, AudiobookDetailsResponse, AudiobookListResponse,
    AudiobookSummaryResponse, AudiobookTagsResponse, SlugAvailabilityResponse,
    TrackCreatedResponse,
};
pub use tracks::{add_track, remove_track, reorder_tracks, replace_track_medium, update_track};
pub use write::{change_cover, change_status, delete_audiobook, new_audiobook, update_audiobook};

// ---------------------------------------------------------------------------
// Route table
use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware,
    routing::{delete, get, patch, post, put},
};

use crate::infrastructure::web::{api::middlewares, server::AppState};

pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // moderator-protected authoring
    Router::new()
        .route("/all", get(get_audiobooks))
        .route("/tags", get(list_tags))
        .route("/new", post(new_audiobook))
        .route("/id/{audiobook_id}", get(get_audiobook_details))
        .route("/id/{audiobook_id}", patch(update_audiobook))
        .route("/id/{audiobook_id}", delete(delete_audiobook))
        .route("/id/{audiobook_id}/cover", patch(change_cover))
        .route("/id/{audiobook_id}/status", post(change_status))
        .route("/id/{audiobook_id}/tracks", post(add_track))
        // Declared before the `{track_id}` routes so the static segment
        // is matched first for PUT.
        .route("/id/{audiobook_id}/tracks/order", put(reorder_tracks))
        .route("/id/{audiobook_id}/tracks/{track_id}", patch(update_track))
        .route("/id/{audiobook_id}/tracks/{track_id}", delete(remove_track))
        // Replacing the audio is a multipart PUT so the file and its metadata
        // travel together, unlike the JSON metadata PATCH above.
        .route(
            "/id/{audiobook_id}/tracks/{track_id}/audio",
            put(replace_track_medium),
        )
        .layer(middleware::from_fn(middlewares::auth::mod_check))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middlewares::auth::user_guard,
        ))
        // Audio tracks are large; match the general media ceiling.
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024))
        // public
        .merge(
            Router::new()
                .route("/check", get(check_slug))
                .route("/public/all", get(get_public_audiobooks))
                .route("/public/s/{slug}", get(get_public_audiobook)),
        )
}
