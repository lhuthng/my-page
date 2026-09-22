// Project feature handlers, split by responsibility. This module is the
// public surface: route handlers for the router plus the wire types their
// signatures use. Internal helpers stay inside their sub-modules.
mod dto;
mod publish;
mod read;
mod response;
mod trash;
mod update;
mod write;

pub use dto::{
    CheckQuery, CheckResponse, CompleteJsDosUploadResponse, DeleteProjectQuery,
    FeaturedProjectsQuery, FeaturedProjectsResponse, JsDosUploadResponse, LatestProjectsQuery,
    LatestProjectsResponse, ProjectCard, ProjectStats, SetProjectFeaturedBody,
    StartJsDosUploadRequest, StartJsDosUploadResponse,
};
pub use publish::{publish_project, set_project_featured};
pub use read::{
    check_project, get_all_projects, get_featured_projects, get_latest_projects,
    get_project_by_slug, get_project_details,
};
pub use response::{DelegatedGameResponse, ProjectResponse, UpdateProjectResponse};
pub use trash::{delete_project_draft, purge_project_now, restore_project};
pub use update::update_project;
pub use write::{change_cover, new_project};

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
    // optional-auth routes
    Router::new()
        .route("/s/{project_slug}", get(get_project_by_slug))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middlewares::auth::optional_user_guard,
        ))
        .layer(DefaultBodyLimit::max(3 * 1024 * 1024))
        // mod-protected routes
        .merge(
            Router::new()
                .route("/new", post(new_project))
                .route("/all", get(get_all_projects))
                .route("/id/{project_id}", post(publish_project))
                .route("/id/{project_id}", get(get_project_details))
                .route("/id/{project_id}", patch(update_project))
                .route("/id/{project_id}", delete(delete_project_draft))
                .route("/id/{project_id}/restore", post(restore_project))
                .route("/id/{project_id}/cover", patch(change_cover))
                .layer(middleware::from_fn(middlewares::auth::mod_check))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                ))
                .layer(DefaultBodyLimit::max(100 * 1024 * 1024)),
        )
        .merge(
            Router::new()
                .route("/id/{project_id}/purge", delete(purge_project_now))
                .layer(middleware::from_fn(middlewares::auth::admin_check))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                )),
        )
        // admin-protected routes
        .merge(
            Router::new()
                .route("/id/{project_id}/featured", put(set_project_featured))
                .layer(middleware::from_fn(middlewares::auth::admin_check))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                )),
        )
        // public routes
        .merge(
            Router::new()
                .route("/latest", get(get_latest_projects))
                .route("/featured", get(get_featured_projects))
                .route("/check", get(check_project)),
        )
}
