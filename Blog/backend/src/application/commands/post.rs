use std::collections::HashMap;

pub struct CheckSlugCommand {
    pub post_slug: String,
}

pub struct PostCommand {
    pub user_id: i64,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub categories: Vec<String>,
    pub cover_image: Option<String>,
    pub media_usage: HashMap<String, i64>,
}

pub struct PublishCommand {
    pub user_id: i64,
    pub slug: String,
}

pub struct GetCategoriesCommand {}
