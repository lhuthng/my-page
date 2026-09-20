// sqlx `FromRow` structs for the post aggregate's queries.
use sqlx::prelude::FromRow;

#[derive(Debug, FromRow)]
pub struct PostRow {
    pub post_id: i64,
    pub title: String,
    pub slug: String,
    pub excerpt: String,
    pub author_name: String,
    pub author_slug: String,
    pub url: Option<String>,
    pub cover_media_type: Option<String>,
    pub status: String,
    pub views: i64,
    pub likes: i64,
    pub comments_count: i64,
    pub reading_time_minutes: i64,
}

#[derive(Debug, FromRow)]
pub struct PostContentRow {
    pub post_id: i64,
    pub author_name: String,
    pub author_slug: String,
    pub author_avatar_url: Option<String>,
    pub title: String,
    pub excerpt: String,
    pub content: String,
    pub published_at: Option<String>,
    pub updated_at: Option<String>,
    pub url: Option<String>,
    pub cover_media_type: Option<String>,
    pub cover_video_url: Option<String>,
    pub cover_video_type: Option<String>,
    pub og_image_seconds: i64,
    pub reading_time_minutes: i64,
}

#[derive(Debug, FromRow)]
pub struct PostSearchRow {
    pub title: String,
    pub slug: String,
    pub cover_image_url: Option<String>,
    #[allow(dead_code)]
    pub score: i32,
}

#[derive(Debug, FromRow)]
pub struct TagRow {
    pub post_id: i64,
    pub tag_name: String,
    pub tag_slug: String,
}

#[derive(Debug, FromRow)]
pub struct TagSummaryRow {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub post_count: i64,
    #[allow(dead_code)]
    pub score: i32,
}

#[derive(Debug, FromRow)]
pub struct MediumUsageRow {
    pub code: i64,
    pub url: String,
}

#[derive(Debug, FromRow)]
pub struct MediumUsageWithNameRow {
    pub code: i64,
    pub url: String,
    pub short_name: String,
}

#[derive(Debug, FromRow)]
pub struct PostDetailsRow {
    pub post_id: i64,
    pub updated_at: Option<String>,
    pub title: String,
    pub slug: String,
    pub excerpt: String,
    pub series_id: Option<i64>,
    pub content: String,
    pub user_id: i64,
    pub is_featured: i64,
    pub cover_url: Option<String>,
    pub cover_media_type: Option<String>,
    pub og_image_seconds: i64,
}

