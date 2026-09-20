// sqlx FromRow structs for dashboard queries.
use sqlx::prelude::FromRow;

#[derive(FromRow)]
struct RoleCountRow {
    role: String,
    count: i64,
}

#[derive(FromRow)]
struct GrowthDayRow {
    date: String,
    count: i64,
}

#[derive(FromRow)]
struct UserInfoRow {
    username: String,
    display_name: String,
    role: String,
    avatar_url: Option<String>,
    created_at: String,
}

#[derive(FromRow)]
struct DashTagRow {
    post_id: i64,
    tag_name: String,
    tag_slug: String,
}

#[derive(FromRow)]
struct DashProjectRow {
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

#[derive(FromRow)]
struct DashboardTagRecordRow {
    id: i64,
    name: String,
    slug: String,
    description: Option<String>,
    post_count: i64,
}

