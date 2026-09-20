// Creating a post: inline multipart intake with per-file short names,
// cover fields, and media usage extraction. Updates live in `update`.
use std::{collections::HashMap, sync::Arc};

use axum::{
    Extension, Json,
    extract::{Multipart, State},
    response::IntoResponse,
};

use crate::{
    application::{
        commands::{
            media::UploadMediaWithoutDescriptionCommand,
            post::NewPostCommand,
        },
        services::{media::MediaService, post::PostService},
    },
    domain::{
        entities::secret::Claims,
        errors::post::PostError,
    },
    infrastructure::web::{
        api::handlers::support::cover::{
            CreateCoverUpload, apply_created_cover_upload, try_collect_create_cover_field,
        },
        api::handlers::post::dto::PostData,
        api::support::{
            media_short_names::replace_media_short_names,
            multipart::FileData,
        },
        server::AppState,
    },
};

#[axum::debug_handler]
pub async fn new_post(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, PostError> {
    let uploader_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| PostError::InternalError("Cannot parse id".to_string()))?;

    let mut post_data: Option<PostData> = None;
    let mut file_map = HashMap::<usize, FileData>::new();
    let mut short_name_map = HashMap::<usize, String>::new();
    let mut create_cover = CreateCoverUpload::default();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| PostError::InternalError(e.to_string()))?
    {
        let field_name = field
            .name()
            .ok_or(PostError::UploadFailed("Empty field found.".to_string()))?
            .to_string();

        if field_name == "post_data" {
            let bytes = field
                .bytes()
                .await
                .map_err(|_| PostError::UploadFailed("Cannot read post data.".to_string()))?;
            post_data = Some(
                serde_json::from_slice::<PostData>(&bytes)
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
        } else if try_collect_create_cover_field(&field_name, field, &mut create_cover).await? {
        } else if field_name == "excerpt" {
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

    let mut content = post_data.content;

    let mut media_usage = HashMap::<String, i64>::new();
    replace_media_short_names(&mut content, &mut media_usage);

    let cmd = NewPostCommand {
        user_id: uploader_id,
        title: post_data.title,
        slug: post_data.slug,
        excerpt: post_data.excerpt,
        tags: post_data.tags,
        content,
        cover_media: None,
        media_usage,
        content_kind: "post".to_string(),
    };

    let post_id = state.post_service.new_post(cmd).await?;

    apply_created_cover_upload(&state, uploader_id, post_id, create_cover).await?;

    Ok(Json(serde_json::json!({ "id": post_id })))
}

