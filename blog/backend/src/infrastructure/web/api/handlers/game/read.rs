// Game read endpoints: public slugs and details, admin listings.
use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path as AxumPath, Query, State},
    response::IntoResponse,
};

use crate::{
    application::{
        commands::{
            game::{
                GetFeaturedGamesCommand, GetGameBySlugCommand, GetGameDetailsCommand,
                GetLatestGamesCommand,
            },
            post::CheckSlugCommand,
        },
        services::{game::GameService, post::PostService},
    },
    domain::{entities::secret::Claims, errors::game::GameError},
    infrastructure::web::{
        api::handlers::v86::runtime_descriptor,
        api::handlers::game::response::game_response,
        api::handlers::game::dto::{
            CheckQuery, CheckResponse, FeaturedGamesQuery, FeaturedGamesResponse,
            LatestGamesQuery, LatestGamesResponse,
        },
        server::AppState,
    },
};

pub async fn check_game(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CheckQuery>,
) -> Result<impl IntoResponse, GameError> {
    if let Some(post_slug) = query.slug {
        let exists = state
            .post_service
            .check_slug(CheckSlugCommand { post_slug })
            .await?;
        Ok(Json(CheckResponse { exists }))
    } else {
        Ok(Json(CheckResponse { exists: true }))
    }
}

pub async fn get_game_by_slug(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    AxumPath(slug): AxumPath<String>,
) -> Result<impl IntoResponse, GameError> {
    // The viewer is identified so the response can report `is_owner`. This used
    // to sit behind a `with_draft` flag that also returned the unpublished
    // body; with a single body there is nothing left to gate, so the viewer is
    // simply whoever the token says.
    let as_id = match opt_claims {
        Some(claims) => Some(
            claims
                .user_id
                .parse::<i64>()
                .map_err(|_| GameError::InternalError("Cannot parse id".to_string()))?,
        ),
        None => None,
    };

    let game = state
        .game_service
        .get_game_by_slug(GetGameBySlugCommand { slug, as_id })
        .await?;
    let mut response = game_response(game);
    if response.launcher_type == "v86" {
        response.v86_runtime = runtime_descriptor(
            &state.game_service.pool,
            &response.slug,
            state.artifact_base_url(),
        )
        .await?;
        let game = sqlx::query(
            "SELECT system_version_id, manifest_text, artifact_revision FROM game_v86_games WHERE game_id = ?",
        )
        .bind(response.id)
        .fetch_optional(&state.game_service.pool)
        .await?;
        if let Some(game) = game {
            use sqlx::Row;
            response.v86_system_version_id = Some(game.get("system_version_id"));
            response.v86_manifest = Some(game.get("manifest_text"));
            response.v86_artifact_revision = Some(game.get("artifact_revision"));
        }
    }
    Ok(Json(response))
}

pub async fn get_game_details(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(game_id): AxumPath<i64>,
) -> Result<impl IntoResponse, GameError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse id".to_string()))?;
    let is_admin = claims.role == "admin";
    let game = state
        .game_service
        .get_game_details(GetGameDetailsCommand {
            game_id,
            viewing_user_id: user_id,
            required_author_id: if is_admin { None } else { Some(user_id) },
        })
        .await?;
    let mut response = game_response(game);
    if response.launcher_type == "v86" {
        let game = sqlx::query(
            "SELECT system_version_id, manifest_text, artifact_revision FROM game_v86_games WHERE game_id = ?",
        )
        .bind(response.id)
        .fetch_optional(&state.game_service.pool)
        .await?;
        if let Some(game) = game {
            use sqlx::Row;
            response.v86_system_version_id = Some(game.get("system_version_id"));
            response.v86_manifest = Some(game.get("manifest_text"));
            response.v86_artifact_revision = Some(game.get("artifact_revision"));
        }
    }
    Ok(Json(response))
}

pub async fn get_latest_games(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LatestGamesQuery>,
) -> Result<impl IntoResponse, GameError> {
    let games = state
        .game_service
        .get_latest_game_snapshots(GetLatestGamesCommand {
            limit: query.limit.unwrap_or(24),
            offset: query.offset.unwrap_or(0),
            public_only: true,
            required_author_id: None,
        })
        .await?;
    Ok(Json(LatestGamesResponse {
        games: games.games.into_iter().map(Into::into).collect(),
        has_more: games.has_more,
    }))
}

pub async fn get_featured_games(
    State(state): State<Arc<AppState>>,
    Query(query): Query<FeaturedGamesQuery>,
) -> Result<impl IntoResponse, GameError> {
    let games = state
        .game_service
        .get_featured_game_snapshots(GetFeaturedGamesCommand {
            limit: query.limit.unwrap_or(5),
        })
        .await?;

    Ok(Json(FeaturedGamesResponse {
        featured_games: games.into_iter().map(Into::into).collect(),
        has_more: false,
    }))
}

pub async fn get_all_games(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<LatestGamesQuery>,
) -> Result<impl IntoResponse, GameError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse id".to_string()))?;
    let games = state
        .game_service
        .get_latest_game_snapshots(GetLatestGamesCommand {
            limit: query.limit.unwrap_or(100),
            offset: query.offset.unwrap_or(0),
            public_only: false,
            required_author_id: (claims.role != "admin").then_some(user_id),
        })
        .await?;
    Ok(Json(LatestGamesResponse {
        games: games.games.into_iter().map(Into::into).collect(),
        has_more: games.has_more,
    }))
}

