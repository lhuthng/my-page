#[derive(Debug, Clone)]
pub struct CategoryResult {
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone)]
pub struct PostSnapshot {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub tag_names: Vec<String>,
    pub tag_slugs: Vec<String>,
    pub excerpt: String,
    pub author_name: String,
    pub author_slug: String,
    pub status: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PostDetails {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub tags: Vec<String>,
    pub excerpt: String,
    pub series_slug: Option<String>,
    pub series_cover_url: Option<String>,
    pub content: String,
    pub draft: String,
    pub is_featured: i64,
    pub cover_url: Option<String>,
    pub medium_urls: Vec<String>,
    pub medium_short_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PostSummary {
    pub title: String,
    pub slug: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub author_name: String,
    pub author_slug: String,
    pub author_avatar_url: Option<String>,
    pub tags: Vec<String>,
    pub excerpt: String,
    pub content: String,
    pub draft: String,
    pub published_at: Option<String>,
    pub medium_urls: Vec<String>,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Comment {
    pub id: i64,
    pub content: String,
    pub created_at: String,
    pub display_name: Option<String>,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub user_role: Option<String>,
}
