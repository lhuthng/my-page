use crate::{
    application::commands,
    domain::{entities, errors},
    infrastructure::web::server::MailConfig,
};

#[async_trait::async_trait]
pub trait NewsletterService {
    async fn subscribe(
        &self,
        cmd: commands::newsletter::SubscribeCommand,
    ) -> Result<entities::newsletter::ConfirmSubscriptionMailPayload, errors::newsletter::NewsletterError>;

    async fn confirm_subscription(
        &self,
        cmd: commands::newsletter::ConfirmSubscriptionCommand,
    ) -> Result<(), errors::newsletter::NewsletterError>;

    /// Returns `true` when the subscription was actually removed and `false`
    /// when it was already unsubscribed (or the token was invalid), so callers
    /// can tailor the message. Never returns an error for an invalid token.
    async fn unsubscribe(
        &self,
        cmd: commands::newsletter::UnsubscribeCommand,
    ) -> Result<bool, errors::newsletter::NewsletterError>;

    /// Email-based unsubscribe (no token required). Returns `true` when the
    /// subscription was actually removed, `false` when there was no active
    /// subscription for that email. Never returns an error for a missing email.
    async fn unsubscribe_by_email(
        &self,
        cmd: commands::newsletter::UnsubscribeByEmailCommand,
    ) -> Result<bool, errors::newsletter::NewsletterError>;

    async fn list_subscribers(
        &self,
        cmd: commands::newsletter::ListSubscribersCommand,
    ) -> Result<Vec<entities::newsletter::SubscriberSnapshot>, errors::newsletter::NewsletterError>;

    async fn list_campaigns(
        &self,
        cmd: commands::newsletter::ListSubscribersCommand,
    ) -> Result<Vec<entities::newsletter::CampaignSnapshot>, errors::newsletter::NewsletterError>;

    async fn send_campaign_for_post(
        &self,
        post_id: i64,
        subject: String,
        body_html: String,
        body_text: String,
        sent_by_user_id: i64,
        mail_config: MailConfig,
        app_base_url: String,
    ) -> Result<(), errors::newsletter::NewsletterError>;

    async fn send_manual_campaign(
        &self,
        cmd: commands::newsletter::SendCampaignCommand,
        mail_config: MailConfig,
        app_base_url: String,
    ) -> Result<(), errors::newsletter::NewsletterError>;
}
