// The project delete lifecycle: soft-delete to trash (with the legacy
// shared-post guard), restore, admin purge.
use std::sync::Arc;

use axum::{
    Extension,
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
};

use crate::{
    domain::{entities::secret::Claims, errors::project::ProjectError},
    infrastructure::web::{
        api::handlers::project::dto::DeleteProjectQuery,
        api::support::ownership::require_can_delete,
        server::AppState,
    },
};

pub async fn delete_project_draft(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(project_id): AxumPath<i64>,
    Query(query): Query<DeleteProjectQuery>,
) -> Result<StatusCode, ProjectError> {
    let post_id = require_can_delete(
        &state.project_service.pool,
        "projects",
        project_id,
        &claims,
        ProjectError::InternalError,
        ProjectError::ProjectNotFound,
        ProjectError::Forbidden,
    )
    .await?;
    // If this project's post is shared with its delegated game (legacy migration),
    // deleting the project must NOT soft-delete the shared post — that would also hide the game.
    // In that case just hard-delete the project row and keep the post/game.
    let shared_game: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM games WHERE post_id = ?",
    )
    .bind(post_id)
    .fetch_optional(&state.project_service.pool)
    .await?;
    if shared_game.is_some() {
        sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(project_id)
            .execute(&state.project_service.pool)
            .await?;
        return Ok(StatusCode::NO_CONTENT);
    }
    // kind guard: ensure this post is actually a project (content_kind check done via projects join, but also verify posts.content_kind)
    let row = sqlx::query_as::<_, (String, Option<String>, String)>(
        "SELECT content_kind, deleted_at, status FROM posts WHERE id = ?",
    )
    .bind(post_id)
    .fetch_optional(&state.project_service.pool)
    .await?
    .ok_or(ProjectError::ProjectNotFound)?;
    if row.0 != "project" {
        return Err(ProjectError::InvalidDemo(
            "Use the typed delete endpoint for this content kind.".to_string(),
        ));
    }
    if row.1.is_some() {
        return Err(ProjectError::Conflict(
            "Project already in trash.".to_string(),
        ));
    }
    let reason = query
        .reason
        .unwrap_or_else(|| "user_request".to_string());
    let allowed = ["user_request", "dmca", "moderation", "replaced", "other"];
    if !allowed.contains(&reason.as_str()) {
        return Err(ProjectError::InvalidDemo("Invalid deletion reason.".to_string()));
    }
    // soft-delete: flag, keep row for 7-day rollback
    sqlx::query(
        "UPDATE posts SET deleted_at = CURRENT_TIMESTAMP, deletion_reason = ?, deletion_detail = ?, deleted_by = ?, scheduled_purge_at = datetime('now','+7 days'), prev_status = status WHERE id = ?",
    )
    .bind(&reason)
    .bind(&query.detail)
    .bind(
        claims
            .user_id
            .parse::<i64>()
            .unwrap_or(0),
    )
    .bind(post_id)
    .execute(&state.project_service.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn restore_project(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(project_id): AxumPath<i64>,
) -> Result<StatusCode, ProjectError> {
    let post_id = require_can_delete(
        &state.project_service.pool,
        "projects",
        project_id,
        &claims,
        ProjectError::InternalError,
        ProjectError::ProjectNotFound,
        ProjectError::Forbidden,
    )
    .await?;
    let deleted_at: Option<String> = sqlx::query_scalar("SELECT deleted_at FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_optional(&state.project_service.pool)
        .await?
        .ok_or(ProjectError::ProjectNotFound)?;
    if deleted_at.is_none() {
        return Err(ProjectError::Conflict("Project is not in trash.".to_string()));
    }
    sqlx::query(
        "UPDATE posts SET deleted_at = NULL, deletion_reason = NULL, deletion_detail = NULL, deleted_by = NULL, scheduled_purge_at = NULL, prev_status = NULL WHERE id = ?",
    )
    .bind(post_id)
    .execute(&state.project_service.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn purge_project_now(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(project_id): AxumPath<i64>,
) -> Result<StatusCode, ProjectError> {
    if claims.role != "admin" {
        return Err(ProjectError::Forbidden);
    }
    let post_id: Option<i64> = sqlx::query_scalar("SELECT post_id FROM projects WHERE id=?")
        .bind(project_id)
        .fetch_optional(&state.project_service.pool)
        .await?
        .ok_or(ProjectError::ProjectNotFound)?;
    // hard purge: delete post (cascades)
    sqlx::query("DELETE FROM posts WHERE id = ?")
        .bind(post_id)
        .execute(&state.project_service.pool)
        .await?;
    // best-effort disk cleanup
    let dir = state.project_demo_config.dir.join(project_id.to_string());
    let _ = tokio::fs::remove_dir_all(&dir).await;
    Ok(StatusCode::NO_CONTENT)
}

