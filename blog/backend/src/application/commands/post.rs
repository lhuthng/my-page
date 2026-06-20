use std::collections::HashMap;

pub struct CheckSlugCommand {
    pub post_slug: String,
}

#[allow(dead_code)]
pub struct NewPostCommand {
    pub user_id: i64,
    pub title: String,
    pub slug: String,
    pub excerpt: String,
    pub content: String,
    pub tags: Vec<String>,
    pub cover_media: Option<String>,
    pub media_usage: HashMap<String, i64>,
    pub content_kind: String,
}

#[allow(dead_code)]
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

pub struct SearchTagsCommand {
    pub term: Option<String>,
    pub size: i64,
    pub offset: i64,
}

pub struct GetPostsByTagCommand {
    pub slug: String,
    pub limit: i64,
    pub offset: i64,
}

pub struct PostNewCommentCommand {
    pub post_id: i64,
    pub user_id: i64,
    pub parent_id: Option<i64>,
    pub content: String,
    pub guest_identity: Option<String>,
}
pub struct PostNewAnynymouseCommentCommand {
    pub post_id: i64,
    pub parent_id: Option<i64>,
    pub content: String,
    pub guest_identity: String,
}

pub struct GetCommentsCommand {
    pub post_id: i64,
    pub limit: i64,
    pub before: Option<i64>,
    pub parent_id: Option<i64>,
}

pub struct PushNewViewCommand {
    pub post_id: i64,
}

pub struct PushNewLikeCommand {
    pub post_id: i64,
}

pub struct GetRelatedPostsCommand {
    pub post_id: i64,
}

#[allow(dead_code)]
pub struct SetRelatedPostsCommand {
    pub user_id: i64,
    pub post_id: i64,
    pub related_post_slugs: Vec<String>,
}

pub struct SetFeaturedPostCommand {
    pub post_id: i64,
    pub is_featured: bool,
}

#[allow(dead_code)]
pub struct UpdatePostCoverCommand {
    pub user_id: i64,
    pub post_id: i64,
    pub video_short_name: Option<String>,
    pub og_image_seconds: Option<i64>,
}
