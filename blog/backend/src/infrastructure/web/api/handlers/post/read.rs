// Post read endpoints: slugs, details, search, tags, categories, feeds.
use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path as AxumPath, Query, State},
    response::IntoResponse,
};

use crate::{
    application::{
        commands::{
            post::{
                CheckSlugCommand, GetCategoriesCommand, GetDetailedPostsCommand,
                GetFeaturedPostsCommand, GetLatestPostsCommand, GetPostsByTagCommand,
                GetRelatedPostsCommand, SearchPostCommand, SearchTagsCommand,
            },
            project::GetProjectsByTagCommand,
        },
        services::{post::PostService, project::ProjectService},
    },
    domain::{entities::secret::Claims, errors::post::PostError},
    infrastructure::web::{
        api::handlers::post::dto::{
            CheckQuery, CheckResponse, GetFeaturedPostsQuery, GetTagPostsQuery, SearchPostQuery,
            SearchTagsQuery,
        },
        api::handlers::post::response::{
            GetFeaturedPostsResponse, GetPostDetailsResponse, GetRelatedPostsResponse,
            Post, PostSeriesResponse, SearchPostResponse, SearchTagsResponse, TagPostsResponse,
        },
        server::AppState,
    },
};

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
        is_featured,
        cover_url,
        cover_media_type,
        og_image_url,
        og_image_seconds,
        medium_urls,
        medium_short_names,
        is_owner,
        updated_at,
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
        is_featured,
        cover_url,
        cover_media_type,
        og_image_url,
        og_image_seconds,
        medium_urls,
        medium_short_names,
        is_owner,
        updated_at,
    }))
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

pub async fn get_post_by_slug(
    State(state): State<Arc<AppState>>,
    Extension(_opt_claims): Extension<Option<Claims>>,
    Path(post_slug): Path<String>,
) -> Result<impl IntoResponse, PostError> {
    let post = state
        .post_service
        .get_post(GetPostCommand { slug: post_slug })
        .await?;
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
        medium_urls: post.medium_urls,
        published_at: normalize_optional_utc_timestamp(post.published_at),
        updated_at: normalize_optional_utc_timestamp(post.updated_at),
        cover_media_type: post.cover_media_type,
        cover_video_url: post.cover_video_url,
        cover_video_type: post.cover_video_type,
        og_image_seconds: post.og_image_seconds,
        reading_time_minutes: post.reading_time_minutes,
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

#[axum::debug_handler]
pub async fn search(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchPostQuery>,
) -> Result<impl IntoResponse, PostError> {
    let term = query.term;
    let size = crate::helper::string::clamp_page_size(query.size, 1, 100);
    let offset = crate::helper::string::clamp_offset(query.offset);

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
            size: crate::helper::string::clamp_page_size(query.size, 24, 100),
            offset: crate::helper::string::clamp_offset(query.offset),
        })
        .await?;

    let tags = tags
        .into_iter()
        .map(
            |TagSummary {
                 name,
                 slug,
                 description,
                 post_count,
             }| SearchTagResult {
                name,
                slug,
                description,
                post_count,
            },
        )
        .collect();

    Ok(Json(SearchTagsResponse { tags }))
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
            limit: crate::helper::string::clamp_page_size(query.limit, 24, 100),
            offset: crate::helper::string::clamp_offset(query.offset),
        })
        .await?;
    let projects = state
        .project_service
        .get_project_snapshots_by_tag(GetProjectsByTagCommand {
            slug: tag.slug.clone(),
            limit: crate::helper::string::clamp_page_size(query.limit, 24, 100),
            offset: crate::helper::string::clamp_offset(query.offset),
        })
        .await
        .unwrap_or_default();

    Ok(Json(TagPostsResponse {
        tag: SearchTagResult {
            name: tag.name,
            slug: tag.slug,
            description: tag.description,
            post_count: tag.post_count,
        },
        posts: posts.into_iter().map(Into::into).collect(),
        projects: projects.into_iter().map(Into::into).collect(),
    }))
}

#[axum::debug_handler]
pub async fn get_featured_posts(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetFeaturedPostsBody>,
) -> Result<impl IntoResponse, PostError> {
    let cmd = GetFeaturedPostsCommand {
        limit: crate::helper::string::clamp_page_size(Some(query.limit), 1, 100),
    };
    let featured_posts = state.post_service.get_featured_post_snapshots(cmd).await?;

    let wrapped_featured_posts = GetFeaturedPostsResponse {
        featured_posts: featured_posts.into_iter().map(|post| post.into()).collect(),
        has_more: false,
    };

    Ok(Json(wrapped_featured_posts))
}

#[axum::debug_handler]
pub async fn get_latest_posts(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetFeaturedPostsQuery>,
) -> Result<impl IntoResponse, PostError> {
    let limit = crate::helper::string::clamp_page_size(query.limit, 1, 100);
    let offset = crate::helper::string::clamp_offset(query.offset);
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

