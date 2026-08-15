use base64::{Engine, engine::general_purpose};
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, SqlitePool};

use crate::{
    application::{
        commands::newsletter::{
            ConfirmSubscriptionCommand, ListSubscribersCommand, SendCampaignCommand,
            SubscribeCommand, UnsubscribeByEmailCommand, UnsubscribeCommand,
        },
        services::newsletter::NewsletterService,
    },
    domain::{
        entities::newsletter::{
            CampaignSnapshot, ConfirmSubscriptionMailPayload, SubscriberSnapshot,
        },
        errors::newsletter::NewsletterError,
    },
    infrastructure::{mail::send_campaign_email, web::server::MailConfig},
};

const DELIMITER: char = '`';
const CONFIRM_TOKEN_EXPIRY_MINUTES: i64 = 30;
const CAMPAIGN_CHUNK_SIZE: usize = 25;
const CAMPAIGN_CHUNK_DELAY_MS: u64 = 300;

pub struct NewsletterServiceImpl {
    pub pool: SqlitePool,
}

impl NewsletterServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow, Debug)]
struct SubscriberRow {
    id: i64,
    #[allow(dead_code)]
    email: String,
    status: String,
    confirm_token_hash: Option<String>,
    confirm_token_expires_at: Option<DateTime<Utc>>,
}

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

    async fn list_subscribers(
        &self,
        cmd: ListSubscribersCommand,
    ) -> Result<Vec<SubscriberSnapshot>, NewsletterError> {
        if !cmd.is_admin {
            return Err(NewsletterError::PermissionDenied);
        }

        let rows = sqlx::query_as::<_, (i64, String, String, String, Option<String>)>(
            r#"
            SELECT id, email, status, created_at, confirmed_at
            FROM newsletter_subscribers
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, email, status, created_at, confirmed_at)| SubscriberSnapshot {
                id,
                email,
                status,
                created_at,
                confirmed_at,
            })
            .collect())
    }

    async fn list_campaigns(
        &self,
        cmd: ListSubscribersCommand,
    ) -> Result<Vec<CampaignSnapshot>, NewsletterError> {
        if !cmd.is_admin {
            return Err(NewsletterError::PermissionDenied);
        }

        let rows = sqlx::query_as::<
            _,
            (
                i64,
                Option<i64>,
                String,
                i64,
                i64,
                i64,
                String,
                Option<String>,
            ),
        >(
            r#"
            SELECT id, post_id, subject, recipient_count, success_count, failure_count, started_at, completed_at
            FROM newsletter_campaigns
            ORDER BY started_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    post_id,
                    subject,
                    recipient_count,
                    success_count,
                    failure_count,
                    started_at,
                    completed_at,
                )| CampaignSnapshot {
                    id,
                    post_id,
                    subject,
                    recipient_count,
                    success_count,
                    failure_count,
                    started_at,
                    completed_at,
                },
            )
            .collect())
    }

    async fn send_campaign_for_post(
        &self,
        post_id: i64,
        subject: String,
        body_html: String,
        body_text: String,
        sent_by_user_id: i64,
        mail_config: MailConfig,
        app_base_url: String,
    ) -> Result<(), NewsletterError> {
        self.run_campaign(
            Some(post_id),
            subject,
            body_html,
            body_text,
            sent_by_user_id,
            mail_config,
            app_base_url,
        )
        .await
    }

    async fn send_manual_campaign(
        &self,
        cmd: SendCampaignCommand,
        mail_config: MailConfig,
        app_base_url: String,
    ) -> Result<(), NewsletterError> {
        self.run_campaign(
            cmd.post_id,
            cmd.subject,
            cmd.body_html,
            cmd.body_text,
            cmd.sent_by_user_id,
            mail_config,
            app_base_url,
        )
        .await
    }
}

impl NewsletterServiceImpl {
    #[allow(clippy::too_many_arguments)]
    async fn run_campaign(
        &self,
        post_id: Option<i64>,
        subject: String,
        body_html: String,
        body_text: String,
        sent_by_user_id: i64,
        mail_config: MailConfig,
        app_base_url: String,
    ) -> Result<(), NewsletterError> {
        // Insert the campaign row first: the unique index on `post_id` (where
        // not null) acts as the double-send guard. A conflict here means this
        // post already had a campaign fired, which is expected and not an error.
        let campaign_id: Option<i64> = match sqlx::query_scalar(
            r#"
            INSERT INTO newsletter_campaigns (post_id, subject, body_text, body_html, sent_by_user_id)
            VALUES (?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(post_id)
        .bind(&subject)
        .bind(&body_text)
        .bind(&body_html)
        .bind(sent_by_user_id)
        .fetch_one(&self.pool)
        .await
        {
            Ok(id) => Some(id),
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => None,
            Err(e) => return Err(NewsletterError::InternalError(e.to_string())),
        };

        let Some(campaign_id) = campaign_id else {
            return Ok(());
        };

        let subscriber_ids: Vec<(i64, String)> = sqlx::query_as(
            r#"
            SELECT id, email FROM newsletter_subscribers WHERE status = 'confirmed'
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        sqlx::query("UPDATE newsletter_campaigns SET recipient_count = ? WHERE id = ?")
            .bind(subscriber_ids.len() as i64)
            .bind(campaign_id)
            .execute(&self.pool)
            .await?;

        let mut success_count: i64 = 0;
        let mut failure_count: i64 = 0;

        for chunk in subscriber_ids.chunks(CAMPAIGN_CHUNK_SIZE) {
            for (subscriber_id, email) in chunk {
                let (unsubscribe_token, _) = derive_unsubscribe_token(*subscriber_id);
                let result = send_campaign_email(
                    &mail_config,
                    &app_base_url,
                    email,
                    &unsubscribe_token,
                    &subject,
                    &body_html,
                    &body_text,
                )
                .await;

                match result {
                    Ok(()) => success_count += 1,
                    Err(err) => {
                        tracing::error!(
                            "Failed to send newsletter campaign {} to {}: {}",
                            campaign_id,
                            email,
                            err
                        );
                        failure_count += 1;
                    }
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(CAMPAIGN_CHUNK_DELAY_MS)).await;
        }

        sqlx::query(
            r#"
            UPDATE newsletter_campaigns
            SET success_count = ?, failure_count = ?, completed_at = datetime('now')
            WHERE id = ?
            "#,
        )
        .bind(success_count)
        .bind(failure_count)
        .bind(campaign_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
