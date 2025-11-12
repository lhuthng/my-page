use crate::{
    application::commands,
    domain::{entities, errors},
};

#[async_trait::async_trait]
pub trait AuthService {
    async fn login(
        &self,
        cmd: commands::auth::LoginCommand,
        config: entities::auth::AuthConfig,
    ) -> Result<entities::auth::AuthTokens, errors::auth::AuthError>;
    async fn register(
        &self,
        cmd: commands::auth::RegisterCommand,
    ) -> Result<(), errors::auth::AuthError>;
    async fn refresh_access_token(
        &self,
        cmd: commands::auth::RefreshAccessTokenCommand,
        config: entities::auth::AuthConfig,
    ) -> Result<entities::auth::AuthTokens, errors::auth::AuthError>;
}
