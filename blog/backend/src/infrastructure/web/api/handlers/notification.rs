// In-app notifications: list, unread count, and read markers. Every handler
// scopes to the session's user; ids from other accounts answer 404.
use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    middleware,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use crate::{
    domain::{entities::secret::Claims, errors::notification::NotificationError},
    helper::{
        string::clamp_page_size,
        time::{normalize_optional_utc_timestamp, normalize_utc_timestamp},
    },
    infrastructure::{
        persistence::notification as store,
        web::{api::middlewares, server::AppState},
    },
};

const DEFAULT_PAGE_SIZE: i64 = 20;
const MAX_PAGE_SIZE: i64 = 50;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    before: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct NotificationActor {
    username: Option<String>,
    display_name: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct NotificationPost {
    title: String,
    slug: String,
    kind: String,
}

#[derive(Debug, Serialize)]
pub struct NotificationComment {
    id: i64,
    content: String,
}

#[derive(Debug, Serialize)]
pub struct NotificationItem {
    id: i64,
    kind: String,
    created_at: String,
    read_at: Option<String>,
    actor: NotificationActor,
    post: NotificationPost,
    comment: NotificationComment,
}

#[derive(Debug, Serialize)]
pub struct NotificationsResponse {
    items: Vec<NotificationItem>,
    has_more: bool,
}

#[derive(Debug, Serialize)]
pub struct UnreadCountResponse {
    unread: i64,
}

#[derive(Debug, Serialize)]
pub struct MarkAllReadResponse {
    updated: u64,
}

fn user_id(claims: &Claims) -> Result<i64, NotificationError> {
    claims
        .user_id
        .parse::<i64>()
        .map_err(|e| NotificationError::InternalError(e.to_string()))
}

fn to_item(row: store::NotificationRow) -> NotificationItem {
    NotificationItem {
        id: row.id,
        kind: row.kind,
        created_at: normalize_utc_timestamp(row.created_at),
        read_at: normalize_optional_utc_timestamp(row.read_at),
        actor: NotificationActor {
            username: row.actor_username,
            display_name: row.actor_display_name,
            avatar_url: row.actor_avatar_url,
        },
        post: NotificationPost {
            title: row.post_title,
            slug: row.post_slug,
            kind: row.post_kind,
        },
        comment: NotificationComment {
            id: row.comment_id,
            content: row.comment_content,
        },
    }
}

pub async fn list_notifications(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<ListQuery>,
) -> Result<Json<NotificationsResponse>, NotificationError> {
    let user_id = user_id(&claims)?;
    let limit = clamp_page_size(query.limit, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE);

    let mut rows =
        store::list_notifications(&state.post_service.pool, user_id, query.before, limit).await?;

    let has_more = rows.len() > limit as usize;
    if has_more {
        rows.truncate(limit as usize);
    }

    Ok(Json(NotificationsResponse {
        items: rows.into_iter().map(to_item).collect(),
        has_more,
    }))
}

pub async fn unread_count(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<UnreadCountResponse>, NotificationError> {
    let user_id = user_id(&claims)?;
    let unread = store::count_unread(&state.post_service.pool, user_id).await?;
    Ok(Json(UnreadCountResponse { unread }))
}

pub async fn mark_one_read(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(notification_id): Path<i64>,
) -> Result<StatusCode, NotificationError> {
    let user_id = user_id(&claims)?;
    let found = store::mark_read(&state.post_service.pool, user_id, notification_id).await?;
    if !found {
        return Err(NotificationError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn mark_all_read(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<MarkAllReadResponse>, NotificationError> {
    let user_id = user_id(&claims)?;
    let updated = store::mark_all_read(&state.post_service.pool, user_id).await?;
    Ok(Json(MarkAllReadResponse { updated }))
}

pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_notifications))
        .route("/unread-count", get(unread_count))
        .route("/read-all", post(mark_all_read))
        .route("/{id}/read", post(mark_one_read))
        .layer(middleware::from_fn_with_state(
            state,
            middlewares::auth::user_guard,
        ))
}
