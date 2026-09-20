// Token plumbing: JWT pairs, verification/reset token generation, hashing,
and the mail payload builders.
use sqlx::SqlitePool;

use crate::domain::errors::auth::AuthError;

use super::rows::UserRow;

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

}
