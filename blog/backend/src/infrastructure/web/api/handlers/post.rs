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
            media::{ChangePostCoverCommand, UploadMediaWithoutDescriptionCommand},
            post::{
                CheckSlugCommand, GetCategoriesCommand, GetCommentsCommand,
                GetDetailedPostsCommand, GetFeaturedPostsCommand, GetLatestPostsCommand,
                GetPostCommand, NewPostCommand, PostNewAnynymouseCommentCommand,
                PostNewCommentCommand, PublishCommand, SearchPostCommand, UpdatePostCommand,
            },
        },
        services::{media::MediaService, post::PostService},
    },
    domain::{
        entities::{
            media::MediumDetails,
            post::{PostDetails, PostSummary},
            secret::Claims,
        },
        errors::{media::MediaError, post::PostError},
    },
    helper::string::replace_range_unicode,
    infrastructure::web::{
        api::handlers::common::{MediumData, extract_medium},
        server::AppState,
    },
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
    Path(post_id_str): Path<String>,
) -> Result<(), PostError> {
    let post_id = post_id_str
        .parse::<i64>()
        .map_err(|_| PostError::PostNotFound)?;

    let cmd = PublishCommand {
        user_id: claims
            .user_id
            .parse::<i64>()
            .map_err(|e| PostError::InternalError(e.to_string()))?,
        post_id,
    };

    state.post_service.publish(cmd).await?;

    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct GetPostDetailsResponse {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub tags: Vec<String>,
    pub excerpt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_cover_url: Option<String>,
    pub content: String,
    pub draft: String,
    pub is_featured: i64,
    pub medium_urls: Vec<String>,
    pub medium_short_names: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
}

pub async fn get_post_details(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(post_id): Path<String>,
) -> Result<impl IntoResponse, PostError> {
    let required_author_id = Some(
        claims
            .user_id
            .parse::<i64>()
            .map_err(|e| PostError::InternalError(e.to_string()))?,
    );
    let post_id = post_id
        .parse::<i64>()
        .map_err(|e| PostError::InternalError(e.to_string()))?;
    let PostDetails {
        id,
        title,
        slug,
        tags,
        excerpt,
        series_slug,
        series_cover_url,
        content,
        draft,
        is_featured,
        cover_url,
        medium_urls,
        medium_short_names,
    } = state
        .post_service
        .get_post_details(GetDetailedPostsCommand {
            required_author_id,
            post_id,
        })
        .await?;

    Ok(Json(GetPostDetailsResponse {
        id,
        title,
        slug,
        tags,
        excerpt,
        series_slug,
        series_cover_url,
        content,
        draft,
        is_featured,
        cover_url,
        medium_urls,
        medium_short_names,
    }))
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
            if let Ok(bytes) = field.bytes().await {
                post_data = Some(serde_json::from_slice::<PostPatchData>(&bytes).unwrap());
            }
        } else if let Some(index_str) = field_name.strip_prefix("file_") {
            let index: usize = index_str
                .parse()
                .map_err(|_| PostError::UploadFailed(format!("Invalid file index")))?;

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

    if post_data
        .content
        .as_ref()
        .xor(post_data.draft.as_ref())
        .is_some()
    {
        return Err(PostError::UploadFailed(
            "Content and Draft must both present or both absent.".to_string(),
        ));
    }

    let mut cmd = UpdatePostCommand {
        user_id: uploader_id,
        post_id,
        title: post_data.title,
        slug: post_data.slug,
        excerpt: post_data.excerpt,
        content: post_data.content.clone(),
        draft: post_data.draft.clone(),
        tags: post_data.tags,
        media_usage: None,
    };

    if let Some(content) = post_data.content
        && let Some(draft) = post_data.draft
    {
        let mut content = content;
        let mut draft = draft;

        let reg = Regex::new(r"@(?:\([\d_]+\))?\[[\w-]+:([^\]]+)\]").unwrap();

        let mut content_extraction = Vec::<ShortNameExtraction>::new();

        for cap in reg.captures_iter(&content) {
            if let Some(matched) = cap.get(1) {
                content_extraction.push(ShortNameExtraction {
                    short_name: matched.as_str().to_string(),
                    start: matched.start(),
                });
            }
        }
        content_extraction.sort_by_key(|k| Reverse(k.start));

        let mut draft_extraction = Vec::<ShortNameExtraction>::new();

        for cap in reg.captures_iter(&draft) {
            if let Some(matched) = cap.get(1) {
                let short_name = matched.as_str().to_string();
                let start = matched.start();
                draft_extraction.push(ShortNameExtraction { short_name, start });
            }
        }
        draft_extraction.sort_by_key(|k| Reverse(k.start));

        let mut media_usage = HashMap::<String, i64>::new();

        for data in content_extraction {
            let len = media_usage.len();

            let index = media_usage
                .entry(data.short_name.clone())
                .or_insert_with(|| len as i64)
                .to_string();

            let len = data.short_name.len();
            replace_range_unicode(&mut content, data.start, len, index.to_string());
        }

        for data in draft_extraction {
            let len = media_usage.len();

            let index = media_usage
                .entry(data.short_name.clone())
                .or_insert_with(|| len as i64)
                .to_string();

            let len = data.short_name.len();
            replace_range_unicode(&mut draft, data.start, len, index.to_string());
        }

        cmd.content = Some(content);
        cmd.draft = Some(draft);
        cmd.media_usage = Some(media_usage);
    }

    state.post_service.update_post(cmd).await?;

    Ok(())
}

#[derive(Deserialize)]
pub struct GetPostQuery {
    pub with_draft: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct PostResponse {
    pub id: i64,
    pub title: String,
    pub tags: Vec<String>,
    pub author_name: String,
    pub author_slug: String,
    pub excerpt: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft: Option<String>,
    pub medium_urls: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_avatar_url: Option<String>,
}

pub async fn get_post_by_slug(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    Path(post_slug): Path<String>,
    Query(query): Query<GetPostQuery>,
) -> Result<impl IntoResponse, PostError> {
    let mut cmd = GetPostCommand {
        slug: post_slug,
        as_id: None,
    };
    if let Some(claims) = opt_claims
        && let Some(with_draft) = query.with_draft
    {
        if with_draft {
            let id = claims.user_id.parse::<i64>().unwrap();
            cmd.as_id = Some(id);
        }
    }

    let post = state.post_service.get_post(cmd).await?;
    Ok(Json(PostResponse {
        id: post.id,
        title: post.title,
        author_name: post.author_name,
        author_slug: post.author_slug,
        author_avatar_url: post.author_avatar_url,
        tags: post.tags,
        excerpt: post.excerpt,
        content: post.content,
        draft: query
            .with_draft
            .and_then(|with_draft| if with_draft { Some(post.draft) } else { None }),
        medium_urls: post.medium_urls,
        published_at: post.published_at,
        cover_url: post.cover_url,
    }))
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
            .map(|category_result| CategoryResponse {
                name: category_result.name,
                slug: category_result.slug,
            })
            .collect(),
    }))
}

