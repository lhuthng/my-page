// Wire types for the game feature: request bodies, query params, and
// response shapes. Handlers do not declare structs inline.
use serde::{Deserialize, Serialize};

use crate::domain::entities::game::GameLink;

#[derive(Deserialize)]
pub struct CheckQuery {
    pub slug: Option<String>,
}

#[derive(Serialize)]
pub struct CheckResponse {
    pub exists: bool,
}

#[derive(Deserialize)]
pub struct GameData {
    pub title: String,
    pub slug: String,
    pub excerpt: String,
    pub content: String,
    pub tags: Vec<String>,
    pub number_of_files: usize,
    pub launcher_type: String,
    pub demo_width: Option<String>,
    pub demo_height: Option<String>,
    pub demo_url: Option<String>,
    pub instruction: String,
    pub cheatcode: String,
    pub story: String,
    pub related_games: Vec<GameLink>,
    pub v86_upload_id: Option<String>,
}

#[derive(Deserialize)]
pub struct GamePatchData {
    pub title: Option<String>,
    pub slug: Option<String>,
    pub excerpt: Option<String>,
    pub content: Option<String>,
    pub tags: Option<Vec<String>>,
    pub number_of_files: usize,
    pub launcher_type: Option<String>,
    pub demo_width: Option<String>,
    pub demo_height: Option<String>,
    pub demo_url: Option<String>,
    pub instruction: Option<String>,
    pub cheatcode: Option<String>,
    pub story: Option<String>,
    pub related_games: Option<Vec<GameLink>>,
    pub og_image_seconds: Option<i64>,
    pub v86_upload_id: Option<String>,
    pub v86_system_version_id: Option<i64>,
    pub expected_updated_at: Option<String>,
}

#[derive(Deserialize)]
pub struct StartJsDosUploadRequest {
    pub file_name: String,
    pub size_bytes: u64,
}

#[derive(Deserialize, Serialize)]
pub struct StartJsDosUploadResponse {
    pub upload_id: String,
    pub chunk_size_bytes: u64,
    pub next_chunk_index: u64,
    pub expected_size_bytes: u64,
}

#[derive(Serialize)]
pub struct JsDosUploadResponse {
    pub received_size_bytes: u64,
    pub next_chunk_index: u64,
}

#[derive(Serialize)]
pub struct CompleteJsDosUploadResponse {
    pub game_id: i64,
    pub file_name: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub bundle_url: String,
}
#[derive(Deserialize)]
pub struct DeleteGameQuery {
    pub reason: Option<String>,
    pub detail: Option<String>,
    pub force: Option<bool>,
}
#[derive(Deserialize)]
pub struct LatestGamesQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct GameStats {
    pub views: i64,
    pub likes: i64,
    pub comments: i64,
}

#[derive(Serialize)]
pub struct GameCard {
    pub id: i64,
    pub post_id: i64,
    pub title: String,
    pub slug: String,
    pub tag_names: Vec<String>,
    pub tag_slugs: Vec<String>,
    pub excerpt: String,
    pub author_name: String,
    pub author_slug: String,
    pub launcher_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub cover_media_type: Option<String>,
    pub stats: GameStats,
    pub reading_time_minutes: i64,
}

#[derive(Serialize)]
pub struct LatestGamesResponse {
    pub games: Vec<GameCard>,
    pub has_more: bool,
}

#[derive(Deserialize)]
pub struct FeaturedGamesQuery {
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct FeaturedGamesResponse {
    pub featured_games: Vec<GameCard>,
    pub has_more: bool,
}
#[derive(Deserialize)]
pub struct SetGameFeaturedBody {
    pub is_featured: bool,
}
