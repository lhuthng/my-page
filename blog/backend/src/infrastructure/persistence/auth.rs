use base64::{Engine, engine::general_purpose};
use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, SqlitePool};
use tokio::join;
use validator::Validate;

use crate::{
    application::{
        commands::auth::{
            LoginCommand, RefreshAccessTokenCommand, RequestPasswordResetCommand,
            ResendVerificationCommand, ResetPasswordCommand, VerifyEmailCommand,
        },
        services::auth::AuthService,
    },
    domain::{
        entities::{
            auth::{
                AuthConfig, AuthTokens, LoginResult, RegisterCredentials, RegisterResult,
                PasswordResetMailPayload, RequestPasswordResetResult, ResendVerificationResult,
                ResetPasswordCredentials, VerificationMailPayload,
            },
            secret::Claims,
        },
        errors::auth::AuthError,
    },
    infrastructure::web::api::secrets::encode_into_jwt_token,
};

const DELIMITER: char = '`';
const EMAIL_VERIFICATION_EXPIRY_MINUTES: i64 = 30;
const RESEND_VERIFICATION_COOLDOWN_SECONDS: i64 = 60;
const PASSWORD_RESET_EXPIRY_MINUTES: i64 = 30;
const PASSWORD_RESET_COOLDOWN_SECONDS: i64 = 60;

pub struct AuthServiceImpl {
    pub pool: SqlitePool,
}

#[derive(FromRow, Debug)]
struct UserRow {
    id: i64,
    username: String,
    password_hash: String,
    email: String,
    role: String,
    email_verified_at: Option<DateTime<Utc>>,
}

#[derive(FromRow, Debug)]
struct SessionRow {
    user_id: i64,
    role: String,
    token_hash: String,
    expires_at: DateTime<Utc>,
}

#[derive(FromRow, Debug)]
struct VerificationRow {
    token_hash: String,
    expires_at: DateTime<Utc>,
    sent_at: DateTime<Utc>,
}

impl AuthServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

async fn generate_token_pair(id: &i64) -> (String, String) {
    let mut token_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut token_bytes);
    let mut token = hex::encode(token_bytes);

    let token_hash = hash(&token, DEFAULT_COST).unwrap();

    let raw = format!("{}{}{}", id, DELIMITER, token);
    token = general_purpose::URL_SAFE_NO_PAD.encode(raw);

    (token, token_hash)
}

fn hash_verification_secret(secret: &str) -> String {
    hex::encode(Sha256::digest(secret.as_bytes()))
}

fn generate_verification_token(user_id: i64) -> (String, String) {
    let mut token_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut token_bytes);

    let secret = hex::encode(token_bytes);
    let token_hash = hash_verification_secret(&secret);
    let raw = format!("{}{}{}", user_id, DELIMITER, secret);
    let token = general_purpose::URL_SAFE_NO_PAD.encode(raw);

    (token, token_hash)
}

fn decode_verification_token(token: &str) -> Result<(i64, String), AuthError> {
    let raw = general_purpose::URL_SAFE_NO_PAD
        .decode(token)
        .map_err(|_| AuthError::InvalidToken)?;
    let raw = String::from_utf8(raw).map_err(|_| AuthError::InvalidToken)?;
    let parts: Vec<&str> = raw.splitn(2, DELIMITER).collect();
    if parts.len() != 2 {
        return Err(AuthError::InvalidToken);
    }

    let user_id = parts[0]
        .parse::<i64>()
        .map_err(|_| AuthError::InvalidToken)?;
    Ok((user_id, parts[1].to_string()))
}

