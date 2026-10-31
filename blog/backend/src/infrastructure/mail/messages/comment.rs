/// Notifies a post's author that someone commented on their post.
use crate::infrastructure::web::server::{MailConfig, MailTransportConfig};

use super::super::brevo::{BrevoAddress, BrevoEmailPayload, send_brevo_email};
use super::super::smtp::send_html_email_via_smtp;

pub async fn send_comment_notification_email(
    mail_config: &MailConfig,
    recipient_email: &str,
    recipient_name: Option<&str>,
    subject: &str,
    text: &str,
    html: &str,
) -> Result<(), String> {
    match &mail_config.transport {
        MailTransportConfig::BrevoApi { api_key } => {
            send_brevo_email(
                api_key,
                &BrevoEmailPayload {
                    sender: BrevoAddress {
                        email: &mail_config.from,
                        name: None,
                    },
                    to: vec![BrevoAddress {
                        email: recipient_email,
                        name: recipient_name,
                    }],
                    reply_to: None,
                    subject,
                    text_content: text,
                    html_content: Some(html),
                    headers: None,
                },
            )
            .await
        }
        MailTransportConfig::Smtp { .. } => {
            send_html_email_via_smtp(mail_config, recipient_email, subject, text, html).await
        }
    }
}
