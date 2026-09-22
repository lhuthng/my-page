// Campaign delivery: listing, per-post fan-out, manual campaigns, and the
// chunked run_campaign loop.

use crate::application::commands::newsletter::{ListSubscribersCommand, SendCampaignCommand};
use crate::domain::entities::newsletter::{CampaignSnapshot, SubscriberSnapshot};
use crate::domain::errors::newsletter::NewsletterError;
use crate::infrastructure::{mail::send_campaign_email, web::server::MailConfig};

use super::subscribers::derive_unsubscribe_token;
use super::{CAMPAIGN_CHUNK_DELAY_MS, CAMPAIGN_CHUNK_SIZE, NewsletterServiceImpl};

impl NewsletterServiceImpl {
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn run_campaign(
        &self,
        post_id: Option<i64>,
        source: &str,
        subject: String,
        body_html: String,
        body_text: String,
        sent_by_user_id: i64,
        mail_config: MailConfig,
        app_base_url: String,
    ) -> Result<(), NewsletterError> {
        // Insert the campaign row first: the unique index on `post_id` (where
        // not null and `source = 'publish'`) acts as the double-send guard. A
        // conflict here means this post already had a publish campaign fired,
        // which is expected and not an error. Manual sends never touch that
        // slot, so a one-off dashboard send can't block the publish campaign.
        let campaign_id: Option<i64> = match sqlx::query_scalar(
            r#"
            INSERT INTO newsletter_campaigns (post_id, source, subject, body_text, body_html, sent_by_user_id)
            VALUES (?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(post_id)
        .bind(source)
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

impl NewsletterServiceImpl {
    pub(super) async fn list_subscribers(
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
            .map(
                |(id, email, status, created_at, confirmed_at)| SubscriberSnapshot {
                    id,
                    email,
                    status,
                    created_at,
                    confirmed_at,
                },
            )
            .collect())
    }

    pub(super) async fn list_campaigns(
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

    #[allow(clippy::too_many_arguments)] // mirrors the port trait's signature
    pub(super) async fn send_campaign_for_post(
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
            "publish",
            subject,
            body_html,
            body_text,
            sent_by_user_id,
            mail_config,
            app_base_url,
        )
        .await
    }

    pub(super) async fn send_manual_campaign(
        &self,
        cmd: SendCampaignCommand,
        mail_config: MailConfig,
        app_base_url: String,
    ) -> Result<(), NewsletterError> {
        self.run_campaign(
            cmd.post_id,
            "manual",
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
