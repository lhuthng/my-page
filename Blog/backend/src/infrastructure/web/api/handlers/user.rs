use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, State},
};
use serde::Serialize;

use crate::{
    application::{
        commands::user::{GetUserCommand, MeCommand},
        services::user::UserService,
    },
    domain::{entities::secret::Claims, errors::user::UserError},
    infrastructure::web::server::AppState,
};

#[derive(Debug, Serialize)]
pub struct MeResponse {
    username: String,
    display_name: String,
    role: String,
}

#[axum::debug_handler]
pub async fn me(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<MeResponse>, UserError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|e| UserError::InternalError(e.to_string()))?;

    let cmd = MeCommand { user_id };

    match state.user_service.me(cmd).await {
        Ok(me) => Ok(Json(MeResponse {
            username: me.username,
            display_name: me.display_name,
            role: me.role,
        })),
        Err(e) => Err(e),
    }
}

#[derive(Debug, Serialize)]
pub struct GetUserResponse {
    username: String,
    display_name: String,
    bio: String,
    role: String,
}

#[axum::debug_handler]
pub async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(username): Path<String>,
) -> Result<Json<GetUserResponse>, UserError> {
    let cmd = GetUserCommand { username };
    match state.user_service.get_user(cmd).await {
        Ok(user) => Ok(Json(GetUserResponse {
            username: user.username,
            display_name: user.display_name,
            bio: user.bio,
            role: user.role,
        })),
        Err(e) => Err(e),
    }
}
