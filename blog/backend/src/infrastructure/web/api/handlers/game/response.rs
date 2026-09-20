// Game wire responses and the conversions that build them from domain
// entities.
use serde::Serialize;

use crate::domain::entities::game::{Game, GameLink, GameSnapshot};
use crate::helper::time::normalize_optional_utc_timestamp;
use crate::infrastructure::web::api::handlers::v86::V86RuntimeDescriptor;
use crate::infrastructure::web::api::handlers::game::dto::{GameCard, GameStats};

#[derive(Serialize)]
pub struct UpdateGameResponse {
    pub updated_at: String,
}

/// Re-point a game's v86 artifact at another system version. Only the base
/// OS image changes; the game disk and launcher ISOs stay content-addressed
/// as-is, so this is a plain row update. Bumping `artifact_revision` mirrors
/// `attach_ready_game_tx` and makes any in-flight package build or snapshot
/// capture fail its revision/system checks instead of half-landing.
#[derive(Serialize)]
pub struct GameResponse {
    pub id: i64,
    pub post_id: i64,
    pub title: String,
    pub slug: String,
    pub tags: Vec<String>,
    pub author_name: String,
    pub author_slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_avatar_url: Option<String>,
    pub excerpt: String,
    pub content: String,
    pub medium_urls: Vec<String>,
    pub medium_short_names: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_media_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub og_image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_video_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_video_type: Option<String>,
    pub og_image_seconds: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    pub launcher_type: String,
    pub demo_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_demo_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demo_width: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demo_height: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsdos_bundle_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsdos_bundle_file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsdos_bundle_size_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v86_runtime: Option<V86RuntimeDescriptor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v86_system_version_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v86_manifest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v86_artifact_revision: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v86_game_file_name: Option<String>,
    pub instruction: String,
    pub cheatcode: String,
    pub story: String,
    pub related_games: Vec<GameLink>,
    pub is_owner: bool,
}

pub(super) fn game_response(game: Game) -> GameResponse {
    let mut demo_url = game.demo.demo_url.clone().unwrap_or_default();
    let raw_demo_url = if demo_url.contains("://") {
        Some(demo_url.clone())
    } else {
        None
    };
    let jsdos_bundle_url = game
        .demo
        .jsdos_bundle
        .as_ref()
        .map(|_| format!("games/s/{}/jsdos", game.slug));
    let jsdos_bundle_file_name = game
        .demo
        .jsdos_bundle
        .as_ref()
        .map(|bundle| bundle.original_file_name.clone());
    let jsdos_bundle_size_bytes = game
        .demo
        .jsdos_bundle
        .as_ref()
        .map(|bundle| bundle.size_bytes);
    if game.demo.launcher_type == "jsdos" && jsdos_bundle_url.is_some() {
        demo_url = format!("games/s/{}/jsdos", game.slug);
    }

    GameResponse {
        demo_url,
        id: game.id,
        post_id: game.post_id,
        title: game.title,
        slug: game.slug,
        tags: game.tags,
        author_name: game.author_name,
        author_slug: game.author_slug,
        author_avatar_url: game.author_avatar_url,
        excerpt: game.excerpt,
        content: game.content,
        medium_urls: game.medium_urls,
        medium_short_names: game.medium_short_names,
        cover_url: game.cover_url,
        cover_media_type: game.cover_media_type,
        og_image_url: game.og_image_url,
        cover_video_url: game.cover_video_url,
        cover_video_type: game.cover_video_type,
        og_image_seconds: game.og_image_seconds,
        published_at: normalize_optional_utc_timestamp(game.published_at),
        updated_at: normalize_optional_utc_timestamp(game.updated_at),
        launcher_type: game.demo.launcher_type,
        raw_demo_url,
        demo_width: game.demo.width,
        demo_height: game.demo.height,
        jsdos_bundle_url,
        jsdos_bundle_file_name,
        jsdos_bundle_size_bytes,
        v86_runtime: None,
        v86_system_version_id: None,
        v86_manifest: None,
        v86_artifact_revision: None,
        v86_game_file_name: None,
        instruction: game.instruction,
        cheatcode: game.cheatcode,
        story: game.story,
        related_games: game.related_games,
        is_owner: game.is_owner,
    }
}

impl From<GameSnapshot> for GameCard {
    fn from(value: GameSnapshot) -> Self {
        GameCard {
            id: value.id,
            post_id: value.post_id,
            title: value.title,
            slug: value.slug,
            tag_names: value.tag_names,
            tag_slugs: value.tag_slugs,
            excerpt: value.excerpt,
            author_name: value.author_name,
            author_slug: value.author_slug,
            launcher_type: value.launcher_type,
            url: value.url,
            cover_media_type: value.cover_media_type,
            stats: GameStats {
                views: value.stats.views,
                likes: value.stats.likes,
                comments: value.stats.comments,
            },
            reading_time_minutes: value.reading_time_minutes,
        }
    }
}

