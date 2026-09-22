use crate::domain::entities::mail::ContactFormCredential;
use crate::infrastructure::web::server::{MailConfig, MailTransportConfig};

use super::super::brevo::{BrevoAddress, BrevoEmailPayload, send_brevo_email};
use super::super::smtp::send_contact_emails_via_smtp;
use super::super::templates;

pub async fn send_contact_emails(
    mail_config: &MailConfig,
    app_base_url: &str,
    contact_form: &ContactFormCredential,
) -> Result<(), String> {
    match &mail_config.transport {
        MailTransportConfig::BrevoApi { api_key } => {
            let (confirmation_text, confirmation_html) =
                templates::contact_confirmation(app_base_url, &contact_form.name);
            let notification_text = format!(
                "Name: {}\nEmail: {}\n\nMessage:\n{}",
                contact_form.name, contact_form.email, contact_form.content
            );

            send_brevo_email(
                api_key,
                &BrevoEmailPayload {
                    sender: BrevoAddress {
                        email: &mail_config.from,
                        name: None,
                    },
                    to: vec![BrevoAddress {
                        email: &contact_form.email,
                        name: Some(&contact_form.name),
                    }],
                    reply_to: None,
                    subject: "Thanks for reaching out",
                    text_content: &confirmation_text,
                    html_content: Some(&confirmation_html),
                    headers: None,
                },
            )
            .await?;

            send_brevo_email(
                api_key,
                &BrevoEmailPayload {
                    sender: BrevoAddress {
                        email: &mail_config.from,
                        name: None,
                    },
                    to: vec![BrevoAddress {
                        email: &mail_config.to,
                        name: None,
                    }],
                    reply_to: Some(BrevoAddress {
                        email: &contact_form.email,
                        name: Some(&contact_form.name),
                    }),
                    subject: "New portfolio contact form submission",
                    text_content: &notification_text,
                    html_content: None,
                    headers: None,
                },
            )
            .await
        }
        MailTransportConfig::Smtp { .. } => {
            send_contact_emails_via_smtp(mail_config, app_base_url, contact_form).await
        }
    }
}
