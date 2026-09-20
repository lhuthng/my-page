// Lifecycle transitions: publish a draft and toggle the featured flag.
use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path as AxumPath, State},
};

use crate::{
    application::{
        commands::{
            post::PublishCommand,
            game::{GetGamePostIdCommand, SetFeaturedGameCommand},
        },
        services::{game::GameService, post::PostService},
    },
    domain::{entities::secret::Claims, errors::game::GameError},
    infrastructure::web::{
        api::handlers::game::dto::SetGameFeaturedBody,
        server::AppState,
    },
};

#[axum::debug_handler]
pub async fn publish_game(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(game_id): AxumPath<i64>,
) -> Result<(), GameError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse id".to_string()))?;
    let post_id = state
        .game_service
        .get_game_post_id(GetGamePostIdCommand {
            game_id,
            required_author_id: Some(user_id),
        })
        .await?;
    let launcher_type: String = sqlx::query_scalar("SELECT launcher_type FROM games WHERE id = ?")
        .bind(game_id)
        .fetch_one(&state.game_service.pool)
        .await?;
    if launcher_type == "jsdos" {
        let has_bundle: Option<i64> =
            sqlx::query_scalar("SELECT game_id FROM game_jsdos_bundles WHERE game_id = ?")
                .bind(game_id)
                .fetch_optional(&state.game_service.pool)
                .await?;
        if has_bundle.is_none() {
            return Err(GameError::InvalidDemo(
                "A completed js-dos bundle is required before publishing.".to_string(),
            ));
        }
    }
    if launcher_type == "v86" {
        let has_game: Option<i64> =
            sqlx::query_scalar("SELECT game_id FROM game_v86_games WHERE game_id = ?")
                .bind(game_id)
                .fetch_optional(&state.game_service.pool)
                .await?;
        if has_game.is_none() {
            return Err(GameError::InvalidDemo(
                "A completed v86 game artifact is required before publishing.".to_string(),
            ));
        }
    }
    state
        .post_service
        .publish(PublishCommand { user_id, post_id })
        .await?;
    Ok(())
}

pub async fn set_game_featured(
    State(state): State<Arc<AppState>>,
    AxumPath(game_id): AxumPath<i64>,
    Json(body): Json<SetGameFeaturedBody>,
) -> Result<impl IntoResponse, GameError> {
    state
        .game_service
        .set_game_featured(SetFeaturedGameCommand {
            game_id,
            is_featured: body.is_featured,
        })
        .await?;
    Ok(())
}