async fn upsert_verification_token(pool: &SqlitePool, user_id: i64) -> Result<String, AuthError> {
    let (token, token_hash) = generate_verification_token(user_id);
    let now = Utc::now();
    let expires_at = now + Duration::minutes(EMAIL_VERIFICATION_EXPIRY_MINUTES);

    sqlx::query(
        r#"
        INSERT INTO email_verification_tokens (user_id, token_hash, expires_at, sent_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (user_id)
        DO UPDATE SET
            token_hash = excluded.token_hash,
            expires_at = excluded.expires_at,
            sent_at = excluded.sent_at
        "#,
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at.to_rfc3339())
    .bind(now.to_rfc3339())
    .execute(pool)
    .await?;

    Ok(token)
}

fn verification_mail_payload(user_row: &UserRow, token: String) -> VerificationMailPayload {
    VerificationMailPayload {
        username: user_row.username.clone(),
        email: user_row.email.clone(),
        token,
    }
}

fn password_reset_mail_payload(user_row: &UserRow, token: String) -> PasswordResetMailPayload {
    PasswordResetMailPayload {
        username: user_row.username.clone(),
        email: user_row.email.clone(),
        token,
    }
}

async fn upsert_password_reset_token(pool: &SqlitePool, user_id: i64) -> Result<String, AuthError> {
    let (token, token_hash) = generate_verification_token(user_id);
    let now = Utc::now();
    let expires_at = now + Duration::minutes(PASSWORD_RESET_EXPIRY_MINUTES);

    sqlx::query(
        r#"
        INSERT INTO password_reset_tokens (user_id, token_hash, expires_at, sent_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (user_id)
        DO UPDATE SET
            token_hash = excluded.token_hash,
            expires_at = excluded.expires_at,
            sent_at = excluded.sent_at
        "#,
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at.to_rfc3339())
    .bind(now.to_rfc3339())
    .execute(pool)
    .await?;

    Ok(token)
}

#[async_trait::async_trait]
impl AuthService for AuthServiceImpl {
    async fn login(&self, cmd: LoginCommand, config: AuthConfig) -> Result<LoginResult, AuthError> {
        let user_row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, username, password_hash, email, role, email_verified_at
            FROM users
            WHERE username = ?
            "#,
        )
        .bind(cmd.username)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        let is_valid = match verify(&cmd.password, &user_row.password_hash) {
            Ok(valid) => valid,
            Err(e) => return Err(AuthError::InternalError(e.to_string())),
        };

        if !is_valid {
            return Err(AuthError::InvalidCredentials);
        }

        if user_row.email_verified_at.is_none() {
            let verification_row = sqlx::query_as::<_, VerificationRow>(
                r#"
                SELECT token_hash, expires_at, sent_at
                FROM email_verification_tokens
                WHERE user_id = ?
                "#,
            )
            .bind(user_row.id)
            .fetch_optional(&self.pool)
            .await?;

            let verification_mail = match verification_row {
                Some(row) if row.expires_at > Utc::now() => None,
                _ => {
                    let token = upsert_verification_token(&self.pool, user_row.id).await?;
                    Some(verification_mail_payload(&user_row, token))
                }
            };

            return Ok(LoginResult::VerificationRequired { verification_mail });
        }

        let claims = Claims::new(
            user_row.id.to_string(),
            user_row.role,
            user_row.email_verified_at.is_some(),
            config.access_expire_hours,
        );

        let (access_token_res, refresh_token_res) = join!(
            encode_into_jwt_token(claims, &config.header, &config.encoding_key,),
            generate_token_pair(&user_row.id),
        );

        let access_token = access_token_res?;
        let (refresh_token, refresh_token_hash) = refresh_token_res;

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO sessions (user_id, token_hash, expires_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id)
            DO UPDATE SET
                token_hash = excluded.token_hash,
                expires_at = excluded.expires_at;
            "#,
        )
        .bind(user_row.id)
        .bind(refresh_token_hash)
        .bind((Utc::now() + Duration::hours(config.refresh_expire_hours)).to_rfc3339())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(LoginResult::Authenticated(AuthTokens {
            access_token,
            refresh_token,
        }))
    }

    async fn register(&self, reg_creds: RegisterCredentials) -> Result<RegisterResult, AuthError> {
        let existing_row = sqlx::query(
            r#"
            SELECT username, email
            FROM users
            WHERE username = ? OR email = ?
            "#,
        )
        .bind(&reg_creds.username)
        .bind(&reg_creds.email)
        .fetch_optional(&self.pool)
        .await?;

        match existing_row {
            Some(_) => Err(AuthError::UserAlreadyExists),
            None => {
                let mut tx = self.pool.begin().await?;

                let password_hash = hash(&reg_creds.password, DEFAULT_COST)?;

                let user = sqlx::query_as::<_, UserRow>(
                    r#"
                    INSERT INTO users (username, password_hash, email)
                    VALUES ($1, $2, $3)
                    RETURNING *
                    "#,
                )
                .bind(&reg_creds.username)
                .bind(&password_hash)
                .bind(&reg_creds.email)
                .fetch_one(&mut *tx)
                .await?;

                sqlx::query(
                    r#"
                    INSERT INTO user_meta (user_id, display_name)
                    VALUES ($1, $2)
                    "#,
                )
                .bind(user.id)
                .bind(user.username.clone())
                .execute(&mut *tx)
                .await?;

                tx.commit().await?;

                let token = upsert_verification_token(&self.pool, user.id).await?;

                Ok(RegisterResult {
                    verification_mail: verification_mail_payload(&user, token),
                })
            }
        }
    }

    async fn refresh_access_token(
        &self,
        cmd: RefreshAccessTokenCommand,
        config: AuthConfig,
    ) -> Result<AuthTokens, AuthError> {
        let raw = general_purpose::URL_SAFE_NO_PAD
            .decode(&cmd.refresh_token)
            .map_err(|_| AuthError::InvalidToken)?;
        let raw = String::from_utf8(raw).map_err(|_| AuthError::InvalidToken)?;

        let parts: Vec<&str> = raw.splitn(2, DELIMITER).collect();
        if parts.len() != 2 {
            return Err(AuthError::InvalidToken);
        }
        let user_id = parts[0];
        let token = parts[1];

        match sqlx::query_as::<_, SessionRow>(
            r#"
            SELECT user_id, token_hash, expires_at, role
            FROM sessions JOIN users ON users.id = user_id
            WHERE user_id = ? AND users.email_verified_at IS NOT NULL
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        {
            Ok(row) => {
                let token_hash = row.token_hash.clone();
                let token_owned = token.to_string();
                let is_valid = tokio::task::spawn_blocking(move || verify(&token_owned, &token_hash))
                    .await
                    .map_err(|e| AuthError::InternalError(e.to_string()))?
                    .map_err(|e| AuthError::InternalError(e.to_string()))?;

                if !is_valid {
                    return Err(AuthError::InvalidToken);
                }
                if Utc::now() > row.expires_at {
                    return Err(AuthError::ExpiredToken);
                }

                let claims = Claims::new(
                    row.user_id.to_string(),
                    row.role.to_string(),
                    true,
                    config.access_expire_hours,
                );

                let access_token =
                    encode_into_jwt_token(claims, &config.header, &config.encoding_key).await?;

                Ok(AuthTokens {
                    access_token,
                    refresh_token: cmd.refresh_token,
                })
            }
            Err(sqlx::Error::RowNotFound) => Err(AuthError::InvalidToken),
            Err(e) => Err(AuthError::InternalError(e.to_string())),
        }
    }

    async fn verify_email(&self, cmd: VerifyEmailCommand) -> Result<(), AuthError> {
        let (user_id, secret) = decode_verification_token(&cmd.token)?;
        let verification_row = sqlx::query_as::<_, VerificationRow>(
            r#"
            SELECT token_hash, expires_at, sent_at
            FROM email_verification_tokens
            WHERE user_id = ?
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = verification_row else {
            return Err(AuthError::InvalidToken);
        };

        if row.expires_at <= Utc::now() {
            sqlx::query("DELETE FROM email_verification_tokens WHERE user_id = ?")
                .bind(user_id)
                .execute(&self.pool)
                .await?;
            return Err(AuthError::ExpiredToken);
        }

        if row.token_hash != hash_verification_secret(&secret) {
            return Err(AuthError::InvalidToken);
        }

        let mut tx = self.pool.begin().await?;

        sqlx::query("UPDATE users SET email_verified_at = ? WHERE id = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM email_verification_tokens WHERE user_id = ?")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM sessions WHERE user_id = ?")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }

    async fn resend_verification(
        &self,
        cmd: ResendVerificationCommand,
    ) -> Result<ResendVerificationResult, AuthError> {
        let identifier = cmd.identifier.trim();
        if identifier.is_empty() {
            return Err(AuthError::Validation(
                "Username or email is required.".to_string(),
            ));
        }

        let user_row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, username, password_hash, email, role, email_verified_at
            FROM users
            WHERE username = ? OR email = ?
            "#,
        )
        .bind(identifier)
        .bind(identifier)
        .fetch_optional(&self.pool)
        .await?;

        let Some(user_row) = user_row else {
            return Ok(ResendVerificationResult::UserNotFound);
        };

        if user_row.email_verified_at.is_some() {
            return Ok(ResendVerificationResult::AlreadyVerified);
        }

        let existing = sqlx::query_as::<_, VerificationRow>(
            r#"
            SELECT token_hash, expires_at, sent_at
            FROM email_verification_tokens
            WHERE user_id = ?
            "#,
        )
        .bind(user_row.id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = existing {
            let elapsed = Utc::now() - row.sent_at;
            if elapsed < Duration::seconds(RESEND_VERIFICATION_COOLDOWN_SECONDS) {
                return Err(AuthError::ResendTooSoon);
            }
        }

        let token = upsert_verification_token(&self.pool, user_row.id).await?;

        Ok(ResendVerificationResult::VerificationMailQueued(
            verification_mail_payload(&user_row, token),
        ))
    }

    async fn request_password_reset(
        &self,
        cmd: RequestPasswordResetCommand,
    ) -> Result<RequestPasswordResetResult, AuthError> {
        let username = cmd.username.trim();
        let email = cmd.email.trim();
        if username.is_empty() || email.is_empty() {
            return Err(AuthError::Validation(
                "Username and email are required.".to_string(),
            ));
        }

        let user_row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, username, password_hash, email, role, email_verified_at
            FROM users
            WHERE username = ? AND email = ?
            "#,
        )
        .bind(username)
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        let Some(user_row) = user_row else {
            return Ok(RequestPasswordResetResult::UserNotFound);
        };

        if user_row.email_verified_at.is_none() {
            return Ok(RequestPasswordResetResult::UserNotFound);
        }

        let existing = sqlx::query_as::<_, VerificationRow>(
            r#"
            SELECT token_hash, expires_at, sent_at
            FROM password_reset_tokens
            WHERE user_id = ?
            "#,
        )
        .bind(user_row.id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = existing {
            let elapsed = Utc::now() - row.sent_at;
            if elapsed < Duration::seconds(PASSWORD_RESET_COOLDOWN_SECONDS) {
                return Ok(RequestPasswordResetResult::RecentlySent);
            }
        }

        let token = upsert_password_reset_token(&self.pool, user_row.id).await?;

        Ok(RequestPasswordResetResult::ResetMailQueued(
            password_reset_mail_payload(&user_row, token),
        ))
    }

    async fn reset_password(&self, cmd: ResetPasswordCommand) -> Result<(), AuthError> {
        let reset_creds = ResetPasswordCredentials {
            password: cmd.password,
        };
        if let Err(err) = reset_creds.validate() {
            return Err(AuthError::Validation(err.to_string()));
        }

        let (user_id, secret) = decode_verification_token(&cmd.token)?;
        let reset_row = sqlx::query_as::<_, VerificationRow>(
            r#"
            SELECT token_hash, expires_at, sent_at
            FROM password_reset_tokens
            WHERE user_id = ?
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = reset_row else {
            return Err(AuthError::InvalidToken);
        };

        if row.expires_at <= Utc::now() {
            sqlx::query("DELETE FROM password_reset_tokens WHERE user_id = ?")
                .bind(user_id)
                .execute(&self.pool)
                .await?;
            return Err(AuthError::ExpiredToken);
        }

        if row.token_hash != hash_verification_secret(&secret) {
            return Err(AuthError::InvalidToken);
        }

        let password_hash = hash(&reset_creds.password, DEFAULT_COST)?;
        let mut tx = self.pool.begin().await?;

        sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
            .bind(password_hash)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM password_reset_tokens WHERE user_id = ?")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM sessions WHERE user_id = ?")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }
}
