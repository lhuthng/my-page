// Credential lifecycle: registration, password reset request and reset.
use sqlx::Row;

use crate::application::{
    commands::auth::ResetPasswordCommand,
    services::auth::AuthService,
};
use crate::domain::errors::auth::AuthError;

use super::rows::UserRow;
use super::tokens::{
    generate_token_pair, hash_verification_secret, password_reset_mail_payload,
    upsert_password_reset_token,
};
use super::AuthServiceImpl;

#[async_trait::async_trait]
impl AuthService for AuthServiceImpl {
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

                let reg_password = reg_creds.password.clone();
                let password_hash =
                    tokio::task::spawn_blocking(move || hash(&reg_password, DEFAULT_COST))
                        .await
                        .map_err(|e| AuthError::InternalError(e.to_string()))??;

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

        let reset_password = reset_creds.password.clone();
        let password_hash = tokio::task::spawn_blocking(move || hash(&reset_password, DEFAULT_COST))
            .await
            .map_err(|e| AuthError::InternalError(e.to_string()))??;
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
