// sqlx FromRow structs for the project aggregate's queries.
use sqlx::prelude::FromRow;

#[derive(Debug, FromRow)]
struct ProjectSnapshotRow {
    project_id: i64,
    post_id: i64,
    title: String,
    slug: String,
    excerpt: String,
    author_name: String,
    author_slug: String,
    status: String,
    url: Option<String>,
    cover_media_type: Option<String>,
    demo_type: String,
    views: i64,
    likes: i64,
    comments_count: i64,
    reading_time_minutes: i64,
}

#[derive(Debug, FromRow)]
struct ProjectContentRow {
    project_id: i64,
    post_id: i64,
    user_id: i64,
    author_name: String,
    author_slug: String,
    author_avatar_url: Option<String>,
    title: String,
    slug: String,
    excerpt: String,
    content: String,
    published_at: Option<String>,
    updated_at: Option<String>,
    cover_url: Option<String>,
    cover_media_type: Option<String>,
    cover_video_url: Option<String>,
    cover_video_type: Option<String>,
    og_image_seconds: i64,
    demo_type: String,
    demo_entry_path: String,
    demo_width: Option<String>,
    demo_height: Option<String>,
    demo_config: Option<String>,
    demo_url: Option<String>,
    delegate_game_id: Option<i64>,
    inherit_thumbnail: i64,
    inherit_tags: i64,
}

#[derive(Debug, FromRow)]
struct ProjectLinkRow {
    label: String,
    url: String,
}

#[derive(Debug, FromRow)]
struct ProjectTagRow {
    project_id: i64,
    tag_name: String,
    tag_slug: String,
}

