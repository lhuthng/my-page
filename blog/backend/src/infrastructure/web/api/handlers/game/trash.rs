// The game delete lifecycle: soft-delete to trash, restore, admin purge.
use std::sync::Arc;

use axum::{
    Extension,
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
};

use crate::{
    domain::{entities::secret::Claims, errors::game::GameError},
    infrastructure::web::{
        api::handlers::game::dto::DeleteGameQuery,
        api::support::ownership::require_can_delete,
        server::AppState,
    },
};


pub async fn delete_game_draft(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(game_id): AxumPath<i64>,
    Query(query): Query<DeleteGameQuery>,
) -> Result<StatusCode, GameError> {
    let post_id = require_can_delete(
        &state.game_service.pool,
        "games",
        game_id,
        &claims,
        GameError::InternalError,
        GameError::GameNotFound,
        GameError::Forbidden,
    )
    .await?;
    let row = sqlx::query_as::<_, (String, String, String, Option<String>)>(
        "SELECT posts.content_kind, posts.title, posts.slug, posts.deleted_at FROM posts JOIN games ON games.post_id = posts.id WHERE games.id = ?",
    )
    .bind(game_id)
    .fetch_optional(&state.game_service.pool)
    .await?
    .ok_or(GameError::GameNotFound)?;
    // For legacy shared posts (project and game share same post_id with content_kind='project'),
    // allow deletion via the game endpoint — the kind guard would otherwise block it.
    let is_shared = sqlx::query_scalar::<_, Option<i64>>("SELECT id FROM projects WHERE post_id = ?")
        .bind(post_id)
        .fetch_optional(&state.game_service.pool)
        .await?
        .is_some();
    if row.0 != "game" && !(is_shared && row.0 == "project") {
        return Err(GameError::InvalidDemo(
            "Use the typed delete endpoint for this content kind.".to_string(),
        ));
    }
    if row.3.is_some() && !is_shared {
        return Err(GameError::Conflict("Game already in trash.".to_string()));
    }
    // Shared-post game: hard-delete the game row only, keep the post for the project
    if is_shared {
        // check delegated published projects (includes the shared project itself)
        let delegating: Vec<i64> = sqlx::query_scalar(
            "SELECT projects.id FROM projects JOIN posts p ON p.id = projects.post_id WHERE projects.delegate_game_id = ? AND p.deleted_at IS NULL AND p.status = 'published'",
        )
        .bind(game_id)
        .fetch_all(&state.game_service.pool)
        .await?;
        if !delegating.is_empty() && query.force != Some(true) {
            return Err(GameError::Conflict(format!(
                "Game is delegated by {} published project(s); use ?force=true to confirm.",
                delegating.len()
            )));
        }
        let delegating_json = serde_json::to_string(&delegating).unwrap_or_else(|_| "[]".to_string());
        sqlx::query(
            "INSERT INTO game_deletion_log (game_id, slug, title, reason, detail, deleted_by, delegated_project_ids) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(game_id)
        .bind(&row.2)
        .bind(&row.1)
        .bind(query.reason.clone().unwrap_or_else(|| "user_request".to_string()))
        .bind(&query.detail)
        .bind(claims.user_id.parse::<i64>().unwrap_or(0))
        .bind(&delegating_json)
        .execute(&state.game_service.pool)
        .await
        .ok();
        sqlx::query("DELETE FROM games WHERE id = ?")
            .bind(game_id)
            .execute(&state.game_service.pool)
            .await?;
        let dir = state.project_demo_config.dir.join(format!("game-{}", game_id));
        let _ = tokio::fs::remove_dir_all(&dir).await;
        return Ok(StatusCode::NO_CONTENT);
    }
    let reason = query.reason.unwrap_or_else(|| "user_request".to_string());
    let allowed = ["user_request", "dmca", "moderation", "replaced", "other"];
    if !allowed.contains(&reason.as_str()) {
        return Err(GameError::InvalidDemo("Invalid deletion reason.".to_string()));
    }
    // check delegated published projects
    let delegating: Vec<i64> = sqlx::query_scalar(
        "SELECT projects.id FROM projects JOIN posts p ON p.id = projects.post_id WHERE projects.delegate_game_id = ? AND p.deleted_at IS NULL AND p.status = 'published'",
    )
    .bind(game_id)
    .fetch_all(&state.game_service.pool)
    .await?;
    if !delegating.is_empty() && query.force != Some(true) {
        return Err(GameError::Conflict(format!(
            "Game is delegated by {} published project(s); use ?force=true to confirm.",
            delegating.len()
        )));
    }
    // capture delegated ids for log (even before soft-delete, keep for later hard purge)
    let delegating_json = serde_json::to_string(&delegating).unwrap_or_else(|_| "[]".to_string());
    // log for tombstone (also survives hard purge)
    sqlx::query(
        "INSERT INTO game_deletion_log (game_id, slug, title, reason, detail, deleted_by, delegated_project_ids) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(game_id)
    .bind(&row.2)
    .bind(&row.1)
    .bind(&reason)
    .bind(&query.detail)
    .bind(claims.user_id.parse::<i64>().unwrap_or(0))
    .bind(&delegating_json)
    .execute(&state.game_service.pool)
    .await
    .ok();
    sqlx::query(
        "UPDATE posts SET deleted_at = CURRENT_TIMESTAMP, deletion_reason = ?, deletion_detail = ?, deleted_by = ?, scheduled_purge_at = datetime('now','+7 days'), prev_status = status WHERE id = ?",
    )
    .bind(&reason)
    .bind(&query.detail)
    .bind(claims.user_id.parse::<i64>().unwrap_or(0))
    .bind(post_id)
    .execute(&state.game_service.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn restore_game(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(game_id): AxumPath<i64>,
) -> Result<StatusCode, GameError> {
    let post_id = require_can_delete(
        &state.game_service.pool,
        "games",
        game_id,
        &claims,
        GameError::InternalError,
        GameError::GameNotFound,
        GameError::Forbidden,
    )
    .await?;
    let deleted_at: Option<String> = sqlx::query_scalar("SELECT deleted_at FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_optional(&state.game_service.pool)
        .await?
        .ok_or(GameError::GameNotFound)?;
    if deleted_at.is_none() {
        return Err(GameError::Conflict("Game is not in trash.".to_string()));
    }
    sqlx::query(
        "UPDATE posts SET deleted_at = NULL, deletion_reason = NULL, deletion_detail = NULL, deleted_by = NULL, scheduled_purge_at = NULL, prev_status = NULL WHERE id = ?",
    )
    .bind(post_id)
    .execute(&state.game_service.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn purge_game_now(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(game_id): AxumPath<i64>,
) -> Result<StatusCode, GameError> {
    if claims.role != "admin" {
        return Err(GameError::Forbidden);
    }
    let post_id: Option<i64> = sqlx::query_scalar("SELECT post_id FROM games WHERE id=?")
        .bind(game_id)
        .fetch_optional(&state.game_service.pool)
        .await?
        .ok_or(GameError::GameNotFound)?;
    sqlx::query("DELETE FROM posts WHERE id = ?")
        .bind(post_id)
        .execute(&state.game_service.pool)
        .await?;
    let dir = state.project_demo_config.dir.join(format!("game-{}", game_id));
    let _ = tokio::fs::remove_dir_all(&dir).await;
    Ok(StatusCode::NO_CONTENT)
}

