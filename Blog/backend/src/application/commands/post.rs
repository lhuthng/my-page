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

pub struct GetPostCommand {
    pub slug: String,
}

pub struct PublishCommand {
    pub user_id: i64,
    pub post_id: i64,
}

pub struct GetCategoriesCommand {}

pub struct GetFeaturedPostsCommand {
    pub limit: i64,
}

pub struct GetLatestPostsCommand {
    pub limit: i64,
    pub offset: i64,
}

pub struct GetDetailedPostsCommand {
    pub required_author_id: Option<i64>,
    pub post_id: i64,
}

pub struct PostNewCommentCommand {
    pub post_id: i64,
    pub user_id: i64,
    pub content: String,
}
pub struct PostNewAnynymouseCommentCommand {
    pub post_id: i64,
    pub content: String,
}
