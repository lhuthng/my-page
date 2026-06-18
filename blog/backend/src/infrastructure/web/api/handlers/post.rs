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
                GetPostCommand, GetPostsByTagCommand, GetRelatedPostsCommand, NewPostCommand,
                PostNewAnynymouseCommentCommand, PostNewCommentCommand, PublishCommand,
                PushNewLikeCommand, PushNewViewCommand, SearchPostCommand, SearchTagsCommand,
                SetFeaturedPostCommand, SetRelatedPostsCommand, UpdatePostCommand,
                UpdatePostCoverCommand,
            },
            project::GetProjectsByTagCommand,
        },
        services::{media::MediaService, post::PostService, project::ProjectService},
    },
    domain::{
        entities::{
            media::MediumDetails,
            post::{PostDetails, PostSnapshot, PostSummary, TagSummary},
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_media_type: Option<String>,
    pub og_image_seconds: i64,
    pub is_owner: bool,
}

pub async fn get_post_details(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(post_id): Path<String>,
) -> Result<impl IntoResponse, PostError> {
    let uploader_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|e| PostError::InternalError(e.to_string()))?;
    let is_admin = claims.role == "admin";
    let post_id = post_id
        .parse::<i64>()
        .map_err(|e| PostError::InternalError(e.to_string()))?;

    // Admins can view any post; others can only view their own
    let required_author_id = if is_admin { None } else { Some(uploader_id) };

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
        cover_media_type,
        og_image_seconds,
        medium_urls,
        medium_short_names,
        is_owner,
    } = state
        .post_service
        .get_post_details(GetDetailedPostsCommand {
            required_author_id,
            viewing_user_id: uploader_id,
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
        cover_media_type,
        og_image_seconds,
        medium_urls,
        medium_short_names,
        is_owner,
    }))
}

#[derive(Serialize)]
pub struct GetRelatedPostsResponse {
    pub posts: Vec<SearchPostResult>,
}

pub async fn get_related_posts(
    State(state): State<Arc<AppState>>,
    Path(post_id): Path<i64>,
) -> Result<impl IntoResponse, PostError> {
    let related = state
        .post_service
        .get_related_posts(GetRelatedPostsCommand { post_id })
        .await?;

    let posts: Vec<SearchPostResult> = related
        .into_iter()
        .map(|s| SearchPostResult {
            title: s.title,
            slug: s.slug,
            cover_url: s.cover_url,
        })
        .collect();

    Ok(Json(GetRelatedPostsResponse { posts }))
}

#[derive(Deserialize)]
pub struct SetRelatedPostsBody {
    pub related_post_slugs: Vec<String>,
}

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
            if let Ok(bytes) = field.bytes().await {
                post_data = Some(serde_json::from_slice::<PostPatchData>(&bytes).unwrap());
            }
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
        let mut media_usage = HashMap::<String, i64>::new();

        replace_media_short_names(&mut content, &mut media_usage);
        replace_media_short_names(&mut draft, &mut media_usage);

        cmd.content = Some(content);
        cmd.draft = Some(draft);
        cmd.media_usage = Some(media_usage);
    }

    state.post_service.update_post(cmd).await?;

    if post_data.video_short_name.is_some() || post_data.og_image_seconds.is_some() {
        state
            .post_service
            .update_post_cover(UpdatePostCoverCommand {
                user_id: uploader_id,
                post_id,
                video_short_name: post_data.video_short_name,
                og_image_seconds: post_data.og_image_seconds,
            })
            .await?;
    }

    Ok(())
}

