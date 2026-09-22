// Project wire responses and the conversions that build them from domain
// entities.
use serde::Serialize;

use crate::domain::entities::project::{Project, ProjectLink, ProjectSnapshot};
use crate::helper::time::normalize_optional_utc_timestamp;
use crate::infrastructure::web::api::handlers::project::dto::{ProjectCard, ProjectStats};
use crate::infrastructure::web::api::handlers::v86::V86RuntimeDescriptor;

#[derive(Serialize)]
pub struct UpdateProjectResponse {
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct ProjectResponse {
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
    pub demo_type: String,
    pub demo_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_demo_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demo_width: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demo_height: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demo_config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delegate_game_id: Option<i64>,
    pub inherit_thumbnail: bool,
    pub inherit_tags: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delegated_game: Option<DelegatedGameResponse>,
    pub links: Vec<ProjectLink>,
    pub is_owner: bool,
}

#[derive(Serialize)]
pub struct DelegatedGameResponse {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub launcher_type: String,
    pub demo_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demo_width: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demo_height: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsdos_bundle_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v86_runtime: Option<V86RuntimeDescriptor>,
}

pub(super) fn project_response(project: Project) -> ProjectResponse {
    let demo_url = project.demo.demo_url.clone().unwrap_or_default();
    let raw_demo_url = if demo_url.contains("://") {
        Some(demo_url.clone())
    } else {
        None
    };
    ProjectResponse {
        demo_url,
        id: project.id,
        post_id: project.post_id,
        title: project.title,
        slug: project.slug,
        tags: project.tags,
        author_name: project.author_name,
        author_slug: project.author_slug,
        author_avatar_url: project.author_avatar_url,
        excerpt: project.excerpt,
        content: project.content,
        medium_urls: project.medium_urls,
        medium_short_names: project.medium_short_names,
        cover_url: project.cover_url,
        cover_media_type: project.cover_media_type,
        og_image_url: project.og_image_url,
        cover_video_url: project.cover_video_url,
        cover_video_type: project.cover_video_type,
        og_image_seconds: project.og_image_seconds,
        published_at: normalize_optional_utc_timestamp(project.published_at),
        updated_at: normalize_optional_utc_timestamp(project.updated_at),
        demo_type: project.demo.demo_type,
        raw_demo_url,
        demo_width: project.demo.width,
        demo_height: project.demo.height,
        demo_config: project.demo.config,
        delegate_game_id: project.delegate_game_id,
        inherit_thumbnail: project.inherit_thumbnail,
        inherit_tags: project.inherit_tags,
        delegated_game: project.delegated_game.map(|game| DelegatedGameResponse {
            jsdos_bundle_url: game
                .jsdos_bundle
                .as_ref()
                .map(|_| format!("games/s/{}/jsdos", game.slug)),
            demo_url: if game.jsdos_bundle.is_some() {
                format!("games/s/{}/jsdos", game.slug)
            } else {
                game.demo_url.unwrap_or_default()
            },
            id: game.id,
            title: game.title,
            slug: game.slug,
            launcher_type: game.launcher_type,
            demo_width: game.width,
            demo_height: game.height,
            v86_runtime: None,
        }),
        links: project.links,
        is_owner: project.is_owner,
    }
}

impl From<ProjectSnapshot> for ProjectCard {
    fn from(value: ProjectSnapshot) -> Self {
        ProjectCard {
            id: value.id,
            post_id: value.post_id,
            title: value.title,
            slug: value.slug,
            tag_names: value.tag_names,
            tag_slugs: value.tag_slugs,
            excerpt: value.excerpt,
            author_name: value.author_name,
            author_slug: value.author_slug,
            demo_type: value.demo_type,
            url: value.url,
            cover_media_type: value.cover_media_type,
            stats: ProjectStats {
                views: value.stats.views,
                likes: value.stats.likes,
                comments: value.stats.comments,
            },
            reading_time_minutes: value.reading_time_minutes,
        }
    }
}
