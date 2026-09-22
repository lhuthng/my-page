// Newsletter persistence adapter: implements
// `application::services::newsletter::NewsletterService` against SQLite.
use sqlx::SqlitePool;

mod campaigns;
mod rows;
mod subscribers;

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

// `NewsletterService` port implementation: thin delegation to the inherent
// methods in the sibling files (inherent candidates win over trait
// candidates in method resolution, so `self.<method>` calls the local one).
use crate::application::commands;
use crate::application::services::newsletter::NewsletterService;
use crate::domain::{entities, errors};
use crate::infrastructure::web::server::MailConfig;

#[async_trait::async_trait]
impl NewsletterService for NewsletterServiceImpl {
    async fn subscribe(
        &self,
        cmd: commands::newsletter::SubscribeCommand,
    ) -> Result<
        entities::newsletter::ConfirmSubscriptionMailPayload,
        errors::newsletter::NewsletterError,
    > {
        self.subscribe(cmd).await
    }
    async fn confirm_subscription(
        &self,
        cmd: commands::newsletter::ConfirmSubscriptionCommand,
    ) -> Result<(), errors::newsletter::NewsletterError> {
        self.confirm_subscription(cmd).await
    }
    async fn unsubscribe(
        &self,
        cmd: commands::newsletter::UnsubscribeCommand,
    ) -> Result<bool, errors::newsletter::NewsletterError> {
        self.unsubscribe(cmd).await
    }
    async fn unsubscribe_by_email(
        &self,
        cmd: commands::newsletter::UnsubscribeByEmailCommand,
    ) -> Result<bool, errors::newsletter::NewsletterError> {
        self.unsubscribe_by_email(cmd).await
    }
    async fn list_subscribers(
        &self,
        cmd: commands::newsletter::ListSubscribersCommand,
    ) -> Result<Vec<entities::newsletter::SubscriberSnapshot>, errors::newsletter::NewsletterError>
    {
        self.list_subscribers(cmd).await
    }
    async fn list_campaigns(
        &self,
        cmd: commands::newsletter::ListSubscribersCommand,
    ) -> Result<Vec<entities::newsletter::CampaignSnapshot>, errors::newsletter::NewsletterError>
    {
        self.list_campaigns(cmd).await
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
    ) -> Result<(), errors::newsletter::NewsletterError> {
        self.send_campaign_for_post(
            post_id,
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
        cmd: commands::newsletter::SendCampaignCommand,
        mail_config: MailConfig,
        app_base_url: String,
    ) -> Result<(), errors::newsletter::NewsletterError> {
        self.send_manual_campaign(cmd, mail_config, app_base_url)
            .await
    }
}