#[derive(Debug)]
pub struct ShortNameExtraction {
    pub short_name: String,
    pub start: usize,
}

#[derive(Deserialize)]
pub struct PostData {
    title: String,
    slug: String,
    excerpt: String,
    content: String,
    tags: Vec<String>,
    number_of_files: usize,
}

#[derive(Deserialize)]
pub struct PostPatchData {
    title: Option<String>,
    slug: Option<String>,
    excerpt: Option<String>,
    content: Option<String>,
    draft: Option<String>,
    tags: Option<Vec<String>>,
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
            .ok_or(PostError::UploadFailed("Empty field found.".to_string()))?;

        if field_name == "post_data" {
            if let Ok(bytes) = field.bytes().await {
                post_data = Some(serde_json::from_slice::<PostData>(&bytes).unwrap());
            }
        } else if let Some(index_str) = field_name.strip_prefix("file_") {
            let index: usize = index_str
                .parse()
                .map_err(|_| PostError::UploadFailed(format!("Invalid file index")))?;

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

    let cmd = NewPostCommand {
        user_id: uploader_id,
        title: post_data.title,
        slug: post_data.slug,
        excerpt: post_data.excerpt,
        tags: post_data.tags,
        content,
        cover_image: None,
        media_usage,
    };

    state.post_service.new_post(cmd).await?;

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchPostQuery {
    pub term: String,
    pub size: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct SearchPostResult {
    pub title: String,
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
}

#[derive(Serialize)]
pub struct SearchPostResponse {
    pub posts: Vec<SearchPostResult>,
}

#[axum::debug_handler]
pub async fn search(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchPostQuery>,
) -> Result<impl IntoResponse, PostError> {
    let term = query.term;
    let size = query.size.unwrap_or(1);
    let offset = query.offset.unwrap_or(0);

    let user_snapshots = state
        .post_service
        .search(SearchPostCommand { term, size, offset })
        .await?;

    let posts: Vec<SearchPostResult> = user_snapshots
        .into_iter()
        .map(
            |PostSummary {
                 title,
                 slug,
                 cover_url,
             }| SearchPostResult {
                title,
                slug,
                cover_url,
            },
        )
        .collect();

    Ok(Json(SearchPostResponse { posts }))
}

#[derive(Deserialize)]
pub struct GetFeaturedPostsBody {
    pub limit: i64,
}

#[derive(Serialize)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub tag_names: Vec<String>,
    pub tag_slugs: Vec<String>,
    pub excerpt: String,
    pub author_name: String,
    pub author_slug: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Serialize)]
pub struct GetFeaturedPostsResponse {
    pub featured_posts: Vec<Post>,
}

#[axum::debug_handler]
pub async fn get_featured_posts(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetFeaturedPostsBody>,
) -> Result<impl IntoResponse, PostError> {
    let cmd = GetFeaturedPostsCommand { limit: query.limit };
    let featured_posts = state.post_service.get_featured_post_snapshots(cmd).await?;

    let wrapped_featured_posts = GetFeaturedPostsResponse {
        featured_posts: featured_posts
            .into_iter()
            .map(|post| Post {
                id: post.id,
                title: post.title,
                slug: post.slug,
                tag_names: post.tag_names,
                tag_slugs: post.tag_slugs,
                excerpt: post.excerpt,
                author_name: post.author_name,
                author_slug: post.author_slug,
                url: post.url,
            })
            .collect(),
    };

    Ok(Json(wrapped_featured_posts))
}

#[derive(Deserialize)]
pub struct GetFeaturedPostsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[axum::debug_handler]
pub async fn get_latest_posts(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetFeaturedPostsQuery>,
) -> Result<impl IntoResponse, PostError> {
    let limit = query.limit.unwrap_or(1);
    let offset = query.offset.unwrap_or(0);
    let cmd = GetLatestPostsCommand { limit, offset };
    let featured_posts = state.post_service.get_latest_post_snapshots(cmd).await?;

    let wrapped_featured_posts = GetFeaturedPostsResponse {
        featured_posts: featured_posts
            .into_iter()
            .map(|post| Post {
                id: post.id,
                title: post.title,
                slug: post.slug,
                tag_names: post.tag_names,
                tag_slugs: post.tag_slugs,
                excerpt: post.excerpt,
                author_name: post.author_name,
                author_slug: post.author_slug,
                url: post.url,
            })
            .collect(),
    };

    Ok(Json(wrapped_featured_posts))
}

#[derive(Deserialize)]
pub struct NewCommentBody {
    pub content: String,
}

#[derive(Serialize)]
pub struct NewCommentResponse {
    pub comment_id: i64,
}

pub async fn new_comment(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    Path(post_id_str): Path<String>,
    Json(body): Json<NewCommentBody>,
) -> Result<impl IntoResponse, PostError> {
    let post_id: i64 = post_id_str.parse().map_err(|_| PostError::PostNotFound)?;
    let comment_id = match opt_claims {
        Some(claims) => {
            let user_id = claims
                .user_id
                .parse::<i64>()
                .map_err(|e| PostError::InternalError(e.to_string()))?;

            state
                .post_service
                .post_new_comment(PostNewCommentCommand {
                    post_id,
                    user_id,
                    content: body.content,
                })
                .await?
        }
        None => {
            state
                .post_service
                .post_new_anonymous_comment(PostNewAnynymouseCommentCommand {
                    post_id,
                    content: body.content,
                })
                .await?
        }
    };

    Ok(Json(NewCommentResponse { comment_id }))
}

#[derive(Deserialize)]
pub struct CommentsQuery {
    pub before: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Deserialize, Serialize)]
