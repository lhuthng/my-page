//! Book-scoped author API.
//!
//! Exposure is **per audiobook**, not per deployment. A book has an
//! `author_api_enabled` flag (default 0) that its creator turns on from the
//! dashboard; until then every `/author` route answers 404 for that book, and
//! switching it off again makes any issued keys inert.
//!
//! Two auth tiers live here:
//!   * `/audiobooks/id/{id}/settings` and `/keys` — session JWT, book creator.
//!   * everything else — `abk_…` key, bound to that one book.
mod download;
mod dto;
mod keys;
mod read;
mod response;
mod settings;
mod shared;
mod write;

use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::StatusCode,
    middleware,
    routing::{delete, get, post, put},
};

use crate::infrastructure::web::{api::middlewares, server::AppState};

pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let key_tier = Router::new()
        .route(
            "/audiobooks/id/{id}",
            get(read::get_manifest).patch(write::patch_book),
        )
        .route("/audiobooks/id/{id}/status", post(write::change_status))
        .route(
            "/audiobooks/id/{id}/chapters",
            put(write::put_chapters_batch),
        )
        .route(
            "/audiobooks/id/{id}/chapters/{number}",
            put(write::put_chapter)
                .patch(write::patch_chapter)
                .delete(write::delete_chapter),
        )
        .route(
            "/audiobooks/id/{id}/chapters/{number}/audio",
            get(download::get_chapter_audio),
        )
        .route("/audiobooks/id/{id}/archive", get(download::get_archive))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middlewares::auth::author_key_guard,
        ))
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024));

    let jwt_tier = Router::new()
        .route(
            "/audiobooks/id/{id}/settings",
            get(settings::get_settings).put(settings::put_settings),
        )
        .route(
            "/audiobooks/id/{id}/keys",
            get(keys::list_keys).post(keys::issue_key),
        )
        .route(
            "/audiobooks/id/{id}/keys/{key_id}",
            delete(keys::revoke_key),
        )
        .layer(middleware::from_fn(middlewares::auth::mod_check))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middlewares::auth::user_guard,
        ));

    // A layer wraps the fallback it finds at `.layer()` time, so an unmatched
    // path would otherwise answer `401 Invalid or revoked author key` from
    // `author_key_guard` instead of a plain 404. Setting the fallback *after*
    // the layers keeps it unwrapped — the same trap `router.rs` documents.
    key_tier
        .merge(jwt_tier)
        .fallback(|| async { StatusCode::NOT_FOUND })
}

#[cfg(test)]
mod tests;
