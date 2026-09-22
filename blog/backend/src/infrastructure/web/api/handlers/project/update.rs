// Updating a project: multipart intake, delegated-game and demo-type
// validation. Creation lives in `write`.
use std::{collections::HashMap, sync::Arc};

use axum::{
    Extension, Json,
    extract::{Multipart, Path as AxumPath, State},
    response::IntoResponse,
};

use crate::{
    application::{
        commands::{
            post::{UpdatePostCommand, UpdatePostCoverCommand},
            project::{GetProjectPostIdCommand, UpdateProjectCommand},
        },
        services::{post::PostService, project::ProjectService},
    },
    domain::{entities::secret::Claims, errors::project::ProjectError},
    infrastructure::web::{
        api::handlers::project::dto::ProjectPatchData,
        api::handlers::project::response::UpdateProjectResponse,
        api::handlers::project::write::normalize_links,
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
pub async fn update_project(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(project_id): AxumPath<i64>,
    multipart: Multipart,
) -> Result<impl IntoResponse, ProjectError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| ProjectError::InternalError("Cannot parse id".to_string()))?;

    // Authorise before reading any project internals or buffering the upload.
    let post_id = state
        .project_service
        .get_project_post_id(GetProjectPostIdCommand {
            project_id,
            required_author_id: Some(user_id),
        })
        .await?;

    let parsed = parse_multipart::<ProjectPatchData, ProjectError>(
        multipart,
        "project_data",
        "No project data is given.",
        ProjectError::InternalError,
        ProjectError::UploadFailed,
    )
    .await?;
    let mut data = parsed.data;
    let has_demo_url = data.demo_url.as_ref().is_some_and(|u| !u.trim().is_empty());
    let has_demo_attachments = parsed.demo_zip.is_some() || has_demo_url;
    if let Some(ref demo_type) = data.demo_type {
        if has_demo_attachments {
            match demo_type.as_str() {
                "none" => {
                    return Err(ProjectError::InvalidDemo(
                        "Demo attachments are not accepted for projects without demos.".to_string(),
                    ));
                }
                "html5" | "webgl" => {
                    if data.demo_url.is_some() {
                        return Err(ProjectError::InvalidDemo(format!(
                            "Demo URL is not accepted for {} projects.",
                            demo_type
                        )));
                    }
                    if parsed.demo_zip.is_none() {
                        return Err(ProjectError::InvalidDemo(format!(
                            "Demo zip is required for {} projects.",
                            demo_type
                        )));
                    }
                }
                "embed" | "download" | "video" => {
                    let has_url = data.demo_url.as_ref().is_some_and(|u| !u.trim().is_empty());
                    if !has_url {
                        return Err(ProjectError::InvalidDemo(format!(
                            "Demo URL is required for {} projects.",
                            demo_type
                        )));
                    }
                }
                "game" => {
                    if has_demo_url || parsed.demo_zip.is_some() {
                        return Err(ProjectError::InvalidDemo(
                            "Delegated projects play their game's launcher; demo attachments are not accepted."
                                .to_string(),
                        ));
                    }
                    let Some(game_id) = data.delegate_game_id else {
                        return Err(ProjectError::InvalidDemo(
                            "A delegated project must select a game.".to_string(),
                        ));
                    };
                    let exists: Option<i64> =
                        sqlx::query_scalar("SELECT id FROM games WHERE id = ?")
                            .bind(game_id)
                            .fetch_optional(&state.project_service.pool)
                            .await?;
                    if exists.is_none() {
                        return Err(ProjectError::InvalidDemo(format!(
                            "The selected game ({game_id}) does not exist."
                        )));
                    }
                }
                _ => {
                    return Err(ProjectError::InvalidDemo(format!(
                        "Unsupported demo type: {}",
                        demo_type
                    )));
                }
            }
        }
    } else if has_demo_attachments {
        return Err(ProjectError::InvalidDemo(
            "Demo type is required when providing demo attachments.".to_string(),
        ));
    }

    upload_inline_media(
        &state,
        user_id,
        data.number_of_files,
        &parsed.files,
        &parsed.short_names,
        ProjectError::UploadFailed,
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
            // Ownership was already established by get_project_post_id above.
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

    // Validate the client-supplied URL before it can be replaced by the
    // locally-extracted demo path below.
    let mut demo_url = validate_demo_url(data.demo_url, ProjectError::InvalidDemo)?
        .filter(|u| !u.trim().is_empty());
    if parsed.demo_zip.is_some() {
        let local_demo_url = state
            .project_demo_config
            .dir
            .join(project_id.to_string())
            .join("index.html");
        demo_url = Some(local_demo_url.to_str().unwrap_or("").to_string());
    }

    state
        .project_service
        .update_project(UpdateProjectCommand {
            project_id,
            user_id,
            demo_type: data.demo_type,
            demo_entry_path: None,
            demo_width: data.demo_width,
            demo_height: data.demo_height,
            demo_config: data.demo_config,
            demo_url,
            delegate_game_id: data.delegate_game_id,
            inherit_thumbnail: data.inherit_thumbnail,
            inherit_tags: data.inherit_tags,
            links: data.links.map(normalize_links).transpose()?,
        })
        .await?;

    if let Some(zip) = parsed.demo_zip {
        extract_demo_zip(
            &state.project_demo_config,
            project_id.to_string(),
            zip,
            ProjectError::InternalError,
            ProjectError::InvalidDemo,
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

    Ok(Json(UpdateProjectResponse { updated_at }))
}
