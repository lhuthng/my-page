// Updating a game (multipart intake, launcher validation, v86 package and
// system switch handling). Creation lives in `write`.
use std::{collections::HashMap, sync::Arc};

use axum::{
    Extension, Json,
    extract::{Multipart, Path as AxumPath, State},
    response::IntoResponse,
};

use crate::{
    application::{
        commands::{
            game::{GetGamePostIdCommand, UpdateGameCommand},
            post::{UpdatePostCommand, UpdatePostCoverCommand},
        },
        services::{game::GameService, post::PostService},
    },
    domain::{entities::secret::Claims, errors::game::GameError},
    infrastructure::web::{
        api::handlers::game::dto::GamePatchData,
        api::handlers::game::response::UpdateGameResponse,
        api::handlers::game::write::normalize_links,
        api::handlers::v86::attach_ready_game_tx,
        api::support::{
            demo_archive::extract_demo_zip,
            links::validate_demo_url,
            media_short_names::replace_media_short_names,
            multipart::{parse_multipart, upload_inline_media},
        },
        server::AppState,
    },
};

#[axum::debug_handler]
pub async fn update_game(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(game_id): AxumPath<i64>,
    multipart: Multipart,
) -> Result<impl IntoResponse, GameError> {
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

    let parsed = parse_multipart::<GamePatchData, GameError>(
        multipart,
        "game_data",
        "No game data is given.",
        GameError::InternalError,
        GameError::UploadFailed,
    )
    .await?;
    let mut data = parsed.data;
    let current_launcher_type: String =
        sqlx::query_scalar("SELECT launcher_type FROM games WHERE id = ?")
            .bind(game_id)
            .fetch_optional(&state.game_service.pool)
            .await?
            .ok_or(GameError::GameNotFound)?;
    let effective_launcher_type = data
        .launcher_type
        .as_deref()
        .unwrap_or(current_launcher_type.as_str())
        .to_string();

    let has_demo_url = data.demo_url.as_ref().is_some_and(|u| !u.trim().is_empty());
    let has_demo_attachments = parsed.demo_zip.is_some() || has_demo_url;
    if let Some(ref launcher_type) = data.launcher_type {
        if has_demo_attachments {
            match launcher_type.as_str() {
                "html5" | "webgl" => {
                    if data.demo_url.is_some() {
                        return Err(GameError::InvalidDemo(format!(
                            "Demo URL is not accepted for {} games.",
                            launcher_type
                        )));
                    }
                    if parsed.demo_zip.is_none() {
                        return Err(GameError::InvalidDemo(format!(
                            "Demo zip is required for {} games.",
                            launcher_type
                        )));
                    }
                }
                "embed" | "download" | "video" => {
                    let has_url = data.demo_url.as_ref().is_some_and(|u| !u.trim().is_empty());
                    if !has_url {
                        return Err(GameError::InvalidDemo(format!(
                            "Demo URL is required for {} games.",
                            launcher_type
                        )));
                    }
                }
                "jsdos" => {
                    if has_demo_url || parsed.demo_zip.is_some() {
                        return Err(GameError::InvalidDemo(
                            "js-dos bundles must be uploaded through the js-dos upload endpoint."
                                .to_string(),
                        ));
                    }
                }
                "v86" => {
                    if has_demo_url || parsed.demo_zip.is_some() {
                        return Err(GameError::InvalidDemo(
                            "v86 games must be uploaded through the v86 package endpoint."
                                .to_string(),
                        ));
                    }
                }
                _ => {
                    return Err(GameError::InvalidDemo(format!(
                        "Unsupported launcher type: {}",
                        launcher_type
                    )));
                }
            }
        }
    } else if has_demo_attachments {
        return Err(GameError::InvalidDemo(
            "Launcher type is required when providing demo attachments.".to_string(),
        ));
    }

    upload_inline_media(
        &state,
        user_id,
        data.number_of_files,
        &parsed.files,
        &parsed.short_names,
        GameError::UploadFailed,
    )
    .await?;

    let mut media_usage = None;
    if let Some(content) = data.content.as_mut() {
        let mut usage = HashMap::<String, i64>::new();
        replace_media_short_names(content, &mut usage);
        media_usage = Some(usage);
    }

    let updated_at = state
        .post_service
        .update_post(UpdatePostCommand {
            user_id,
            required_author_id: Some(user_id),
            expected_updated_at: data.expected_updated_at.take(),
            post_id,
            title: data.title,
            slug: data.slug,
            excerpt: data.excerpt,
            content: data.content,
            tags: data.tags,
            media_usage,
        })
        .await?;

    let mut demo_url =
        validate_demo_url(data.demo_url, GameError::InvalidDemo)?.filter(|u| !u.trim().is_empty());
    if parsed.demo_zip.is_some() {
        let local_demo_url = state
            .project_demo_config
            .dir
            .join(format!("game-{game_id}"))
            .join("index.html");
        demo_url = Some(local_demo_url.to_str().unwrap_or("").to_string());
    }

    let keeps_jsdos_bundle = effective_launcher_type == "jsdos";
    let keeps_v86_game = effective_launcher_type == "v86";

    state
        .game_service
        .update_game(UpdateGameCommand {
            game_id,
            user_id,
            launcher_type: data.launcher_type,
            demo_width: data.demo_width,
            demo_height: data.demo_height,
            demo_url,
            instruction: data.instruction,
            cheatcode: data.cheatcode,
            story: data.story,
            related_games: data.related_games.map(normalize_links).transpose()?,
        })
        .await?;

    if data.v86_upload_id.is_some() && data.v86_system_version_id.is_some() {
        return Err(GameError::InvalidDemo(
            "Send either a v86 package upload or a system switch, not both.".to_string(),
        ));
    }

    if let Some(upload_id) = data.v86_upload_id.as_deref() {
        if !keeps_v86_game {
            return Err(GameError::InvalidDemo(
                "A v86 package cannot be attached to a non-v86 game.".to_string(),
            ));
        }
        let mut tx = state.game_service.pool.begin().await?;
        attach_ready_game_tx(
            &mut tx,
            game_id,
            user_id,
            upload_id,
            state.project_demo_config.v86_download_chunk_size,
        )
        .await?;
        tx.commit().await?;
    }

    // A system switch re-points the existing package: disk and launcher ISOs
    // are content-addressed and OS-agnostic within a platform, so nothing is
    // re-uploaded. When the launcher type moves away from v86, the artifact
    // row is deleted below and the field is moot.
    if let Some(system_version_id) = data.v86_system_version_id
        && keeps_v86_game
    {
        repoint_game_system(&state.game_service.pool, game_id, system_version_id).await?;
    }

    if !keeps_jsdos_bundle
        && let Some(storage_key) = sqlx::query_scalar::<_, String>(
            "SELECT storage_key FROM game_jsdos_bundles WHERE game_id = ?",
        )
        .bind(game_id)
        .fetch_optional(&state.game_service.pool)
        .await?
    {
        sqlx::query("DELETE FROM game_jsdos_bundles WHERE game_id = ?")
            .bind(game_id)
            .execute(&state.game_service.pool)
            .await?;
        tokio::fs::remove_file(state.project_demo_config.dir.join(storage_key))
            .await
            .ok();
    }

    if !keeps_v86_game {
        sqlx::query("DELETE FROM game_v86_games WHERE game_id = ?")
            .bind(game_id)
            .execute(&state.game_service.pool)
            .await?;
    }

    if let Some(zip) = parsed.demo_zip {
        extract_demo_zip(
            &state.project_demo_config,
            format!("game-{game_id}"),
            zip,
            GameError::InternalError,
            GameError::InvalidDemo,
        )
        .await?;
    }

    if data.og_image_seconds.is_some() {
        state
            .post_service
            .update_post_cover(UpdatePostCoverCommand {
                user_id,
                post_id,
                og_image_seconds: data.og_image_seconds,
            })
            .await?;
    }

    Ok(Json(UpdateGameResponse { updated_at }))
}