#[derive(Deserialize)]
pub struct GetPostQuery {
    pub with_draft: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct PostSeriesResponse {
    pub title: String,
    pub slug: String,
    pub cover_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_post: Option<SearchPostResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_post: Option<SearchPostResult>,
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
    pub updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_media_type: Option<String>,
    pub og_image_seconds: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<PostSeriesResponse>,
    pub related_posts: Vec<SearchPostResult>,
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
        && with_draft
    {
        let id = claims.user_id.parse::<i64>().unwrap();
        cmd.as_id = Some(id);
    }

    let post = state.post_service.get_post(cmd).await?;
    let related = state
        .post_service
        .get_related_posts(GetRelatedPostsCommand { post_id: post.id })
        .await
        .unwrap_or_default();

    let related_posts: Vec<SearchPostResult> = related
        .into_iter()
        .map(|s| SearchPostResult {
            title: s.title,
            slug: s.slug,
            cover_url: s.cover_url,
        })
        .collect();

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
            .and_then(|with_draft| with_draft.then_some(post.draft)),
        medium_urls: post.medium_urls,
        published_at: post.published_at,
        updated_at: post.updated_at,
        cover_media_type: post.cover_media_type,
        og_image_seconds: post.og_image_seconds,
        series: post.post_series.map(|series| PostSeriesResponse {
            title: series.series_title,
            slug: series.series_slug,
            cover_url: series.series_cover_url,
            previous_post: series.previous_post.map(|post| SearchPostResult {
                title: post.title,
                slug: post.slug,
                cover_url: post.cover_url,
            }),
            next_post: series.next_post.map(|post| SearchPostResult {
                title: post.title,
                slug: post.slug,
                cover_url: post.cover_url,
            }),
        }),
        cover_url: post.cover_url,
        related_posts,
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

fn extract_media_short_names(content: &str) -> Vec<ShortNameExtraction> {
    let syntaxes = [
        Regex::new(r"@(?:\([\d_]+\))?\[[\w-]+:([^\]]+)\]").unwrap(),
        Regex::new(r":::app\s+lottie\s+([^\s]+)").unwrap(),
    ];

    let mut extraction = Vec::<ShortNameExtraction>::new();
    for reg in syntaxes {
        for cap in reg.captures_iter(content) {
            if let Some(matched) = cap.get(1) {
                extraction.push(ShortNameExtraction {
                    short_name: matched.as_str().to_string(),
                    start: matched.start(),
                });
            }
        }
    }
    extraction.sort_by_key(|k| Reverse(k.start));
    extraction
}

fn replace_media_short_names(content: &mut String, usage: &mut HashMap<String, i64>) {
    for data in extract_media_short_names(content) {
        let len = usage.len();
        let index = usage
            .entry(data.short_name.clone())
            .or_insert_with(|| len as i64)
            .to_string();
        replace_range_unicode(content, data.start, data.short_name.len(), index);
    }
}

#[derive(Deserialize)]
pub struct PostData {
    title: String,
    slug: String,
    excerpt: String,
    content: String,
    tags: Vec<String>,
    number_of_files: usize,
    video_short_name: Option<String>,
    og_image_seconds: Option<i64>,
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
    video_short_name: Option<String>,
    og_image_seconds: Option<i64>,
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
) -> Result<impl IntoResponse, PostError> {
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

    if post_data.video_short_name.is_some() || post_data.og_image_seconds.is_some() {
        state
            .post_service
            .update_post_cover(UpdatePostCoverCommand {
                user_id: uploader_id,
                post_id,
                video_short_name: post_data.video_short_name,
                og_image_seconds: post_data.og_image_seconds,
            })
            .await?;
    }

    Ok(Json(serde_json::json!({ "id": post_id })))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchPostQuery {
    pub term: String,
    pub size: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize, Deserialize)]
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

#[derive(Deserialize)]
pub struct SearchTagsQuery {
    pub term: Option<String>,
    pub size: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct SearchTagResult {
    pub name: String,
    pub slug: String,
    pub post_count: i64,
}

#[derive(Serialize)]
pub struct SearchTagsResponse {
    pub tags: Vec<SearchTagResult>,
}

#[derive(Serialize)]
pub struct TagPostsResponse {
    pub tag: SearchTagResult,
    pub posts: Vec<Post>,
    pub projects: Vec<super::project::ProjectCard>,
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

#[axum::debug_handler]
pub async fn search_tags(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchTagsQuery>,
) -> Result<impl IntoResponse, PostError> {
    let tags = state
        .post_service
        .search_tags(SearchTagsCommand {
            term: query.term,
            size: query.size.unwrap_or(24),
            offset: query.offset.unwrap_or(0),
        })
        .await?;

    let tags = tags
        .into_iter()
        .map(
            |TagSummary {
                 name,
                 slug,
                 post_count,
             }| SearchTagResult {
                name,
                slug,
                post_count,
            },
        )
        .collect();

    Ok(Json(SearchTagsResponse { tags }))
}

#[derive(Deserialize)]
pub struct GetTagPostsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[axum::debug_handler]
pub async fn get_posts_by_tag(
    State(state): State<Arc<AppState>>,
    Path(tag_slug): Path<String>,
    Query(query): Query<GetTagPostsQuery>,
) -> Result<impl IntoResponse, PostError> {
    let (tag, posts) = state
        .post_service
        .get_posts_by_tag(GetPostsByTagCommand {
            slug: tag_slug,
            limit: query.limit.unwrap_or(24),
            offset: query.offset.unwrap_or(0),
        })
        .await?;
    let projects = state
        .project_service
        .get_project_snapshots_by_tag(GetProjectsByTagCommand {
            slug: tag.slug.clone(),
            limit: query.limit.unwrap_or(24),
            offset: query.offset.unwrap_or(0),
        })
        .await
        .unwrap_or_default();

    Ok(Json(TagPostsResponse {
        tag: SearchTagResult {
            name: tag.name,
            slug: tag.slug,
            post_count: tag.post_count,
        },
        posts: posts.into_iter().map(Into::into).collect(),
        projects: projects.into_iter().map(Into::into).collect(),
    }))
}

#[derive(Deserialize)]
pub struct GetFeaturedPostsBody {
    pub limit: i64,
}

#[derive(Serialize)]
pub struct PostStats {
    pub views: i64,
    pub likes: i64,
    pub comments: i64,
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
    pub cover_media_type: Option<String>,
    pub stats: PostStats,
}

impl From<PostSnapshot> for Post {
    fn from(value: PostSnapshot) -> Self {
        let PostSnapshot {
            id,
            title,
            slug,
            tag_names,
            tag_slugs,
            excerpt,
            author_name,
            author_slug,
            url,
            cover_media_type,
            stats,
            ..
        } = value;
        Post {
            id,
            title,
            slug,
            tag_names,
            tag_slugs,
            excerpt,
            author_name,
            author_slug,
            url,
            cover_media_type,
            stats: PostStats {
                views: stats.views,
                likes: stats.likes,
                comments: stats.comments,
            },
        }
    }
}
#[derive(Serialize)]
pub struct GetFeaturedPostsResponse {
    pub featured_posts: Vec<Post>,
    pub has_more: bool,
}

#[axum::debug_handler]
pub async fn get_featured_posts(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetFeaturedPostsBody>,
) -> Result<impl IntoResponse, PostError> {
    let cmd = GetFeaturedPostsCommand { limit: query.limit };
    let featured_posts = state.post_service.get_featured_post_snapshots(cmd).await?;

    let wrapped_featured_posts = GetFeaturedPostsResponse {
        featured_posts: featured_posts.into_iter().map(|post| post.into()).collect(),
        has_more: false,
    };

    Ok(Json(wrapped_featured_posts))
}

#[derive(Deserialize)]
pub struct GetFeaturedPostsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sorted_by_updated: Option<bool>,
    pub sorted_by_created: Option<bool>,
}

#[axum::debug_handler]
pub async fn get_latest_posts(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetFeaturedPostsQuery>,
) -> Result<impl IntoResponse, PostError> {
    let limit = query.limit.unwrap_or(1);
    let offset = query.offset.unwrap_or(0);
    let default = "created";
    let sorted_by = query
        .sorted_by_updated
        .and_then(|v| v.then_some("updated"))
        .or(query.sorted_by_created.and_then(|v| v.then_some("created")))
        .unwrap_or(default)
        .to_string();
    let cmd = GetLatestPostsCommand {
        limit,
        offset,
        sorted_by,
    };
    let featured_posts = state.post_service.get_latest_post_snapshots(cmd).await?;

    let wrapped_featured_posts = GetFeaturedPostsResponse {
        featured_posts: featured_posts
            .posts
            .into_iter()
            .map(|post| post.into())
            .collect(),
        has_more: featured_posts.has_more,
    };

    Ok(Json(wrapped_featured_posts))
}

#[derive(Deserialize)]
pub struct NewCommentBody {
    pub content: String,
    pub parent_id: Option<i64>,
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
                    parent_id: body.parent_id,
                    content: body.content,
                })
                .await?
        }
        None => {
            state
                .post_service
                .post_new_anonymous_comment(PostNewAnynymouseCommentCommand {
                    post_id,
                    parent_id: body.parent_id,
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
    pub parent_id: Option<i64>,
}

#[derive(Deserialize, Serialize)]
pub struct Comment {
    pub id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direct_reply_count: Option<i64>,
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
    has_more: bool,
}

pub async fn get_comments(
    State(state): State<Arc<AppState>>,
    Path(post_id_str): Path<String>,
    Query(query): Query<CommentsQuery>,
) -> Result<impl IntoResponse, PostError> {
    let before = query.before;
    let limit = query.limit.unwrap_or(1);
    let parent_id = query.parent_id;
    let post_id = post_id_str.parse::<i64>().unwrap();

    let page = state
        .post_service
        .get_comments(GetCommentsCommand {
            post_id,
            limit,
            before,
            parent_id,
        })
        .await?;

    let comments: Vec<Comment> = page
        .comments
        .into_iter()
        .map(|comment| Comment {
            id: comment.id,
            parent_id: comment.parent_id,
            direct_reply_count: comment.direct_reply_count,
            content: comment.content,
            created_at: comment.created_at,
            username: comment.username,
            display_name: comment.display_name,
            avatar_url: comment.avatar_url,
            user_role: comment.user_role,
        })
        .collect();

    let wrapped_comments = CommentsResponse {
        comments,
        has_more: page.has_more,
    };

    Ok(Json(wrapped_comments))
}

pub async fn push_view(
    State(state): State<Arc<AppState>>,
    Path(post_id_str): Path<String>,
) -> Result<impl IntoResponse, PostError> {
    let post_id = post_id_str.parse::<i64>().unwrap();

    state
        .post_service
        .push_new_view(PushNewViewCommand { post_id })
        .await?;
    Ok(())
}

pub async fn push_like(
    State(state): State<Arc<AppState>>,
    Path(post_id_str): Path<String>,
) -> Result<impl IntoResponse, PostError> {
    let post_id = post_id_str.parse::<i64>().unwrap();

    state
        .post_service
        .push_new_like(PushNewLikeCommand { post_id })
        .await?;
    Ok(())
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

#[derive(Deserialize)]
pub struct SetPostFeaturedBody {
    pub is_featured: bool,
}

pub async fn set_post_featured(
    State(state): State<Arc<AppState>>,
    Path(post_id): Path<i64>,
    Json(body): Json<SetPostFeaturedBody>,
) -> Result<impl IntoResponse, PostError> {
    let cmd = SetFeaturedPostCommand {
        post_id,
        is_featured: body.is_featured,
    };
    state.post_service.set_post_featured(cmd).await?;
    Ok(())
}
