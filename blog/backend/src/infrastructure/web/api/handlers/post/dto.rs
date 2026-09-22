// Wire types for the post feature: request bodies, query params, and
// response shapes. Handlers do not declare structs inline.
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CheckQuery {
    pub slug: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CheckResponse {
    pub exists: bool,
}

#[derive(Deserialize)]
pub struct PostData {
    pub title: String,
    pub slug: String,
    pub excerpt: String,
    pub content: String,
    pub tags: Vec<String>,
    pub number_of_files: usize,
}

#[derive(Deserialize)]
pub struct PostPatchData {
    pub title: Option<String>,
    pub slug: Option<String>,
    pub excerpt: Option<String>,
    pub content: Option<String>,
    pub tags: Option<Vec<String>>,
    pub number_of_files: usize,
    pub og_image_seconds: Option<i64>,
    /// Optional optimistic-lock token; see `UpdatePostCommand`.
    pub expected_updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchPostQuery {
    pub term: String,
    pub size: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Deserialize)]
pub struct SearchTagsQuery {
    pub term: Option<String>,
    pub size: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Deserialize)]
pub struct GetTagPostsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Deserialize)]
pub struct GetFeaturedPostsBody {
    pub limit: i64,
}

#[derive(Deserialize)]
pub struct GetFeaturedPostsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sorted_by_updated: Option<bool>,
    pub sorted_by_created: Option<bool>,
}

#[derive(Deserialize)]
pub struct NewCommentBody {
    pub content: String,
    pub parent_id: Option<i64>,
    pub guest_identity: Option<String>,
}

#[derive(Deserialize)]
pub struct CommentsQuery {
    pub before: Option<i64>,
    pub limit: Option<i64>,
    pub parent_id: Option<i64>,
}

#[derive(Deserialize)]
pub struct SetPostFeaturedBody {
    pub is_featured: bool,
}

#[derive(Deserialize)]
pub struct DeletePostQuery {
    pub reason: Option<String>,
    pub detail: Option<String>,
}

#[derive(Deserialize)]
pub struct SetRelatedPostsBody {
    pub related_post_slugs: Vec<String>,
}
