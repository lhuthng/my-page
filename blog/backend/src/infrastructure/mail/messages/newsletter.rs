use crate::domain::entities::newsletter::ConfirmSubscriptionMailPayload;
use crate::infrastructure::web::server::{MailConfig, MailTransportConfig};

use super::super::brevo::{BrevoAddress, BrevoEmailPayload, send_brevo_email};
use super::super::smtp::{send_campaign_email_via_smtp, send_html_email_via_smtp};
use super::super::templates;

pub async fn send_subscription_confirm_email(
    mail_config: &MailConfig,
    app_base_url: &str,
    payload: &ConfirmSubscriptionMailPayload,
) -> Result<(), String> {
    let confirm_link = format!(
        "{}/newsletter/confirm?token={}",
        app_base_url.trim_end_matches('/'),
        payload.token
    );
    let (text, html) = templates::subscription_confirm(app_base_url, &confirm_link);

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
                        email: &payload.email,
                        name: None,
                    }],
                    reply_to: None,
                    subject: "Confirm your huuthangle.site newsletter subscription",
                    text_content: &text,
                    html_content: Some(&html),
                    headers: None,
                },
            )
            .await
        }
        MailTransportConfig::Smtp { .. } => {
            send_html_email_via_smtp(
                mail_config,
                &payload.email,
                "Confirm your huuthangle.site newsletter subscription",
                &text,
                &html,
            )
            .await
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn send_campaign_email(
    mail_config: &MailConfig,
    app_base_url: &str,
    recipient_email: &str,
    unsubscribe_token: &str,
    subject: &str,
    body_html: &str,
    body_text: &str,
) -> Result<(), String> {
    let unsubscribe_link = format!(
        "{}/newsletter/unsubscribe?token={}",
        app_base_url.trim_end_matches('/'),
        unsubscribe_token
    );
    let (text, html) = templates::campaign(app_base_url, body_html, body_text, &unsubscribe_link);

    match &mail_config.transport {
        MailTransportConfig::BrevoApi { api_key } => {
            let list_unsubscribe_value = format!("<{}>", unsubscribe_link);
            let mut headers = std::collections::HashMap::new();
            headers.insert("List-Unsubscribe", list_unsubscribe_value.as_str());
            headers.insert("List-Unsubscribe-Post", "List-Unsubscribe=One-Click");

            send_brevo_email(
                api_key,
                &BrevoEmailPayload {
                    sender: BrevoAddress {
                        email: &mail_config.from,
                        name: None,
                    },
                    to: vec![BrevoAddress {
                        email: recipient_email,
                        name: None,
                    }],
                    reply_to: None,
                    subject,
                    text_content: &text,
                    html_content: Some(&html),
                    headers: Some(headers),
                },
            )
            .await
        }
        MailTransportConfig::Smtp { .. } => {
            send_campaign_email_via_smtp(
                mail_config,
                recipient_email,
                &unsubscribe_link,
                subject,
                &text,
                &html,
            )
            .await
        }
    }
}
