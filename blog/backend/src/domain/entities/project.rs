use serde::{Deserialize, Serialize};

use crate::domain::entities::post::PostStats;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectLink {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectDemo {
    pub demo_type: String,
    pub entry_path: String,
    pub width: Option<String>,
    pub height: Option<String>,
    pub config: Option<String>,
    pub demo_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectSnapshot {
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
    pub demo_type: String,
    pub stats: PostStats,
}

#[derive(Debug, Clone)]
pub struct ProjectSnapshotPage {
    pub projects: Vec<ProjectSnapshot>,
    pub has_more: bool,
}

#[derive(Debug, Clone)]
pub struct Project {
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
    pub demo: ProjectDemo,
    pub links: Vec<ProjectLink>,
    pub is_owner: bool,
}
