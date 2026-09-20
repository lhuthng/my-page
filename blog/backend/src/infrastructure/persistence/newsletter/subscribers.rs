// Subscriber lifecycle: subscribe, confirm, unsubscribe; plus the token
// helpers those flows share.
use sqlx::Row;

use crate::application::{
    commands::newsletter::SubscribeCommand,
    services::newsletter::NewsletterService,
};
use crate::domain::errors::newsletter::NewsletterError;

use super::rows::SubscriberRow;
use super::NewsletterServiceImpl;
fn hash_secret(secret: &str) -> String {
    hex::encode(Sha256::digest(secret.as_bytes()))
}

/// Generates a fresh random confirm token for the given subscriber id, in the
/// same scheme used by `email_verification_tokens` / `password_reset_tokens`:
/// a random secret is SHA256-hashed for storage, while the plaintext token
/// (base64 of `"{id}`{secret}"`) is handed back to embed in the email link.
fn generate_confirm_token(subscriber_id: i64) -> (String, String) {
    let mut token_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut token_bytes);

    let secret = hex::encode(token_bytes);
    let token_hash = hash_secret(&secret);
    let raw = format!("{}{}{}", subscriber_id, DELIMITER, secret);
    let token = general_purpose::URL_SAFE_NO_PAD.encode(raw);

    (token, token_hash)
}

fn decode_token(token: &str) -> Result<(i64, String), NewsletterError> {
    let raw = general_purpose::URL_SAFE_NO_PAD
        .decode(token)
        .map_err(|_| NewsletterError::InvalidToken)?;
    let raw = String::from_utf8(raw).map_err(|_| NewsletterError::InvalidToken)?;
    let parts: Vec<&str> = raw.splitn(2, DELIMITER).collect();
    if parts.len() != 2 {
        return Err(NewsletterError::InvalidToken);
    }

    let id = parts[0]
        .parse::<i64>()
        .map_err(|_| NewsletterError::InvalidToken)?;
    Ok((id, parts[1].to_string()))
}

/// The unsubscribe token is never stored in plaintext. Instead it is
/// deterministically derived from the subscriber id and the server's
/// `JWT_SECRET`, so it can be recomputed on every campaign send without a
/// dedicated lookup table. `unsubscribe_token_hash` still stores
/// `SHA256(secret)` (same shape as the confirm-token hash) purely so
/// verification stays a uniform "re-hash and compare" operation.
fn derive_unsubscribe_secret(subscriber_id: i64) -> String {
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_default();
    hex::encode(Sha256::digest(
        format!("{}{}{}", jwt_secret, DELIMITER, subscriber_id).as_bytes(),
    ))
}

fn derive_unsubscribe_token(subscriber_id: i64) -> (String, String) {
    let secret = derive_unsubscribe_secret(subscriber_id);
    let token_hash = hash_secret(&secret);
    let raw = format!("{}{}{}", subscriber_id, DELIMITER, secret);
    let token = general_purpose::URL_SAFE_NO_PAD.encode(raw);
    (token, token_hash)
}


