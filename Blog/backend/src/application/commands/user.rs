use axum::body::Bytes;

#[derive(serde::Deserialize)]
pub struct MeCommand {
    pub user_id: i64,
}

#[derive(serde::Deserialize)]
pub struct ChangeDetailsCommand {
    pub user_id: i64,
    pub display_name: Option<String>,
    pub bio: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct GetUserCommand {
    pub username: String,
}

#[derive(serde::Deserialize)]
pub struct GetRoleCommand {}

#[derive(serde::Deserialize)]
pub struct GetPostsCommand {
    pub username: String,
    pub limit: i64,
    pub offset: i64,
}
