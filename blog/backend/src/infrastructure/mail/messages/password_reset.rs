use lettre::message::header::HeaderValue;

use crate::domain::entities::{
    auth::{PasswordResetMailPayload, VerificationMailPayload},
    mail::ContactFormCredential,
    newsletter::ConfirmSubscriptionMailPayload,
};
use crate::infrastructure::web::server::{MailConfig, MailTransportConfig};

use super::super::brevo::{BrevoAddress, BrevoEmailPayload, send_brevo_email};
use super::super::smtp::send_html_email_via_smtp;
use super::super::templates;

pub async fn send_password_reset_email(
    mail_config: &MailConfig,
    app_base_url: &str,
    payload: &PasswordResetMailPayload,
) -> Result<(), String> {
    let reset_link = format!(
        "{}/reset-password?token={}",
        app_base_url.trim_end_matches('/'),
        payload.token
    );
    let (text, html) = templates::password_reset(app_base_url, &payload.username, &reset_link);

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
                    subject: "Reset your huuthangle.site password",
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
                "Reset your huuthangle.site password",
                &text,
                &html,
            )
            .await
        }
    }
}

