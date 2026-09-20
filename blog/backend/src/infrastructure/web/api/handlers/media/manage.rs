// Media detail management: get and change a medium's description.
use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, State},
    response::IntoResponse,
};

use crate::{
    application::{
        commands::media::{ChangeMediaDetailsCommand, GetMediaDetailsCommand},
        services::media::MediaService,
    },
    domain::{
        entities::secret::Claims,
        errors::media::MediaError,
    },
    infrastructure::web::{
        api::handlers::media::dto::{ChangeDetailsPayload, GetMediaDetailsResponse},
        server::AppState,
    },
};

#[axum::debug_handler]
pub async fn get_details(
    State(state): State<Arc<AppState>>,
    Path(short_name): Path<String>,
) -> Result<impl IntoResponse, MediaError> {
    let details = state
        .media_service
        .get_details(GetMediaDetailsCommand { short_name })
        .await?;

    Ok(Json(GetMediaDetailsResponse {
        short_name: details.short_name,
        description: details.description,
        file_type: details.file_type,
        aliases: details.aliases,
    }))
}

#[derive(Deserialize)]
pub struct ChangeDetailsPayload {
    pub new_short_name: Option<String>,
    pub description: Option<String>,
}

#[axum::debug_handler]
pub async fn change_details(
    State(state): State<Arc<AppState>>,
    Path(short_name): Path<String>,
    Json(payload): Json<ChangeDetailsPayload>,
) -> Result<(), MediaError> {
    let cmd = ChangeMediaDetailsCommand {
        short_name,
        new_short_name: payload.new_short_name,
        description: payload.description,
    };

    state.media_service.change_details(cmd).await
}

