use crate::domain::entities::auth::VerificationMailPayload;
use crate::infrastructure::web::server::{MailConfig, MailTransportConfig};

use super::super::brevo::{BrevoAddress, BrevoEmailPayload, send_brevo_email};
use super::super::smtp::send_html_email_via_smtp;
use super::super::templates;

pub async fn send_verification_email(
    mail_config: &MailConfig,
    app_base_url: &str,
    payload: &VerificationMailPayload,
) -> Result<(), String> {
    let verification_link = format!(
        "{}/verify-email?token={}",
        app_base_url.trim_end_matches('/'),
        payload.token
    );
    let (text, html) = templates::verification(app_base_url, &payload.username, &verification_link);

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
                        name: Some(&payload.username),
                    }],
                    reply_to: None,
                    subject: "Verify your huuthangle.site account",
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
                "Verify your huuthangle.site account",
                &text,
                &html,
            )
            .await
        }
    }
}
