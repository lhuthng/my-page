#[derive(Debug, Clone)]
pub struct CategoryResult {
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone)]
pub struct PostResult {
    pub title: String,
    pub slug: String,
    pub tag_names: Vec<String>,
    pub tag_slugs: Vec<String>,
    pub excerpt: String,
    pub author_name: String,
    pub author_slug: String,
}