/// Re-point a game's v86 artifact at another system version. Only the base
/// OS image changes; the game disk and launcher ISOs stay content-addressed
/// as-is, so this is a plain row update. Bumping `artifact_revision` mirrors
/// `attach_ready_game_tx` and makes any in-flight package build or snapshot
/// capture fail its revision/system checks instead of half-landing.
pub(super) async fn repoint_game_system(
    pool: &sqlx::SqlitePool,
    game_id: i64,
    system_version_id: i64,
) -> Result<(), GameError> {
    let current: Option<(i64, String)> = sqlx::query_as(
        "SELECT g.system_version_id, s.platform_key
         FROM game_v86_games g
         JOIN v86_system_versions v ON v.id = g.system_version_id
         JOIN v86_systems s ON s.id = v.system_id
         WHERE g.game_id = ?",
    )
    .bind(game_id)
    .fetch_optional(pool)
    .await?;
    let Some((current_version_id, current_platform)) = current else {
        return Err(GameError::InvalidDemo(
            "A completed v86 game artifact is required before switching systems.".to_string(),
        ));
    };

    if current_version_id == system_version_id {
        return Ok(());
    }

    let target: Option<(String, bool)> = sqlx::query_as(
        "SELECT s.platform_key, s.is_active
         FROM v86_system_versions v
         JOIN v86_systems s ON s.id = v.system_id
         WHERE v.id = ?",
    )
    .bind(system_version_id)
    .fetch_optional(pool)
    .await?;
    let Some((target_platform, target_active)) = target else {
        return Err(GameError::InvalidDemo(
            "The selected v86 system version does not exist.".to_string(),
        ));
    };
    if !target_active {
        return Err(GameError::InvalidDemo(
            "The selected v86 system is not active.".to_string(),
        ));
    }
    if target_platform != current_platform {
        return Err(GameError::InvalidDemo(
            "A v86 game can only be switched between systems of the same platform.".to_string(),
        ));
    }

    let mut tx = pool.begin().await?;
    let updated = sqlx::query(
        "UPDATE game_v86_games
         SET system_version_id = ?, artifact_revision = artifact_revision + 1,
             updated_at = CURRENT_TIMESTAMP
         WHERE game_id = ?",
    )
    .bind(system_version_id)
    .bind(game_id)
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(GameError::InvalidDemo(
            "The v86 artifact changed while switching systems.".to_string(),
        ));
    }
    tx.commit().await?;
    Ok(())
}
