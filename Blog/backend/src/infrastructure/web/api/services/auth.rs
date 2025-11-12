use base64::{Engine, engine::general_purpose};
use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use sqlx::{FromRow, SqlitePool};
use tokio::join;

use crate::{
    application::{
        commands::auth::{LoginCommand, RefreshAccessTokenCommand, RegisterCommand},
        services::auth::AuthService,
    },
    domain::{
        entities::{
            auth::{AuthConfig, AuthTokens},
            secret::Claims,
        },
        errors::auth::AuthError,
    },
    infrastructure::web::api::secrets::encode_into_jwt_token,
};

const DELIMITER: char = '`';

pub struct AuthServiceImpl {
    pub pool: SqlitePool,
}

#[derive(FromRow, Debug)]
struct UserRow {
    id: i64,
    username: String,
    password_hash: String,
    role: String,
}

#[derive(FromRow, Debug)]
struct SessionRow {
    user_id: i64,
    role: String,
    token_hash: String,
    expires_at: DateTime<Utc>,
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

#[async_trait::async_trait]
impl AuthService for AuthServiceImpl {
    async fn login(&self, cmd: LoginCommand, config: AuthConfig) -> Result<AuthTokens, AuthError> {
        let user_row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, username, password_hash, role
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

        let claims = Claims::new(
            user_row.id.to_string(),
            user_row.role,
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

        Ok(AuthTokens {
            access_token,
            refresh_token,
        })
    }

    async fn register(&self, cmd: RegisterCommand) -> Result<(), AuthError> {
        let existing_row = sqlx::query(
            r#"
            SELECT username, email
            FROM users
            WHERE username = ? OR email = ?
            "#,
        )
        .bind(&cmd.username)
        .bind(&cmd.email)
        .fetch_optional(&self.pool)
        .await?;

        match existing_row {
            Some(_) => Err(AuthError::UserAlreadyExists),
            None => {
                let mut tx = self.pool.begin().await?;

                let password_hash = hash(&cmd.password, DEFAULT_COST)?;

                let user = sqlx::query_as::<_, UserRow>(
                    r#"
                    INSERT INTO users (username, password_hash, email)
                    VALUES ($1, $2, $3)
                    RETURNING *
                    "#,
                )
                .bind(&cmd.username)
                .bind(&password_hash)
                .bind(&cmd.email)
                .fetch_one(&mut *tx)
                .await?;

                sqlx::query(
                    r#"
                    INSERT INTO user_meta (user_id, display_name)
                    VALUES ($1, $2)
                    "#,
                )
                .bind(&user.id)
                .bind(&user.username)
                .execute(&mut *tx)
                .await?;

                tx.commit().await?;

                Ok(())
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
            WHERE user_id = ?
            "#,
        )
        .bind(&user_id)
        .fetch_one(&self.pool)
        .await
        {
            Ok(row) => {
                match verify(token, &row.token_hash) {
                    Ok(value) => {
                        if !value {
                            return Err(AuthError::InvalidToken);
                        }
                    }
                    Err(error) => {
                        return Err(AuthError::InternalError(error.to_string()));
                    }
                }
                if Utc::now() > row.expires_at {
                    return Err(AuthError::ExpiredToken);
                }

                let claims = Claims::new(
                    row.user_id.to_string(),
                    row.role.to_string(),
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
}
