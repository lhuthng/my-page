// Creating a project (multipart intake, delegated-game validation) and
// changing its cover. Updates live in `update`, the delete lifecycle in
// `trash`.
use std::{collections::HashMap, sync::Arc};

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
            project::{GetProjectPostIdCommand, NewProjectCommand},
        },
        services::{media::MediaService, post::PostService, project::ProjectService},
    },
    domain::{
        entities::{media::MediumDetails, project::ProjectLink, secret::Claims},
        errors::{media::MediaError, project::ProjectError},
    },
    infrastructure::web::{
        api::handlers::project::dto::ProjectData,
        api::handlers::support::cover::{MediumData, apply_created_cover_upload, extract_medium},
        api::support::{
            demo_archive::extract_demo_zip,
            links::validate_demo_url,
            media_short_names::replace_media_short_names,
            multipart::{parse_multipart, upload_inline_media},
        },
        server::AppState,
    },
};

/// Upper bound on how many external links a project may list.
const MAX_PROJECT_LINKS: usize = 20;

/// Trim, drop blanks, and validate that every remaining link is an http(s) URL.
///
/// Link URLs are rendered into an `href`, so a `javascript:` or `data:` value
/// must not be storable in the first place.
pub(crate) fn normalize_links(links: Vec<ProjectLink>) -> Result<Vec<ProjectLink>, ProjectError> {
    let kept: Vec<ProjectLink> = links
        .into_iter()
        .filter(|link| !link.label.trim().is_empty() && !link.url.trim().is_empty())
        .collect();

    if kept.len() > MAX_PROJECT_LINKS {
        return Err(ProjectError::InvalidDemo(format!(
            "A project may have at most {MAX_PROJECT_LINKS} links."
        )));
    }

    kept.into_iter()
        .map(|link| {
            let (label, url) = crate::infrastructure::web::api::support::links::validate_link(
                &link.label,
                &link.url,
            )
            .map_err(ProjectError::InvalidDemo)?;
            Ok(ProjectLink { label, url })
        })
        .collect()
}

/// Validate a demo URL when one is present. An empty value is how the editor
/// clears the field, so it is passed through untouched.
pub async fn new_project(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    multipart: Multipart,
) -> Result<impl IntoResponse, ProjectError> {
    let uploader_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| ProjectError::InternalError("Cannot parse id".to_string()))?;
    let parsed = parse_multipart::<ProjectData, ProjectError>(
        multipart,
        "project_data",
        "No project data is given.",
        ProjectError::InternalError,
        ProjectError::UploadFailed,
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
        return Err(ProjectError::InvalidDemo(format!(
            "The project slug '{}' is already in use.",
            data.slug
        )));
    }

    let demo_zip = parsed.demo_zip;
    let create_cover = parsed.create_cover;
    let has_demo_url = data.demo_url.as_ref().is_some_and(|u| !u.trim().is_empty());
    match data.demo_type.as_str() {
        "none" => {
            if has_demo_url || demo_zip.is_some() {
                return Err(ProjectError::InvalidDemo(
                    "Demo attachments are not accepted for projects without demos.".to_string(),
                ));
            }
        }
        "html5" | "webgl" => {
            if has_demo_url {
                return Err(ProjectError::InvalidDemo(format!(
                    "Demo URL is not accepted for {} projects.",
                    data.demo_type
                )));
            }
            if demo_zip.is_none() {
                return Err(ProjectError::InvalidDemo(format!(
                    "Demo zip is required for {} projects.",
                    data.demo_type
                )));
            }
        }
        "embed" | "download" | "video" => {
            if !has_demo_url {
                return Err(ProjectError::InvalidDemo(format!(
                    "Demo URL is required for {} projects.",
                    data.demo_type
                )));
            }
        }
        "game" => {
            if has_demo_url || demo_zip.is_some() {
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
            let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM games WHERE id = ?")
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
                data.demo_type
            )));
        }
    }

    upload_inline_media(
        &state,
        uploader_id,
        data.number_of_files,
        &parsed.files,
        &parsed.short_names,
        ProjectError::UploadFailed,
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
            content_kind: "project".to_string(),
        })
        .await?;

    let project_result = state
        .project_service
        .new_project(NewProjectCommand {
            post_id,
            demo_type: data.demo_type,
            demo_entry_path: "index.html".to_string(),
            demo_width: data.demo_width,
            demo_height: data.demo_height,
            demo_config: data.demo_config,
            demo_url: validate_demo_url(data.demo_url, ProjectError::InvalidDemo)?,
            demo_url_dir: state
                .project_demo_config
                .dir
                .to_str()
                .unwrap_or("")
                .to_string(),
            delegate_game_id: data.delegate_game_id,
            inherit_thumbnail: data.inherit_thumbnail.unwrap_or(true),
            inherit_tags: data.inherit_tags.unwrap_or(true),
            links: normalize_links(data.links)?,
        })
        .await;
    let project_id = match project_result {
        Ok(project_id) => project_id,
        Err(error) => {
            sqlx::query("DELETE FROM posts WHERE id = ?")
                .bind(post_id)
                .execute(&state.project_service.pool)
                .await
                .ok();
            return Err(error);
        }
    };

    if let Some(zip) = demo_zip {
        extract_demo_zip(
            &state.project_demo_config,
            project_id.to_string(),
            zip,
            ProjectError::InternalError,
            ProjectError::InvalidDemo,
        )
        .await?;
    }

    apply_created_cover_upload(&state, uploader_id, post_id, create_cover).await?;

    Ok(Json(
        serde_json::json!({ "id": project_id, "post_id": post_id }),
    ))
}

#[axum::debug_handler]
pub async fn change_cover(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(project_id): AxumPath<i64>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ProjectError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| ProjectError::InternalError("Cannot parse id.".to_string()))?;
    let post_id = state
        .project_service
        .get_project_post_id(GetProjectPostIdCommand {
            project_id,
            required_author_id: Some(user_id),
        })
        .await?;

    let mut medium: Option<MediumData> = None;
    let mut opt_og_image_seconds: Option<i64> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ProjectError::InternalError(e.to_string()))?
    {
        let field_name = field.name().ok_or(MediaError::UploadFailed(
            "Empty field detected.".to_string(),
        ))?;
        match field_name {
            "file" => {
                if medium.is_some() {
                    return Err(ProjectError::UploadFailed(
                        "Only one media is allowed at a time.".to_string(),
                    ));
                }
                medium = Some(extract_medium(field).await?);
            }
            "og_image_seconds" => {
                let text = field.text().await.map_err(|e| {
                    ProjectError::InternalError(format!("Failed to read og_image_seconds: {}", e))
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
    } = medium.ok_or(ProjectError::UploadFailed("Missing file".to_string()))?;

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
