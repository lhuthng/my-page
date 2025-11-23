#[derive(serde::Deserialize)]
pub struct MeCommand {
    pub user_id: i64,
}

#[derive(serde::Deserialize)]
pub struct GetUserCommand {
    pub username: String,
}
