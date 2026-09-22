// sqlx FromRow structs for dashboard queries.
use sqlx::prelude::FromRow;

#[derive(FromRow)]
pub(crate) struct RoleCountRow {
    pub(crate) role: String,
    pub(crate) count: i64,
}

#[derive(FromRow)]
pub(crate) struct GrowthDayRow {
    pub(crate) date: String,
    pub(crate) count: i64,
}

#[derive(FromRow)]
pub(crate) struct UserInfoRow {
    pub(crate) username: String,
    pub(crate) display_name: String,
    pub(crate) role: String,
    pub(crate) avatar_url: Option<String>,
    pub(crate) created_at: String,
}

#[derive(FromRow)]
pub(crate) struct DashTagRow {
    pub(crate) post_id: i64,
    pub(crate) tag_name: String,
    pub(crate) tag_slug: String,
}

#[derive(FromRow)]
pub(crate) struct DashProjectRow {
    pub(crate) project_id: i64,
    pub(crate) post_id: i64,
    pub(crate) title: String,
    pub(crate) slug: String,
    pub(crate) excerpt: String,
    pub(crate) author_name: String,
    pub(crate) author_slug: String,
    pub(crate) status: String,
    pub(crate) url: Option<String>,
    pub(crate) cover_media_type: Option<String>,
    pub(crate) demo_type: String,
    pub(crate) views: i64,
    pub(crate) likes: i64,
    pub(crate) comments_count: i64,
    pub(crate) reading_time_minutes: i64,
}

#[derive(FromRow)]
pub(crate) struct DashboardTagRecordRow {
    pub(crate) id: i64,
    pub(crate) name: String,
    pub(crate) slug: String,
    pub(crate) description: Option<String>,
    pub(crate) post_count: i64,
}
