use std::sync::Arc;

use axum::{Extension, Json, extract::State};
use serde::Serialize;

use crate::{
    application::{commands::user::MeCommand, services::user::UserService},
    domain::{entities::secret::Claims, errors::user::UserError},
    infrastructure::web::server::AppState,
};

#[derive(Debug, Serialize)]
pub struct MeResponse {
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

    match state.user_service.me(MeCommand { user_id }).await {
        Ok(me) => Ok(Json(MeResponse {
            display_name: me.display_name,
            role: me.role,
        })),
        Err(e) => Err(e),
    }
}
