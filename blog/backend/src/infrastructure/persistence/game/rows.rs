// sqlx FromRow structs for the game aggregate's queries.
use sqlx::prelude::FromRow;

#[derive(Debug, FromRow)]
pub(crate) struct GameSnapshotRow {
    pub(crate) game_id: i64,
    pub(crate) post_id: i64,
    pub(crate) title: String,
    pub(crate) slug: String,
    pub(crate) excerpt: String,
    pub(crate) author_name: String,
    pub(crate) author_slug: String,
    pub(crate) status: String,
    pub(crate) url: Option<String>,
    pub(crate) cover_media_type: Option<String>,
    pub(crate) launcher_type: String,
    pub(crate) views: i64,
    pub(crate) likes: i64,
    pub(crate) comments_count: i64,
    pub(crate) reading_time_minutes: i64,
}

#[derive(Debug, FromRow)]
pub(crate) struct GameContentRow {
    pub(crate) game_id: i64,
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
    pub(crate) launcher_type: String,
    pub(crate) demo_width: Option<String>,
    pub(crate) demo_height: Option<String>,
    pub(crate) demo_url: Option<String>,
    pub(crate) instruction: String,
    pub(crate) cheatcode: String,
    pub(crate) story: String,
}

#[derive(Debug, FromRow)]
pub(crate) struct GameTagRow {
    pub(crate) game_id: i64,
    pub(crate) tag_name: String,
    pub(crate) tag_slug: String,
}
