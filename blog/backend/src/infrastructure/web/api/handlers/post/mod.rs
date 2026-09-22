// Post feature handlers, split by responsibility. This module is the public
// surface: route handlers for the router plus the wire types their
// signatures use. Internal helpers stay inside their sub-modules.
mod comments;
mod dto;
mod media;
mod publish;
mod read;
mod response;
mod trash;
mod update;
mod write;

pub use comments::{get_comments, new_comment};
pub use dto::{
    CheckQuery, CheckResponse, CommentsQuery, DeletePostQuery, GetFeaturedPostsBody,
    GetFeaturedPostsQuery, GetTagPostsQuery, NewCommentBody, PostData, PostPatchData,
    SearchPostQuery, SearchTagsQuery, SetPostFeaturedBody, SetRelatedPostsBody,
};
pub use media::{push_like, push_view};
pub use publish::{publish, set_post_featured};
pub use read::{
    check_post, get_categories, get_featured_posts, get_latest_posts, get_post_by_slug,
    get_post_details, get_posts_by_tag, get_related_posts, search, search_tags,
};
pub use response::{
    CategoryResponse, Comment, CommentsResponse, GetCategoriesResponse, GetFeaturedPostsResponse,
    GetPostDetailsResponse, GetRelatedPostsResponse, NewCommentResponse, Post, PostResponse,
    PostSeriesResponse, SearchPostResponse, SearchPostResult, SearchTagResult, SearchTagsResponse,
    TagPostsResponse, UpdatePostResponse,
};
pub use trash::{delete_post, purge_post_now, restore_post};
pub use update::{change_cover, set_related_posts, update_post};
pub use write::new_post;

// ---------------------------------------------------------------------------
// Route tables
use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware,
    routing::{delete, get, patch, post, put},
};

use crate::infrastructure::web::{api::middlewares, server::AppState};

pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // optional-auth routes (view post, new comment)
    Router::new()
        .route("/id/{post_id}/comments/new", put(new_comment))
        .route("/s/{post_slug}", get(get_post_by_slug))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middlewares::auth::optional_user_guard,
        ))
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        // mod-protected routes
        .merge(
            Router::new()
                .route("/new", post(new_post))
                .route("/id/{post_id}", post(publish))
                .route("/id/{post_id}", get(get_post_details))
                .route("/id/{post_id}", patch(update_post))
                .route("/id/{post_id}", delete(delete_post))
                .route("/id/{post_id}/restore", post(restore_post))
                .route("/id/{post_id}/cover", patch(change_cover))
                .route("/id/{post_id}/related", patch(set_related_posts))
                .layer(middleware::from_fn(middlewares::auth::mod_check))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                ))
                .layer(DefaultBodyLimit::max(100 * 1024 * 1024)),
        )
        // admin purge
        .merge(
            Router::new()
                .route("/id/{post_id}/purge", delete(purge_post_now))
                .layer(middleware::from_fn(middlewares::auth::admin_check))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                )),
        )
        // admin-protected routes
        .merge(
            Router::new()
                .route("/id/{post_id}/featured", put(set_post_featured))
                .layer(middleware::from_fn(middlewares::auth::admin_check))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                )),
        )
        // public routes
        .merge(
            Router::new()
                .route("/id/{post_id}/comments", get(get_comments))
                .route("/id/{post_id}/view", post(push_view))
                .route("/id/{post_id}/like", post(push_like))
                .route("/id/{post_id}/related", get(get_related_posts))
                .route("/featured", get(get_featured_posts))
                .route("/latest", get(get_latest_posts))
                .route("/check", get(check_post))
                .route("/categories", get(get_categories))
                .route("/", get(search)),
        )
}

/// Tag browsing lives under its own top-level `/tags` nest but is owned by
/// the post feature.
pub fn tags_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let _ = &state;
    Router::new()
        .route("/", get(search_tags))
        .route("/{tag_slug}", get(get_posts_by_tag))
}
