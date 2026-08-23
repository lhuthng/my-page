use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::HeaderMap,
    response::IntoResponse,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{
    application::{commands::dashboard::*, services::dashboard::DashboardService},
    domain::{entities::secret::Claims, errors::user::UserError},
    infrastructure::web::{
        api::middlewares::analytics::{normalize_country_code, path_group},
        server::AppState,
    },
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

#[derive(Deserialize)]
pub struct VisitorCountriesQuery {
    pub days: Option<i64>,
}

#[derive(Serialize)]
pub struct VisitorCountriesResponse {
    pub countries: Vec<crate::infrastructure::persistence::analytics::VisitorCountryStat>,
}

#[derive(Deserialize)]
pub struct TrackVisitBody {
    pub path: Option<String>,
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

pub async fn get_visitor_countries(
    State(state): State<Arc<AppState>>,
    Query(query): Query<VisitorCountriesQuery>,
) -> Result<impl IntoResponse, UserError> {
    let days = query.days.unwrap_or(30).clamp(1, 365);
    let countries = state.analytics_service.get_country_stats(days).await?;

    Ok(Json(VisitorCountriesResponse { countries }))
}

pub async fn track_visit(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<TrackVisitBody>,
) -> Result<impl IntoResponse, UserError> {
    let country_code = normalize_country_code(&headers);
    let path = body.path.unwrap_or_else(|| "/".to_string());
    let group = path_group(&path);
    let day = Utc::now().date_naive().to_string();

    state
        .analytics_service
        .record_country_visit(&day, &country_code, &group)
        .await?;

    Ok(())
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

#[derive(Serialize)]
pub struct TrashItem {
    pub post_id: i64,
    pub project_id: Option<i64>,
    pub game_id: Option<i64>,
    pub title: String,
    pub slug: String,
    pub content_kind: String,
    pub deleted_at: Option<String>,
    pub scheduled_purge_at: Option<String>,
    pub deletion_reason: Option<String>,
    pub status: String,
}

pub async fn get_trash(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, UserError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|e| UserError::InternalError(e.to_string()))?;
    let is_admin = claims.role == "admin";
    let rows = if is_admin {
        sqlx::query_as::<_, (i64, Option<i64>, Option<i64>, String, String, String, Option<String>, Option<String>, Option<String>, String)>(
            "SELECT posts.id, projects.id, games.id, posts.title, posts.slug, posts.content_kind, posts.deleted_at, posts.scheduled_purge_at, posts.deletion_reason, posts.status FROM posts LEFT JOIN projects ON projects.post_id = posts.id LEFT JOIN games ON games.post_id = posts.id WHERE posts.deleted_at IS NOT NULL ORDER BY posts.deleted_at DESC LIMIT 100",
        )
        .fetch_all(&state.post_service.pool)
        .await
        .map_err(|e| UserError::InternalError(e.to_string()))?
    } else {
        sqlx::query_as::<_, (i64, Option<i64>, Option<i64>, String, String, String, Option<String>, Option<String>, Option<String>, String)>(
            "SELECT posts.id, projects.id, games.id, posts.title, posts.slug, posts.content_kind, posts.deleted_at, posts.scheduled_purge_at, posts.deletion_reason, posts.status FROM posts LEFT JOIN projects ON projects.post_id = posts.id LEFT JOIN games ON games.post_id = posts.id WHERE posts.deleted_at IS NOT NULL AND posts.user_id = ? ORDER BY posts.deleted_at DESC LIMIT 100",
        )
        .bind(user_id)
        .fetch_all(&state.post_service.pool)
        .await
        .map_err(|e| UserError::InternalError(e.to_string()))?
    };
    let items: Vec<TrashItem> = rows
        .into_iter()
        .map(
            |(post_id, project_id, game_id, title, slug, content_kind, deleted_at, scheduled_purge_at, deletion_reason, status)| TrashItem {
                post_id,
                project_id,
                game_id,
                title,
                slug,
                content_kind,
                deleted_at,
                scheduled_purge_at,
                deletion_reason,
                status,
            },
        )
        .collect();
    Ok(Json(serde_json::json!({ "items": items })))
}
