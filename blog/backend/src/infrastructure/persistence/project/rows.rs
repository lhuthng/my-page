// sqlx FromRow structs for the project aggregate's queries.
use sqlx::prelude::FromRow;

#[derive(Debug, FromRow)]
pub(crate) struct ProjectSnapshotRow {
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

#[derive(Debug, FromRow)]
pub(crate) struct ProjectContentRow {
    pub(crate) project_id: i64,
    pub(crate) post_id: i64,
    pub(crate) user_id: i64,
    pub(crate) author_name: String,
    pub(crate) author_slug: String,
    pub(crate) author_avatar_url: Option<String>,
    pub(crate) title: String,
    pub(crate) slug: String,
    pub(crate) excerpt: String,
    pub(crate) content: String,
    pub(crate) published_at: Option<String>,
    pub(crate) updated_at: Option<String>,
    pub(crate) cover_url: Option<String>,
    pub(crate) cover_media_type: Option<String>,
    pub(crate) cover_video_url: Option<String>,
    pub(crate) cover_video_type: Option<String>,
    pub(crate) og_image_seconds: i64,
    pub(crate) demo_type: String,
    pub(crate) demo_entry_path: String,
    pub(crate) demo_width: Option<String>,
    pub(crate) demo_height: Option<String>,
    pub(crate) demo_config: Option<String>,
    pub(crate) demo_url: Option<String>,
    pub(crate) delegate_game_id: Option<i64>,
    pub(crate) inherit_thumbnail: i64,
    pub(crate) inherit_tags: i64,
}

#[derive(Debug, FromRow)]
pub(crate) struct ProjectLinkRow {
    pub(crate) label: String,
    pub(crate) url: String,
}

#[derive(Debug, FromRow)]
pub(crate) struct ProjectTagRow {
    pub(crate) project_id: i64,
    pub(crate) tag_name: String,
    pub(crate) tag_slug: String,
}
