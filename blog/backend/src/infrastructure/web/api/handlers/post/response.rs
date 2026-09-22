// Post wire responses and the conversions that build them from domain
// entities.
use serde::{Deserialize, Serialize};

use crate::domain::entities::post::PostSnapshot;

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
    pub is_featured: i64,
    pub medium_urls: Vec<String>,
    pub medium_short_names: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_media_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub og_image_url: Option<String>,
    pub og_image_seconds: i64,
    pub is_owner: bool,
    /// Optimistic-lock token the editor echoes back as `expected_updated_at`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Serialize)]
pub struct GetRelatedPostsResponse {
    pub posts: Vec<SearchPostResult>,
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
    pub medium_urls: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_media_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_video_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_video_type: Option<String>,
    pub og_image_seconds: i64,
    pub reading_time_minutes: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<PostSeriesResponse>,
    pub related_posts: Vec<SearchPostResult>,
}

#[derive(Serialize, Deserialize)]
pub struct CategoryResponse {
    pub name: String,
    pub slug: String,
}

#[derive(Serialize, Deserialize)]
pub struct GetCategoriesResponse {
    pub categories: Vec<CategoryResponse>,
}

#[derive(Serialize)]
pub struct UpdatePostResponse {
    pub updated_at: String,
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

#[derive(Serialize)]
pub struct SearchTagResult {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
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
    pub projects: Vec<crate::infrastructure::web::api::handlers::project::ProjectCard>,
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
    pub reading_time_minutes: i64,
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
            reading_time_minutes,
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
            reading_time_minutes,
        }
    }
}
#[derive(Serialize)]
pub struct GetFeaturedPostsResponse {
    pub featured_posts: Vec<Post>,
    pub has_more: bool,
}

#[derive(Serialize)]
pub struct NewCommentResponse {
    pub comment_id: i64,
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
    pub comments: Vec<Comment>,
    pub has_more: bool,
}
