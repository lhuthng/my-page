// Creating a game (multipart intake + post/game service calls) and changing
// its cover. Updates live in `update`, the delete lifecycle in `trash`.
use std::{
    collections::HashMap,
    sync::Arc,
};

use axum::{
    Extension, Json,
    extract::{Multipart, Path as AxumPath, State},
    response::IntoResponse,
};

use crate::{
    application::{
        commands::{
            media::ChangePostCoverCommand,
            post::{CheckSlugCommand, NewPostCommand, UpdatePostCoverCommand},
            game::NewGameCommand,
        },
        services::{game::GameService, media::MediaService, post::PostService},
    },
    domain::{
        entities::{
            game::GameLink,
            media::MediumDetails,
            secret::Claims,
        },
        errors::{game::GameError, media::MediaError},
    },
    infrastructure::web::{
        api::handlers::support::cover::{
            MediumData, apply_created_cover_upload, extract_medium,
        },
        api::handlers::game::dto::GameData,
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

pub(super) const MAX_GAME_LINKS: usize = 20;

pub(super) fn normalize_links(links: Vec<GameLink>) -> Result<Vec<GameLink>, GameError> {
    let kept: Vec<GameLink> = links
        .into_iter()
        .filter(|link| !link.title.trim().is_empty() && !link.slug.trim().is_empty())
        .collect();

    if kept.len() > MAX_GAME_LINKS {
        return Err(GameError::InvalidDemo(format!(
            "A game may have at most {MAX_GAME_LINKS} related games."
        )));
    }

    Ok(kept)
}

#[axum::debug_handler]
pub async fn new_game(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    multipart: Multipart,
) -> Result<impl IntoResponse, GameError> {
    let uploader_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse id".to_string()))?;
    let parsed = parse_multipart::<GameData, GameError>(
        multipart,
        "game_data",
        "No game data is given.",
        GameError::InternalError,
        GameError::UploadFailed,
    )
    .await?;
    let mut data = parsed.data;

    if state
        .post_service
        .check_slug(CheckSlugCommand {
            post_slug: data.slug.clone(),
        })
        .await?
    {
        return Err(GameError::InvalidDemo(format!(
            "The game slug '{}' is already in use.",
            data.slug
        )));
    }

    let demo_zip = parsed.demo_zip;
    let create_cover = parsed.create_cover;
    let has_demo_url = data.demo_url.as_ref().is_some_and(|u| !u.trim().is_empty());
    match data.launcher_type.as_str() {
        "html5" | "webgl" => {
            if has_demo_url {
                return Err(GameError::InvalidDemo(format!(
                    "Demo URL is not accepted for {} games.",
                    data.launcher_type
                )));
            }
            if demo_zip.is_none() {
                return Err(GameError::InvalidDemo(format!(
                    "Demo zip is required for {} games.",
                    data.launcher_type
                )));
            }
        }
        "embed" | "download" | "video" => {
            if !has_demo_url {
                return Err(GameError::InvalidDemo(format!(
                    "Demo URL is required for {} games.",
                    data.launcher_type
                )));
            }
        }
        "jsdos" => {
            if has_demo_url || demo_zip.is_some() {
                return Err(GameError::InvalidDemo(
                    "js-dos bundles must be uploaded through the js-dos upload endpoint."
                        .to_string(),
                ));
            }
        }
        "v86" => {
            if has_demo_url || demo_zip.is_some() {
                return Err(GameError::InvalidDemo(
                    "v86 games must be uploaded through the v86 package endpoint.".to_string(),
                ));
            }
            if data.v86_upload_id.is_none() {
                return Err(GameError::InvalidDemo(
                    "A completed v86 game package is required.".to_string(),
                ));
            }
        }
        _ => {
            return Err(GameError::InvalidDemo(format!(
                "Unsupported launcher type: {}",
                data.launcher_type
            )));
        }
    }

    upload_inline_media(
        &state,
        uploader_id,
        data.number_of_files,
        &parsed.files,
        &parsed.short_names,
        GameError::UploadFailed,
    )
    .await?;

    let mut media_usage = HashMap::<String, i64>::new();
    replace_media_short_names(&mut data.content, &mut media_usage);

    let post_id = state
        .post_service
        .new_post(NewPostCommand {
            user_id: uploader_id,
            title: data.title,
            slug: data.slug,
            excerpt: data.excerpt,
            content: data.content,
            tags: data.tags,
            cover_media: None,
            media_usage,
            content_kind: "game".to_string(),
        })
        .await?;

    let game_result = state
        .game_service
        .new_game(NewGameCommand {
            post_id,
            launcher_type: data.launcher_type,
            demo_width: data.demo_width,
            demo_height: data.demo_height,
            demo_url: validate_demo_url(data.demo_url, GameError::InvalidDemo)?,
            instruction: data.instruction,
            cheatcode: data.cheatcode,
            story: data.story,
            related_games: normalize_links(data.related_games)?,
        })
        .await;
    let game_id = match game_result {
        Ok(game_id) => game_id,
        Err(error) => {
            sqlx::query("DELETE FROM posts WHERE id = ?")
                .bind(post_id)
                .execute(&state.game_service.pool)
                .await
                .ok();
            return Err(error.into());
        }
    };

    if let Some(upload_id) = data.v86_upload_id.as_deref() {
        let mut tx = state.game_service.pool.begin().await?;
        let attach_result = attach_ready_game_tx(
            &mut tx,
            game_id,
            uploader_id,
            upload_id,
            state.project_demo_config.v86_download_chunk_size,
        )
        .await;
        if let Err(error) = attach_result {
            tx.rollback().await.ok();
            sqlx::query("DELETE FROM posts WHERE id = ?")
                .bind(post_id)
                .execute(&state.game_service.pool)
                .await
                .ok();
            return Err(error.into());
        }
        tx.commit().await?;
    }

    if let Some(zip) = demo_zip {
        if let Err(err) = extract_demo_zip(
            &state.project_demo_config,
            format!("game-{game_id}"),
            zip,
            GameError::InternalError,
            GameError::InvalidDemo,
        )
        .await
        {
            return Err(err);
        }
    }

    apply_created_cover_upload(&state, uploader_id, post_id, create_cover).await?;

    Ok(Json(
        serde_json::json!({ "id": game_id, "post_id": post_id }),
    ))
}

#[axum::debug_handler]
pub async fn change_cover(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(game_id): AxumPath<i64>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, GameError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse id.".to_string()))?;
    let post_id = state
        .game_service
        .get_game_post_id(GetGamePostIdCommand {
            game_id,
            required_author_id: Some(user_id),
        })
        .await?;

    let mut medium: Option<MediumData> = None;
    let mut opt_og_image_seconds: Option<i64> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| GameError::InternalError(e.to_string()))?
    {
        let field_name = field.name().ok_or(MediaError::UploadFailed(
            "Empty field detected.".to_string(),
        ))?;
        match field_name {
            "file" => {
                if medium.is_some() {
                    return Err(GameError::UploadFailed(
                        "Only one media is allowed at a time.".to_string(),
                    ));
                }
                medium = Some(extract_medium(field).await?);
            }
            "og_image_seconds" => {
                let text = field.text().await.map_err(|e| {
                    GameError::InternalError(format!("Failed to read og_image_seconds: {}", e))
                })?;
                opt_og_image_seconds = text.trim().parse::<i64>().ok();
            }
            _ => {}
        }
    }
    let MediumData {
        filename,
        content_type,
        bytes,
    } = medium.ok_or(GameError::UploadFailed("Missing file".to_string()))?;

    state
        .media_service
        .change_post_cover(
            ChangePostCoverCommand {
                post_id,
                user_id,
                medium_details: MediumDetails {
                    filename,
                    content_type,
                    bytes,
                },
                og_image_seconds: opt_og_image_seconds,
            },
            &state.media_config,
        )
        .await?;

    if let Some(og_image_seconds) = opt_og_image_seconds {
        state
            .post_service
            .update_post_cover(UpdatePostCoverCommand {
                user_id,
                post_id,
                og_image_seconds: Some(og_image_seconds),
            })
            .await?;
    }

    Ok(())
}

