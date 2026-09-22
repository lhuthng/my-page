// Auth persistence adapter: implements
// `application::services::auth::AuthService` against SQLite.
use sqlx::SqlitePool;

mod credentials;
mod rows;
mod sessions;
mod tokens;

const DELIMITER: char = '`';
const EMAIL_VERIFICATION_EXPIRY_MINUTES: i64 = 30;
const RESEND_VERIFICATION_COOLDOWN_SECONDS: i64 = 60;
const PASSWORD_RESET_EXPIRY_MINUTES: i64 = 30;
const PASSWORD_RESET_COOLDOWN_SECONDS: i64 = 60;

pub struct AuthServiceImpl {
    pub pool: SqlitePool,
}

impl AuthServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

// `AuthService` port implementation: thin delegation to the inherent
// methods in the sibling files (inherent candidates win over trait
// candidates in method resolution, so `self.<method>` calls the local one).
use crate::application::commands;
use crate::application::services::auth::AuthService;
use crate::domain::{entities, errors};

#[async_trait::async_trait]
impl AuthService for AuthServiceImpl {
    async fn login(
        &self,
        cmd: commands::auth::LoginCommand,
        config: entities::auth::AuthConfig,
    ) -> Result<entities::auth::LoginResult, errors::auth::AuthError> {
        self.login(cmd, config).await
    }
    async fn register(
        &self,
        reg_creds: entities::auth::RegisterCredentials,
    ) -> Result<entities::auth::RegisterResult, errors::auth::AuthError> {
        self.register(reg_creds).await
    }
    async fn refresh_access_token(
        &self,
        cmd: commands::auth::RefreshAccessTokenCommand,
        config: entities::auth::AuthConfig,
    ) -> Result<entities::auth::AuthTokens, errors::auth::AuthError> {
        self.refresh_access_token(cmd, config).await
    }
    async fn verify_email(
        &self,
        cmd: commands::auth::VerifyEmailCommand,
    ) -> Result<(), errors::auth::AuthError> {
        self.verify_email(cmd).await
    }
    async fn resend_verification(
        &self,
        cmd: commands::auth::ResendVerificationCommand,
    ) -> Result<entities::auth::ResendVerificationResult, errors::auth::AuthError> {
        self.resend_verification(cmd).await
    }
    async fn request_password_reset(
        &self,
        cmd: commands::auth::RequestPasswordResetCommand,
    ) -> Result<entities::auth::RequestPasswordResetResult, errors::auth::AuthError> {
        self.request_password_reset(cmd).await
    }
    async fn reset_password(
        &self,
        cmd: commands::auth::ResetPasswordCommand,
    ) -> Result<(), errors::auth::AuthError> {
        self.reset_password(cmd).await
    }
}
