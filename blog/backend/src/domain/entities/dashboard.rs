use super::post::PostSnapshot;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DashboardUserInfo {
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub avatar_url: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoleCounts {
    pub admin: i64,
    pub moderator: i64,
    pub user: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GrowthPoint {
    pub date: String,
    pub new_posts: i64,
    pub new_users: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardOverview {
    pub total_published: i64,
    pub total_drafts: i64,
    pub total_users: i64,
    pub total_comments: i64,
    pub top_posts_by_views: Vec<PostSnapshot>,
    pub top_posts_by_likes: Vec<PostSnapshot>,
    pub top_posts_by_comments: Vec<PostSnapshot>,
    pub recent_posts: Vec<PostSnapshot>,
    pub recent_users: Vec<DashboardUserInfo>,
    pub role_counts: RoleCounts,
    pub growth: Vec<GrowthPoint>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardPostsResult {
    pub posts: Vec<PostSnapshot>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardUsersResult {
    pub users: Vec<DashboardUserInfo>,
    pub total: i64,
    pub role_counts: RoleCounts,
}
