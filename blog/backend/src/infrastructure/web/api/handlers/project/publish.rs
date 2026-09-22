// Lifecycle transitions: publish a draft and toggle the featured flag.
use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path as AxumPath, State},
    response::IntoResponse,
};

use crate::{
    application::{
        commands::{
            post::PublishCommand,
            project::{GetProjectPostIdCommand, SetFeaturedProjectCommand},
        },
        services::{post::PostService, project::ProjectService},
    },
    domain::{entities::secret::Claims, errors::project::ProjectError},
    infrastructure::web::{api::handlers::project::dto::SetProjectFeaturedBody, server::AppState},
};

#[axum::debug_handler]
pub async fn publish_project(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(project_id): AxumPath<i64>,
) -> Result<(), ProjectError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| ProjectError::InternalError("Cannot parse id".to_string()))?;
    let post_id = state
        .project_service
        .get_project_post_id(GetProjectPostIdCommand {
            project_id,
            required_author_id: Some(user_id),
        })
        .await?;
    state
        .post_service
        .publish(PublishCommand { user_id, post_id })
        .await?;
    Ok(())
}

pub async fn set_project_featured(
    State(state): State<Arc<AppState>>,
    AxumPath(project_id): AxumPath<i64>,
    Json(body): Json<SetProjectFeaturedBody>,
) -> Result<impl IntoResponse, ProjectError> {
    state
        .project_service
        .set_project_featured(SetFeaturedProjectCommand {
            project_id,
            is_featured: body.is_featured,
        })
        .await?;
    Ok(())
}
