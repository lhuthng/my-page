// Updating a post: multipart intake with the same inline-media handling as
// creation, plus related-post and cover management.
use std::{collections::HashMap, sync::Arc};

use axum::{
    Extension, Json,
    body::Bytes,
    extract::{Multipart, Path, State},
    response::IntoResponse,
};

use crate::{
    application::{
        commands::{
            media::{ChangePostCoverCommand, UploadMediaWithoutDescriptionCommand},
            post::{SetRelatedPostsCommand, UpdatePostCommand, UpdatePostCoverCommand},
        },
        services::{media::MediaService, post::PostService},
    },
    domain::{
        entities::{media::MediumDetails, secret::Claims},
        errors::{media::MediaError, post::PostError},
    },
    infrastructure::web::{
        api::handlers::post::dto::{PostPatchData, SetRelatedPostsBody},
        api::handlers::post::response::UpdatePostResponse,
        api::handlers::support::cover::{MediumData, extract_medium},
        api::support::{media_short_names::replace_media_short_names, multipart::FileData},
        server::AppState,
    },
};

pub async fn set_related_posts(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(post_id): Path<i64>,
    Json(body): Json<SetRelatedPostsBody>,
) -> Result<impl IntoResponse, PostError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|e| PostError::InternalError(e.to_string()))?;

    state
        .post_service
        .set_related_posts(SetRelatedPostsCommand {
            user_id,
            post_id,
            related_post_slugs: body.related_post_slugs,
        })
        .await?;

    Ok(())
}

pub async fn update_post(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(post_id): Path<String>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, PostError> {
    let uploader_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| PostError::InternalError("Cannot parse id.".to_string()))?;

    let post_id = post_id
        .parse::<i64>()
        .map_err(|_| PostError::InternalError("Cannot parse post_id.".to_string()))?;

    let mut post_data: Option<PostPatchData> = None;
    let mut file_map = HashMap::<usize, FileData>::new();
    let mut short_name_map = HashMap::<usize, String>::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| PostError::InternalError(e.to_string()))?
    {
        let field_name = field
            .name()
            .ok_or(PostError::UploadFailed("Empty field found.".to_string()))?;

        if field_name == "post_data" {
            let bytes = field
                .bytes()
                .await
                .map_err(|_| PostError::UploadFailed("Cannot read post data.".to_string()))?;
            post_data = Some(
                serde_json::from_slice::<PostPatchData>(&bytes)
                    .map_err(|e| PostError::UploadFailed(format!("Malformed post data: {}", e)))?,
            );
        } else if let Some(index_str) = field_name.strip_prefix("file_") {
            let index: usize = index_str
                .parse()
                .map_err(|_| PostError::UploadFailed("Invalid file index".to_string()))?;

            if file_map.contains_key(&index) {
                return Err(PostError::UploadFailed(format!(
                    "Duplicate file index {index}"
                )));
            }

            let file_name = field
                .file_name()
                .ok_or(PostError::UploadFailed(
                    "Cannot read file name.".to_string(),
                ))?
                .to_string();

            let content_type = field
                .content_type()
                .ok_or(PostError::UploadFailed(format!(
                    "Cannot read content type of {}.",
                    file_name
                )))?
                .to_string();

            let bytes = field.bytes().await.map_err(|_| {
                PostError::UploadFailed(format!("Cannot read bytes of {}.", file_name))
            })?;

            file_map.insert(
                index,
                FileData {
                    file_name,
                    bytes,
                    content_type,
                },
            );
        } else if let Some(index_str) = field_name.strip_prefix("short_name_") {
            let index: usize = index_str
                .parse()
                .map_err(|_| PostError::UploadFailed("Cannot extract file index.".to_string()))?;

            if short_name_map.contains_key(&index) {
                return Err(PostError::UploadFailed(format!(
                    "Duplicated short name indices found ({}).",
                    index
                )));
            }

            short_name_map.insert(
                index,
                field.text().await.map_err(|_| {
                    PostError::UploadFailed(format!("Cannot read short name of index {}.", index))
                })?,
            );
        }
    }

    let post_data = post_data.ok_or(PostError::UploadFailed(
        "No post data is given.".to_string(),
    ))?;

    let mut short_names = Vec::<String>::new();
    let mut file_names = Vec::<String>::new();
    let mut content_types = Vec::<String>::new();
    let mut bytes_list = Vec::<Bytes>::new();

    for i in 1..=post_data.number_of_files {
        let file = file_map
            .get(&i)
            .ok_or_else(|| PostError::UploadFailed(format!("Cannot locate file_{}", i)))?;

        file_names.push(file.file_name.clone());
        content_types.push(file.content_type.clone());
        bytes_list.push(file.bytes.clone());

        let short_name = short_name_map
            .get(&i)
            .ok_or_else(|| PostError::UploadFailed(format!("Cannot locate short_name_{}", i)))?;

        short_names.push(short_name.clone());
    }

    let cmd = UploadMediaWithoutDescriptionCommand {
        uploader_id,
        short_names,
        number_of_files: post_data.number_of_files,
        file_names,
        content_types,
        bytes_list,
    };

    if let Err(media_err) = state
        .media_service
        .bulk_upload(cmd, &state.media_config)
        .await
    {
        return Err(PostError::Media(media_err));
    }

    let mut cmd = UpdatePostCommand {
        user_id: uploader_id,
        // Admins may edit anyone's post; everyone else is limited to their own.
        required_author_id: if claims.role == "admin" {
            None
        } else {
            Some(uploader_id)
        },
        expected_updated_at: post_data.expected_updated_at,
        post_id,
        title: post_data.title,
        slug: post_data.slug,
        excerpt: post_data.excerpt,
        content: post_data.content.clone(),
        tags: post_data.tags,
        media_usage: None,
    };

    if let Some(content) = post_data.content {
        let mut content = content;
        let mut media_usage = HashMap::<String, i64>::new();

        replace_media_short_names(&mut content, &mut media_usage);

        cmd.content = Some(content);
        cmd.media_usage = Some(media_usage);
    }

    let updated_at = state.post_service.update_post(cmd).await?;

    if post_data.og_image_seconds.is_some() {
        state
            .post_service
            .update_post_cover(UpdatePostCoverCommand {
                user_id: uploader_id,
                post_id,
                og_image_seconds: post_data.og_image_seconds,
            })
            .await?;
    }

    Ok(Json(UpdatePostResponse { updated_at }))
}

