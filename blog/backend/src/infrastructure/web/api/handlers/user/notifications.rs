// Notification email preferences and the one-time onboarding prompt.
use std::sync::Arc;

use axum::{Extension, Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::{
    domain::{entities::secret::Claims, errors::user::UserError},
    infrastructure::{persistence::notification as store, web::server::AppState},
};

#[derive(Debug, Serialize)]
pub struct NotificationPreferencesResponse {
    reply_email: bool,
    mention_email: bool,
    post_comment_email: bool,
    prompt_dismissed: bool,
    show_prompt: bool,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateNotificationPreferencesBody {
    #[serde(default)]
    reply_email: Option<bool>,
    #[serde(default)]
    mention_email: Option<bool>,
    #[serde(default)]
    post_comment_email: Option<bool>,
    #[serde(default)]
    prompt_dismissed: Option<bool>,
}

fn to_response(row: store::NotificationPreferencesRow) -> NotificationPreferencesResponse {
    NotificationPreferencesResponse {
        reply_email: row.reply_email != 0,
        mention_email: row.mention_email != 0,
        post_comment_email: row.post_comment_email != 0,
        prompt_dismissed: row.prompt_dismissed != 0,
        show_prompt: row.show_prompt != 0,
    }
}

fn user_id(claims: &Claims) -> Result<i64, UserError> {
    claims
        .user_id
        .parse::<i64>()
        .map_err(|e| UserError::InternalError(e.to_string()))
}

pub async fn get_notification_preferences(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<NotificationPreferencesResponse>, UserError> {
    let user_id = user_id(&claims)?;
    let row = store::get_preferences(&state.post_service.pool, user_id)
        .await?
        .ok_or(UserError::NotFound)?;
    Ok(Json(to_response(row)))
}

pub async fn update_notification_preferences(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<UpdateNotificationPreferencesBody>,
) -> Result<Json<NotificationPreferencesResponse>, UserError> {
    let user_id = user_id(&claims)?;
    let row = store::update_preferences(
        &state.post_service.pool,
        user_id,
        body.reply_email,
        body.mention_email,
        body.post_comment_email,
        // Dismissal is one-way; re-enabling email hides the prompt anyway.
        body.prompt_dismissed.unwrap_or(false),
    )
    .await?
    .ok_or(UserError::NotFound)?;
    Ok(Json(to_response(row)))
}