#[async_trait::async_trait]
impl NewsletterService for NewsletterServiceImpl {
    async fn subscribe(
        &self,
        cmd: SubscribeCommand,
    ) -> Result<ConfirmSubscriptionMailPayload, NewsletterError> {
        let email = cmd.email.trim().to_string();

        let existing = sqlx::query_as::<_, SubscriberRow>(
            r#"
            SELECT id, email, status, confirm_token_hash, confirm_token_expires_at
            FROM newsletter_subscribers
            WHERE email = ?
            "#,
        )
        .bind(&email)
        .fetch_optional(&self.pool)
        .await?;

        let subscriber_id = match existing {
            Some(row) if row.status == "confirmed" => {
                return Err(NewsletterError::AlreadySubscribed);
            }
            Some(row) => row.id,
            None => {
                // Insert first (unsubscribe hash depends on the assigned id),
                // then backfill the deterministic unsubscribe token hash.
                let id: i64 = sqlx::query_scalar(
                    r#"
                    INSERT INTO newsletter_subscribers (email, status, unsubscribe_token_hash)
                    VALUES (?, 'pending', '')
                    RETURNING id
                    "#,
                )
                .bind(&email)
                .fetch_one(&self.pool)
                .await?;

                let (_, unsubscribe_token_hash) = derive_unsubscribe_token(id);
                sqlx::query("UPDATE newsletter_subscribers SET unsubscribe_token_hash = ? WHERE id = ?")
                    .bind(unsubscribe_token_hash)
                    .bind(id)
                    .execute(&self.pool)
                    .await?;

                id
            }
        };

        let (token, token_hash) = generate_confirm_token(subscriber_id);
        let now = Utc::now();
        let expires_at = now + Duration::minutes(CONFIRM_TOKEN_EXPIRY_MINUTES);

        sqlx::query(
            r#"
            UPDATE newsletter_subscribers
            SET confirm_token_hash = ?, confirm_token_expires_at = ?, confirm_sent_at = ?, status = 'pending'
            WHERE id = ?
            "#,
        )
        .bind(token_hash)
        .bind(expires_at.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(subscriber_id)
        .execute(&self.pool)
        .await?;

        Ok(ConfirmSubscriptionMailPayload { email, token })
    }

    async fn confirm_subscription(
        &self,
        cmd: ConfirmSubscriptionCommand,
    ) -> Result<(), NewsletterError> {
        let (subscriber_id, secret) = decode_token(&cmd.token)?;

        let row = sqlx::query_as::<_, SubscriberRow>(
            r#"
            SELECT id, email, status, confirm_token_hash, confirm_token_expires_at
            FROM newsletter_subscribers
            WHERE id = ?
            "#,
        )
        .bind(subscriber_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Err(NewsletterError::InvalidToken);
        };

        // A consumed token (already confirmed) or a previously unsubscribed
        // subscription should not leave the caller stuck on a failure screen:
        // report the end state as an idempotent success instead.
        match row.status.as_str() {
            "confirmed" => return Err(NewsletterError::AlreadyConfirmed),
            "unsubscribed" => return Err(NewsletterError::AlreadyUnsubscribed),
            _ => {}
        }

        let (Some(token_hash), Some(expires_at)) =
            (row.confirm_token_hash, row.confirm_token_expires_at)
        else {
            return Err(NewsletterError::InvalidToken);
        };

        if expires_at <= Utc::now() {
            return Err(NewsletterError::ExpiredToken);
        }

        if token_hash != hash_secret(&secret) {
            return Err(NewsletterError::InvalidToken);
        }

        sqlx::query(
            r#"
            UPDATE newsletter_subscribers
            SET status = 'confirmed',
                confirmed_at = ?,
                confirm_token_hash = NULL,
                confirm_token_expires_at = NULL
            WHERE id = ?
            "#,
        )
        .bind(Utc::now().to_rfc3339())
        .bind(subscriber_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn unsubscribe(&self, cmd: UnsubscribeCommand) -> Result<bool, NewsletterError> {
        let (subscriber_id, secret) = match decode_token(&cmd.token) {
            Ok(v) => v,
            // Unsubscribing is idempotent/friendly: an invalid token just
            // results in a no-op success instead of an error.
            Err(_) => return Ok(false),
        };

        let expected = derive_unsubscribe_secret(subscriber_id);
        if secret != expected {
            return Ok(false);
        }

        let result = sqlx::query(
            r#"
            UPDATE newsletter_subscribers
            SET status = 'unsubscribed', unsubscribed_at = ?
            WHERE id = ? AND status != 'unsubscribed'
            "#,
        )
        .bind(Utc::now().to_rfc3339())
        .bind(subscriber_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn unsubscribe_by_email(
        &self,
        cmd: UnsubscribeByEmailCommand,
    ) -> Result<bool, NewsletterError> {
        let email = cmd.email.trim().to_string();
        if email.is_empty() {
            return Ok(false);
        }

        let result = sqlx::query(
            r#"
            UPDATE newsletter_subscribers
            SET status = 'unsubscribed', unsubscribed_at = ?
            WHERE email = ? AND status != 'unsubscribed'
            "#,
        )
        .bind(Utc::now().to_rfc3339())
        .bind(&email)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

}
