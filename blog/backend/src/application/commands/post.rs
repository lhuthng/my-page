use std::collections::HashMap;

pub struct CheckSlugCommand {
    pub post_slug: String,
}

pub struct NewPostCommand {
    pub user_id: i64,
    pub title: String,
    pub slug: String,
    pub excerpt: String,
    pub content: String,
    pub tags: Vec<String>,
    pub cover_image: Option<String>,
    pub media_usage: HashMap<String, i64>,
}

pub struct UpdatePostCommand {
    pub user_id: i64,
    pub post_id: i64,
    pub title: Option<String>,
    pub slug: Option<String>,
    pub excerpt: Option<String>,
    pub content: Option<String>,
    pub draft: Option<String>,
    pub tags: Option<Vec<String>>,
    pub media_usage: Option<HashMap<String, i64>>,
}

pub struct GetPostCommand {
    pub slug: String,
    pub as_id: Option<i64>,
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
    pub sorted_by: String,
}

pub struct GetDetailedPostsCommand {
    pub required_author_id: Option<i64>,
    pub viewing_user_id: i64,
    pub post_id: i64,
}

pub struct SearchPostCommand {
    pub term: String,
    pub size: i64,
    pub offset: i64,
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

pub struct GetCommentsCommand {
    pub post_id: i64,
    pub limit: i64,
    pub before: Option<i64>,
}

pub struct PushNewViewCommand {
    pub post_id: i64,
}

pub struct PushNewLikeCommand {
    pub post_id: i64,
}