#[axum::debug_handler]
pub async fn change_cover(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(post_id): Path<i64>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, PostError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| PostError::InternalError("Cannot parse id.".to_string()))?;

    let mut opt_filename: Option<String> = None;
    let mut opt_content_type: Option<String> = None;
    let mut opt_bytes: Option<Bytes> = None;
    let mut opt_og_image_seconds: Option<i64> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| PostError::InternalError(e.to_string()))?
    {
        let field_name = field.name().ok_or(MediaError::UploadFailed(
            "Empty field detected.".to_string(),
        ))?;
        match field_name {
            "file" => {
                if opt_filename.is_some() {
                    return Err(PostError::Media(MediaError::UploadFailed(
                        "Only one media is allowed at a time.".to_string(),
                    )));
                }
                let MediumData {
                    filename,
                    content_type,
                    bytes,
                } = extract_medium(field).await?;
                opt_filename = Some(filename);
                opt_content_type = Some(content_type);
                opt_bytes = Some(bytes);
            }
            "og_image_seconds" => {
                let text = field.text().await.map_err(|e| {
                    PostError::InternalError(format!("Failed to read og_image_seconds: {}", e))
                })?;
                opt_og_image_seconds = text.trim().parse::<i64>().ok();
            }
            _ => {}
        }
    }

    let filename =
        opt_filename.ok_or_else(|| MediaError::UploadFailed("Missing file".to_string()))?;
    let content_type = opt_content_type
        .ok_or_else(|| MediaError::UploadFailed("Missing content type".to_string()))?;
    let bytes =
        opt_bytes.ok_or_else(|| MediaError::UploadFailed("Missing file bytes".to_string()))?;

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
