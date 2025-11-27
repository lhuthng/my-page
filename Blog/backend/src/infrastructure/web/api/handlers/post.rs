use std::{cmp::Reverse, collections::HashMap, sync::Arc};

use axum::{
    Extension, Json,
    body::Bytes,
    extract::{Multipart, Path, Query, State},
    response::IntoResponse,
};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    application::{
        commands::{
            media::UploadMediaWithoutDescriptionCommand,
            post::{CheckSlugCommand, GetCategoriesCommand, PostCommand, PublishCommand},
        },
        services::{media::MediaService, post::PostService},
    },
    domain::{entities::secret::Claims, errors::post::PostError},
    helper::string::replace_range_unicode,
    infrastructure::web::server::AppState,
};

#[derive(Deserialize)]
pub struct CheckQuery {
    pub slug: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CheckResponse {
    exists: bool,
}

#[axum::debug_handler]
pub async fn check_post(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CheckQuery>,
) -> Result<impl IntoResponse, PostError> {
    if let Some(post_slug) = query.slug {
        let cmd = CheckSlugCommand { post_slug };
        let exists = state.post_service.check_slug(cmd).await?;
        Ok(Json(CheckResponse { exists }))
    } else {
        Ok(Json(CheckResponse { exists: true }))
    }
}

pub async fn publish(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(slug): Path<String>,
) -> Result<(), PostError> {
    let cmd = PublishCommand {
        user_id: claims
            .user_id
            .parse::<i64>()
            .map_err(|e| PostError::InternalError(e.to_string()))?,
        slug,
    };

    state.post_service.publish(cmd).await?;

    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct CategoryResponse {
    name: String,
    slug: String,
}

#[derive(Serialize, Deserialize)]
pub struct GetCategoriesResponse {
    categories: Vec<CategoryResponse>,
}

pub async fn get_categories(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, PostError> {
    let cmd = GetCategoriesCommand {};
    let categories = state.post_service.get_categories(cmd).await?;
    Ok(Json(GetCategoriesResponse {
        categories: categories
            .into_iter()
            .map(|(name, slug)| CategoryResponse { name, slug })
            .collect(),
    }))
}

pub struct ShortNameExtraction {
    pub short_name: String,
    pub start: usize,
}

#[derive(Deserialize)]
pub struct PostData {
    title: String,
    slug: String,
    content: String,
    categories: Vec<String>,
    number_of_files: usize,
}

pub struct FileData {
    pub file_name: String,
    pub bytes: Bytes,
    pub content_type: String,
}

#[axum::debug_handler]
pub async fn new_post(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    mut multipart: Multipart,
) -> Result<(), PostError> {
    let uploader_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| PostError::InternalError("Cannot parse id".to_string()))?;

    let mut post_data: Option<PostData> = None;
    let mut file_map = HashMap::<usize, FileData>::new();
    let mut short_name_map = HashMap::<usize, String>::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| PostError::InternalError(e.to_string()))?
    {
        let field_name = field
            .name()
            .ok_or(PostError::UploadFailed("Empty field detected.".to_string()))?;

        if field_name == "post_data" {
            if let Ok(bytes) = field.bytes().await {
                post_data = Some(serde_json::from_slice::<PostData>(&bytes).unwrap());
            }
        } else if let Some(index_str) = field_name.strip_prefix("file_") {
            let index: usize = index_str
                .parse()
                .map_err(|e| PostError::UploadFailed(format!("Invalid file index: {e}")))?;

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

    if let Err(_) = state
        .media_service
        .bulk_upload(cmd, &state.media_config)
        .await
    {
        // TODO: map MediaError-PostError
        return Err(PostError::UploadFailed("Failed to add files".to_string()));
    }

    let mut content = post_data.content;

    // Replace short names with indices
    let mut extraction = Vec::<ShortNameExtraction>::new();

    let reg = Regex::new(r"@(?:\([\d_]+\))?\[[\w-]+:([^\]]+)\]").unwrap();
    for cap in reg.captures_iter(&content) {
        if let Some(matched) = cap.get(1) {
            extraction.push(ShortNameExtraction {
                short_name: matched.as_str().to_string(),
                start: matched.start(),
            });
        }
    }

    extraction.sort_by_key(|k| Reverse(k.start));

    let mut media_usage = HashMap::<String, i64>::new();

    for data in extraction {
        let len = media_usage.len();

        let index = media_usage
            .entry(data.short_name.clone())
            .or_insert_with(|| len as i64)
            .to_string();

        let len = data.short_name.len();
        replace_range_unicode(&mut content, data.start, len, index.to_string());
    }

    let cmd = PostCommand {
        user_id: uploader_id,
        title: post_data.title,
        slug: post_data.slug,
        categories: post_data.categories,
        content,
        cover_image: None,
        media_usage,
    };

    state.post_service.post(cmd).await?;

    Ok(())
}
