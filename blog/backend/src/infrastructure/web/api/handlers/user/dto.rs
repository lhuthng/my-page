// User wire types: query params, payloads, and response shapes.
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct MeResponse {
    username: String,
    display_name: String,
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChangeDetailsBody {
    pub display_name: Option<String>,
    pub bio: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GetUserResponse {
    username: String,
    display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    avatar_url: Option<String>,
    bio: String,
    role: String,
}

#[derive(Deserialize)]
pub struct GetPostsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub tag_names: Vec<String>,
    pub tag_slugs: Vec<String>,
    pub excerpt: String,
    pub author_name: String,
    pub author_slug: String,
    pub status: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_media_type: Option<String>,
    pub reading_time_minutes: i64,
}

#[derive(Serialize)]
pub struct GetPostsResponse {
    pub posts: Vec<Post>,
}

#[derive(Serialize)]
pub struct LatestComment {
    pub id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<i64>,
    pub content: String,
    pub created_at: String,
    pub post_title: String,
    pub post_slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

#[derive(Serialize)]
pub struct GetLatestCommentsResponse {
    pub comments: Vec<LatestComment>,
}

#[derive(Deserialize)]
pub struct GetLatestCommentsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct CheckModResponse {
    is_authorized: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchUserQuery {
    pub term: String,
    pub size: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct SearchUserResult {
    pub username: String,
    pub display_name: String,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
}

#[derive(Serialize)]
pub struct SearchUserResponse {
    pub users: Vec<SearchUserResult>,
}

