use bcrypt::{BcryptError, DEFAULT_COST, hash, verify};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use tokio::join;

use crate::{
    application::{
        commands::auth::{LoginCommand, RefreshTokenCommand, RegisterCommand},
        services::auth::AuthService,
    },
    domain::{
        entities::{
            auth::{AuthConfig, AuthTokens},
            user::{User, UserRole},
            value_objects::{Email, HashedPassword},
        },
        errors::auth::AuthError,
    },
};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Claims {
    sub: String,
    exp: usize,
}

pub struct AuthServiceImpl {
    pub pool: SqlitePool,
}

#[derive(FromRow, Debug)]
struct UserRow {
    id: i64,
    username: String,
    password_hash: String,
}

#[derive(FromRow, Debug)]
struct UserMetaRow {
    user_id: i64,
    display_name: String,
    bio: String,
}

#[derive(FromRow, Debug)]
struct SessionRow {
    user_id: i64,
    token_hash: String,
    expires_at: DateTime<Utc>,
}

impl AuthServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

async fn create_jwt_token(
    value: String,
    expire_hours: i64,
    header: &Header,
    encoding_key: &EncodingKey,
) -> Result<String, AuthError> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(expire_hours))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: value.to_string(),
        exp: expiration,
    };

    encode(&header, &claims, &encoding_key)
        .map_err(|_| AuthError::InternalError("Access Token Creation".to_string()))
}

async fn decode_jwt_token(token: String, decoding_key: &DecodingKey) -> Result<String, String> {
    let validation = Validation::default();

    match decode::<Claims>(&token, decoding_key, &validation) {
        Ok(token_data) => Ok(token_data.claims.sub),
        Err(err) => Err(format!("Invalid token: {}", err)),
    }
}

async fn generate_token_pair() -> (String, String) {
    let mut token_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut token_bytes);
    let token = hex::encode(token_bytes);

    let token_hash = hash(&token, DEFAULT_COST).unwrap();

    (token, token_hash)
}

#[async_trait::async_trait]
impl AuthService for AuthServiceImpl {
    async fn login(&self, cmd: LoginCommand, config: AuthConfig) -> Result<AuthTokens, AuthError> {
        let user_row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, username, password_hash
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

        let (access_token_res, refresh_token_res) = join!(
            create_jwt_token(
                user_row.id.to_string(),
                config.access_expire_hours,
                &config.header,
                &config.encoding_key,
            ),
            generate_token_pair(),
        );

        let access_token = access_token_res?;
        let (refresh_token, refresh_token_hash) = refresh_token_res;

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO sessions (user_id, token_hash, expires_at)
            VALUES ($1, $2, $3)
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

    async fn refresh_token(
        &self,
        cmd: RefreshTokenCommand,
        refresh_token: String,
        config: AuthConfig,
    ) -> Result<AuthTokens, AuthError> {
        let user_id = decode_jwt_token(cmd.access_token, &config.decoding_key).await?;

        match sqlx::query_as::<_, SessionRow>(
            r#"
            SELECT user_id, token_hash, expires_at
            FROM sessions
            WHERE user_id = ?
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        {
            Ok(row) => {
                let is_valid = match verify(&refresh_token, &row.token_hash) {
                    Ok(valid) => valid,
                    Err(BcryptError::InvalidHash(_)) => false,
                    Err(e) => return Err(AuthError::InternalError(e.to_string())),
                };

                if !is_valid {
                    return Err(AuthError::InvalidCredentials);
                }
                if Utc::now() > row.expires_at {
                    return Err(AuthError::ExpiredToken);
                }
                let access_token = create_jwt_token(
                    row.user_id.to_string(),
                    config.access_expire_hours,
                    &config.header,
                    &config.encoding_key,
                )
                .await?;

                Ok(AuthTokens {
                    access_token,
                    refresh_token,
                })
            }
            Err(sqlx::Error::RowNotFound) => Err(AuthError::InvalidToken),
            Err(e) => Err(AuthError::InternalError(e.to_string())),
        }
    }
}
