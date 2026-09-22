// Session lifecycle: login and access-token refresh.
use base64::{Engine, engine::general_purpose};
use bcrypt::verify;
use chrono::{Duration, Utc};
use tokio::join;

use crate::application::commands::auth::{LoginCommand, RefreshAccessTokenCommand};
use crate::domain::entities::auth::{AuthConfig, AuthTokens, LoginResult};
use crate::domain::entities::secret::Claims;
use crate::domain::errors::auth::AuthError;
use crate::infrastructure::web::api::secrets::encode_into_jwt_token;

use super::rows::{SessionRow, UserRow, VerificationRow};
use super::tokens::{generate_token_pair, upsert_verification_token, verification_mail_payload};
use super::{AuthServiceImpl, DELIMITER};

impl AuthServiceImpl {
    pub(super) async fn login(
        &self,
        cmd: LoginCommand,
        config: AuthConfig,
    ) -> Result<LoginResult, AuthError> {
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
        // bcrypt at DEFAULT_COST is ~250 ms of pure CPU; keep it off the
        // async workers, like the session-token verify further down.
        let is_valid = {
            let password = cmd.password.clone();
            let password_hash = user_row.password_hash.clone();
            tokio::task::spawn_blocking(move || verify(&password, &password_hash))
                .await
                .map_err(|e| AuthError::InternalError(e.to_string()))?
                .map_err(|e| AuthError::InternalError(e.to_string()))?
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

    pub(super) async fn refresh_access_token(
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
                let is_valid =
                    tokio::task::spawn_blocking(move || verify(&token_owned, &token_hash))
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
}
