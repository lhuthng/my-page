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
    ) -> Result<entities::auth::LoginResult, errors::auth::AuthError>;
    async fn register(
        &self,
        reg_creds: entities::auth::RegisterCredentials,
    ) -> Result<entities::auth::RegisterResult, errors::auth::AuthError>;
    async fn refresh_access_token(
        &self,
        cmd: commands::auth::RefreshAccessTokenCommand,
        config: entities::auth::AuthConfig,
    ) -> Result<entities::auth::AuthTokens, errors::auth::AuthError>;
    async fn verify_email(
        &self,
        cmd: commands::auth::VerifyEmailCommand,
    ) -> Result<(), errors::auth::AuthError>;
    async fn resend_verification(
        &self,
        cmd: commands::auth::ResendVerificationCommand,
    ) -> Result<entities::auth::ResendVerificationResult, errors::auth::AuthError>;
    async fn request_password_reset(
        &self,
        cmd: commands::auth::RequestPasswordResetCommand,
    ) -> Result<entities::auth::RequestPasswordResetResult, errors::auth::AuthError>;
    async fn reset_password(
        &self,
        cmd: commands::auth::ResetPasswordCommand,
    ) -> Result<(), errors::auth::AuthError>;
}
