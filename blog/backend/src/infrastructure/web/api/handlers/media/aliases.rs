// Media alias management: list, add, change, delete aliases per medium.
use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, State},
    response::IntoResponse,
};

use crate::{
    application::{
        commands::media::{
            AddAliasCommand, ChangeAliasCommand, DeleteAliasCommand, GetAliasesCommand,
        },
        services::media::MediaService,
    },
    domain::{
        entities::secret::Claims,
        errors::media::MediaError,
    },
    infrastructure::web::{
        api::handlers::media::dto::{
            AddAliasPayload, ChangeAliasPayload, GetAliasesResponse, MediaQuery,
        },
        server::AppState,
    },
};

#[axum::debug_handler]
pub async fn add_alias(
    State(state): State<Arc<AppState>>,
    Path(short_name): Path<String>,
    Json(payload): Json<AddAliasPayload>,
) -> Result<(), MediaError> {
    let cmd = AddAliasCommand {
        short_name,
        alias: payload.alias,
    };

    state.media_service.add_alias(cmd).await
}

#[axum::debug_handler]
pub async fn change_alias(
    State(state): State<Arc<AppState>>,
    Path((short_name, alias)): Path<(String, String)>,
    Json(payload): Json<ChangeAliasPayload>,
) -> Result<(), MediaError> {
    let cmd = ChangeAliasCommand {
        short_name,
        old_alias: alias,
        new_alias: payload.new_alias,
    };

    state.media_service.change_alias(cmd).await
}

#[axum::debug_handler]
pub async fn delete_alias(
    State(state): State<Arc<AppState>>,
    Path((short_name, alias)): Path<(String, String)>,
) -> Result<(), MediaError> {
    let cmd = DeleteAliasCommand { short_name, alias };

    state.media_service.delete_alias(cmd).await
}
