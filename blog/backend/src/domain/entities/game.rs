use serde::{Deserialize, Serialize};

use crate::domain::entities::post::PostStats;

#[derive(Debug, Clone, Serialize)]
pub struct GameDemo {
    pub launcher_type: String,
    pub width: Option<String>,
    pub height: Option<String>,
    pub demo_url: Option<String>,
    pub jsdos_bundle: Option<JsDosBundle>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsDosBundle {
    pub storage_key: String,
    pub original_file_name: String,
    pub size_bytes: i64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GameSnapshot {
    pub id: i64,
    pub post_id: i64,
    pub title: String,
    pub slug: String,
    pub tag_names: Vec<String>,
    pub tag_slugs: Vec<String>,
    pub excerpt: String,
    pub author_name: String,
    pub author_slug: String,
    pub status: String,
    pub url: Option<String>,
    pub cover_media_type: Option<String>,
    pub launcher_type: String,
    pub stats: PostStats,
    pub reading_time_minutes: i64,
}

#[derive(Debug, Clone)]
pub struct GameSnapshotPage {
    pub games: Vec<GameSnapshot>,
    pub has_more: bool,
}

#[derive(Debug, Clone)]
pub struct Game {
    pub id: i64,
    pub post_id: i64,
    pub title: String,
    pub slug: String,
    pub author_name: String,
    pub author_slug: String,
    pub author_avatar_url: Option<String>,
    pub tags: Vec<String>,
    pub excerpt: String,
    pub content: String,
    pub draft: String,
    pub published_at: Option<String>,
    pub updated_at: Option<String>,
    pub medium_urls: Vec<String>,
    pub medium_short_names: Vec<String>,
    pub cover_url: Option<String>,
    pub cover_media_type: Option<String>,
    pub cover_video_url: Option<String>,
    pub cover_video_type: Option<String>,
    pub og_image_seconds: i64,
    pub demo: GameDemo,
    pub instruction: String,
    pub cheatcode: String,
    pub story: String,
    pub related_games: Vec<GameLink>,
    pub is_owner: bool,
    pub og_image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameLink {
    pub id: i64,
    pub title: String,
    pub slug: String,
}