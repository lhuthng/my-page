use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use serde::Deserialize;

use crate::{
    application::{commands::dashboard::*, services::dashboard::DashboardService},
    domain::{entities::secret::Claims, errors::user::UserError},
    infrastructure::web::server::AppState,
};

#[derive(Deserialize)]
pub struct DashboardProjectsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub search: Option<String>,
}

#[axum::debug_handler]
pub async fn get_overview(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, UserError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|e| UserError::InternalError(e.to_string()))?;

    let result = state
        .dashboard_service
        .get_overview(GetOverviewCommand {
            user_id,
            role: claims.role,
        })
        .await?;

    Ok(Json(result))
}

#[derive(Deserialize)]
pub struct DashboardPostsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub search: Option<String>,
}

#[axum::debug_handler]
pub async fn get_posts(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<DashboardPostsQuery>,
) -> Result<impl IntoResponse, UserError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|e| UserError::InternalError(e.to_string()))?;

    let result = state
        .dashboard_service
        .get_posts(GetDashboardPostsCommand {
            user_id,
            role: claims.role,
            search: query.search,
            limit: query.limit.unwrap_or(20),
            offset: query.offset.unwrap_or(0),
        })
        .await?;

    Ok(Json(result))
}

#[derive(Deserialize)]
pub struct DashboardUsersQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub search: Option<String>,
    pub role: Option<String>,
}

#[axum::debug_handler]
pub async fn get_projects(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<DashboardProjectsQuery>,
) -> Result<impl IntoResponse, UserError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|e| UserError::InternalError(e.to_string()))?;

    let result = state
        .dashboard_service
        .get_projects(GetDashboardProjectsCommand {
            user_id,
            role: claims.role,
            search: query.search,
            limit: query.limit.unwrap_or(20),
            offset: query.offset.unwrap_or(0),
        })
        .await?;

    Ok(Json(result))
}

pub async fn get_users(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<DashboardUsersQuery>,
) -> Result<impl IntoResponse, UserError> {
    let result = state
        .dashboard_service
        .get_users(GetDashboardUsersCommand {
            role: claims.role,
            search: query.search,
            role_filter: query.role,
            limit: query.limit.unwrap_or(20),
            offset: query.offset.unwrap_or(0),
        })
        .await?;

    Ok(Json(result))
}

#[derive(Deserialize)]
pub struct UpdateTagBody {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
}

pub async fn update_tag(
    State(state): State<Arc<AppState>>,
    Path(tag_id): Path<i64>,
    Json(body): Json<UpdateTagBody>,
) -> Result<impl IntoResponse, UserError> {
    let result = state
        .dashboard_service
        .update_tag(UpdateDashboardTagCommand {
            id: tag_id,
            name: body.name,
            slug: body.slug,
            description: body.description,
        })
        .await?;

    Ok(Json(result))
}

pub async fn delete_tag(
    State(state): State<Arc<AppState>>,
    Path(tag_id): Path<i64>,
) -> Result<impl IntoResponse, UserError> {
    state
        .dashboard_service
        .delete_tag(DeleteDashboardTagCommand { id: tag_id })
        .await?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
