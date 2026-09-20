// Project read endpoints: public slugs and details (including the delegated
// game hand-off), admin listings.
use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path as AxumPath, Query, State},
    response::IntoResponse,
};
use sqlx::Row;

use crate::{
    application::{
        commands::{
            post::CheckSlugCommand,
            project::{
                GetFeaturedProjectsCommand, GetLatestProjectsCommand, GetProjectBySlugCommand,
                GetProjectDetailsCommand,
            },
        },
        services::{post::PostService, project::ProjectService},
    },
    domain::{entities::secret::Claims, errors::project::ProjectError},
    infrastructure::web::{
        api::handlers::project::dto::{
            CheckQuery, CheckResponse, FeaturedProjectsQuery, FeaturedProjectsResponse,
            LatestProjectsQuery, LatestProjectsResponse,
        },
        api::handlers::project::response::project_response,
        api::handlers::v86::runtime_descriptor,
        server::AppState,
    },
};

pub async fn check_project(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CheckQuery>,
) -> Result<impl IntoResponse, ProjectError> {
    if let Some(post_slug) = query.slug {
        let exists = state
            .post_service
            .check_slug(CheckSlugCommand { post_slug })
            .await?;
        Ok(Json(CheckResponse { exists }))
    } else {
        Ok(Json(CheckResponse { exists: true }))
    }
}

pub async fn get_project_by_slug(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    AxumPath(slug): AxumPath<String>,
) -> Result<impl IntoResponse, ProjectError> {
    // The viewer is identified so the response can report `is_owner`. This used
    // to sit behind a `with_draft` flag that also returned the unpublished
    // body; with a single body there is nothing left to gate, so the viewer is
    // simply whoever the token says.
    let as_id = match opt_claims {
        Some(claims) => Some(
            claims
                .user_id
                .parse::<i64>()
                .map_err(|_| ProjectError::InternalError("Cannot parse id".to_string()))?,
        ),
        None => None,
    };

    let project = state
        .project_service
        .get_project_by_slug(GetProjectBySlugCommand { slug, as_id })
        .await?;
    let mut response = project_response(project);
    if let Some(game) = response.delegated_game.as_mut()
        && game.launcher_type == "v86"
    {
        game.v86_runtime = runtime_descriptor(
            &state.project_service.pool,
            &game.slug,
            state.artifact_base_url(),
        )
        .await?;
    }
    Ok(Json(response))
}

pub async fn get_project_details(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(project_id): AxumPath<i64>,
) -> Result<impl IntoResponse, ProjectError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| ProjectError::InternalError("Cannot parse id".to_string()))?;
    let is_admin = claims.role == "admin";
    let project = state
        .project_service
        .get_project_details(GetProjectDetailsCommand {
            project_id,
            viewing_user_id: user_id,
            required_author_id: if is_admin { None } else { Some(user_id) },
        })
        .await?;
    let mut response = project_response(project);
    if let Some(game) = response.delegated_game.as_mut()
        && game.launcher_type == "v86"
    {
        game.v86_runtime = runtime_descriptor(
            &state.project_service.pool,
            &game.slug,
            state.artifact_base_url(),
        )
        .await?;
    }
    Ok(Json(response))
}

pub async fn get_latest_projects(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LatestProjectsQuery>,
) -> Result<impl IntoResponse, ProjectError> {
    let projects = state
        .project_service
        .get_latest_project_snapshots(GetLatestProjectsCommand {
            limit: query.limit.unwrap_or(24),
            offset: query.offset.unwrap_or(0),
            public_only: true,
            required_author_id: None,
        })
        .await?;
    Ok(Json(LatestProjectsResponse {
        projects: projects.projects.into_iter().map(Into::into).collect(),
        has_more: projects.has_more,
    }))
}

pub async fn get_featured_projects(
    State(state): State<Arc<AppState>>,
    Query(query): Query<FeaturedProjectsQuery>,
) -> Result<impl IntoResponse, ProjectError> {
    let projects = state
        .project_service
        .get_featured_project_snapshots(GetFeaturedProjectsCommand {
            limit: query.limit.unwrap_or(5),
        })
        .await?;

    Ok(Json(FeaturedProjectsResponse {
        featured_projects: projects.into_iter().map(Into::into).collect(),
        has_more: false,
    }))
}

pub async fn get_all_projects(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<LatestProjectsQuery>,
) -> Result<impl IntoResponse, ProjectError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| ProjectError::InternalError("Cannot parse id".to_string()))?;
    let projects = state
        .project_service
        .get_latest_project_snapshots(GetLatestProjectsCommand {
            limit: query.limit.unwrap_or(100),
            offset: query.offset.unwrap_or(0),
            public_only: false,
            required_author_id: (claims.role != "admin").then_some(user_id),
        })
        .await?;
    Ok(Json(LatestProjectsResponse {
        projects: projects.projects.into_iter().map(Into::into).collect(),
        has_more: projects.has_more,
    }))
}

