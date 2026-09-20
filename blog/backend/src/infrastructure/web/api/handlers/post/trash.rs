// The post delete lifecycle: soft-delete to trash, restore, admin purge.
use std::sync::Arc;

use axum::{
    Extension,
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
};

use crate::{
    domain::{entities::secret::Claims, errors::post::PostError},
    infrastructure::web::{
        api::handlers::post::dto::DeletePostQuery,
        api::support::ownership::is_admin_or_mod,
        server::AppState,
    },
};

pub async fn delete_post(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(post_id): Path<i64>,
    Query(query): Query<DeletePostQuery>,
) -> Result<StatusCode, PostError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| PostError::InternalError("Cannot parse id".to_string()))?;
    let row = sqlx::query_as::<_, (i64, String, Option<String>)>(
        "SELECT user_id, content_kind, deleted_at FROM posts WHERE id = ?",
    )
    .bind(post_id)
    .fetch_optional(&state.post_service.pool)
    .await?
    .ok_or(PostError::PostNotFound)?;
    if row.1 != "post" {
        return Err(PostError::Validation(
            "Use the typed delete endpoint for projects/games.".to_string(),
        ));
    }
    if row.2.is_some() {
        return Err(PostError::Conflict("Post already in trash.".to_string()));
    }
    if row.0 != user_id && !is_admin_or_mod(&claims.role) {
        return Err(PostError::Forbidden);
    }
    let reason = query.reason.unwrap_or_else(|| "user_request".to_string());
    let allowed = ["user_request", "dmca", "moderation", "replaced", "other"];
    if !allowed.contains(&reason.as_str()) {
        return Err(PostError::Validation("Invalid deletion reason.".to_string()));
    }
    sqlx::query(
        "UPDATE posts SET deleted_at = CURRENT_TIMESTAMP, deletion_reason = ?, deletion_detail = ?, deleted_by = ?, scheduled_purge_at = datetime('now','+7 days'), prev_status = status WHERE id = ?",
    )
    .bind(&reason)
    .bind(&query.detail)
    .bind(user_id)
    .bind(post_id)
    .execute(&state.post_service.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn restore_post(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(post_id): Path<i64>,
) -> Result<StatusCode, PostError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| PostError::InternalError("Cannot parse id".to_string()))?;
    let row = sqlx::query_as::<_, (i64, Option<String>)>(
        "SELECT user_id, deleted_at FROM posts WHERE id = ?",
    )
    .bind(post_id)
    .fetch_optional(&state.post_service.pool)
    .await?
    .ok_or(PostError::PostNotFound)?;
    if row.1.is_none() {
        return Err(PostError::Conflict("Post is not in trash.".to_string()));
    }
    if row.0 != user_id && !is_admin_or_mod(&claims.role) {
        return Err(PostError::Forbidden);
    }
    sqlx::query(
        "UPDATE posts SET deleted_at = NULL, deletion_reason = NULL, deletion_detail = NULL, deleted_by = NULL, scheduled_purge_at = NULL, prev_status = NULL WHERE id = ?",
    )
    .bind(post_id)
    .execute(&state.post_service.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn purge_post_now(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(post_id): Path<i64>,
) -> Result<StatusCode, PostError> {
    if claims.role != "admin" {
        return Err(PostError::Forbidden);
    }
    sqlx::query("DELETE FROM posts WHERE id = ?")
        .bind(post_id)
        .execute(&state.post_service.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
