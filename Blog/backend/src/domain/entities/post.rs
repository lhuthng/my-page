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
    pub url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PostDetails {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub tags: Vec<String>,
    pub excerpt: String,
    pub content: String,
    pub draft: String,
    pub is_featured: i64,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Post {
    pub title: String,
    pub author_name: String,
    pub author_slug: String,
    pub tags: Vec<String>,
    pub content: String,
    pub published_at: Option<String>,
    pub medium_urls: Vec<String>,
    pub cover_url: Option<String>,
}
