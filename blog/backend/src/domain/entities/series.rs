use serde::Serialize;

use crate::domain::entities::post::PostSnapshot;

#[derive(Debug, Clone)]
pub struct SeriesSnapshot {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub description: String,
    pub url: Option<String>,
    pub post_count: i64,
    pub owner_username: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SeriesWithPosts {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub username: String,
    pub display_name: String,
    pub description: String,
    pub url: Option<String>,
    pub posts: Vec<PostSnapshot>,
    pub numbers: Vec<i64>,
}
