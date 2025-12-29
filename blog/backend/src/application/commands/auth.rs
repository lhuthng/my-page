#[derive(serde::Deserialize)]
pub struct LoginCommand {
    pub username: String,
    pub password: String,
}

#[derive(serde::Deserialize)]
pub struct RegisterCommand {
    pub username: String,
    pub password: String,
    pub email: String,
}

#[derive(serde::Deserialize)]
pub struct RefreshAccessTokenCommand {
    pub refresh_token: String,
}