pub struct Comment {
    pub id: i64,
    pub content: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_role: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct CommentsResponse {
    comments: Vec<Comment>,
}

pub async fn get_comments(
    State(state): State<Arc<AppState>>,
    Path(post_id_str): Path<String>,
    Query(query): Query<CommentsQuery>,
) -> Result<impl IntoResponse, PostError> {
    let before = query.before;
    let limit = query.limit.unwrap_or(1);
    let post_id = post_id_str.parse::<i64>().unwrap();

    let comments = state
        .post_service
        .get_comments(GetCommentsCommand {
            post_id,
            limit,
            before,
        })
        .await?;

    let comments: Vec<Comment> = comments
        .into_iter()
        .map(|comment| Comment {
            id: comment.id,
            content: comment.content,
            created_at: comment.created_at,
            username: comment.username,
            display_name: comment.display_name,
            avatar_url: comment.avatar_url,
            user_role: comment.user_role,
        })
        .collect();

    let wrapped_comments = CommentsResponse { comments };

    Ok(Json(wrapped_comments))
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

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| PostError::InternalError(e.to_string()))?
    {
        let field_name = field.name().ok_or(MediaError::UploadFailed(
            "Empty field detected.".to_string(),
        ))?;

        if field_name == "file" {
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
            },
            &state.media_config,
        )
        .await?;

    Ok(())
}
