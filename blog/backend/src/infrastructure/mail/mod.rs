use lettre::{
    Message, SmtpTransport, Transport,
    message::{
        Mailbox, MultiPart, SinglePart,
        header::{HeaderName, HeaderValue},
    },
    transport::smtp::authentication::Credentials,
};
use reqwest::Client;
use serde::Serialize;
use tokio::task;

use crate::{
    domain::entities::{
        auth::{PasswordResetMailPayload, VerificationMailPayload},
        mail::ContactFormCredential,
        newsletter::ConfirmSubscriptionMailPayload,
    },
    infrastructure::web::server::{MailConfig, MailTransportConfig},
};

mod templates;

pub use templates::{campaign_post_body, campaign_post_text};
#[cfg(debug_assertions)]
pub use templates::{CampaignPostData, preview_page};

/// Builds a raw (unparsed) email header. Used for `List-Unsubscribe` /
/// `List-Unsubscribe-Post`, which lettre does not model as dedicated header types.
fn raw_header(name: &str, value: String) -> HeaderValue {
    HeaderValue::dangerous_new_pre_encoded(
        HeaderName::new_from_ascii(name.to_string()).expect("valid ascii header name"),
        value.clone(),
        value,
    )
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BrevoAddress<'a> {
    email: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BrevoEmailPayload<'a> {
    sender: BrevoAddress<'a>,
    to: Vec<BrevoAddress<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<BrevoAddress<'a>>,
    subject: &'a str,
    text_content: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    html_content: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    headers: Option<std::collections::HashMap<&'a str, &'a str>>,
}

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

/// Sends a plain + HTML multipart email over SMTP.
async fn send_html_email_via_smtp(
    mail_config: &MailConfig,
    recipient_email: &str,
    subject: &str,
    text: &str,
    html: &str,
) -> Result<(), String> {
    let mail_config = mail_config.clone();
    let recipient_email = recipient_email.to_string();
    let subject = subject.to_string();
    let text = text.to_string();
    let html = html.to_string();

    task::spawn_blocking(move || {
        let sender = parse_mailbox(&mail_config.from, "SMTP_FROM")?;
        let recipient = parse_mailbox(&recipient_email, "SMTP recipient email")?;

        let email = Message::builder()
            .from(sender)
            .to(recipient)
            .subject(subject)
            .message_id(Some(build_message_id(&mail_config)))
            .multipart(
                MultiPart::alternative()
                    .singlepart(SinglePart::plain(text))
                    .singlepart(SinglePart::html(html)),
            )
            .map_err(|err| format!("Failed to build email: {err}"))?;

        build_smtp_mailer(&mail_config)?
            .send(&email)
            .map_err(|err| format!("Failed to send email: {err}"))?;

        Ok(())
    })
    .await
    .map_err(|err| format!("SMTP worker failed: {err}"))?
}

#[allow(clippy::too_many_arguments)]
async fn send_campaign_email_via_smtp(
    mail_config: &MailConfig,
    recipient_email: &str,
    unsubscribe_link: &str,
    subject: &str,
    text: &str,
    html: &str,
) -> Result<(), String> {
    let mail_config = mail_config.clone();
    let recipient_email = recipient_email.to_string();
    let unsubscribe_link = unsubscribe_link.to_string();
    let subject = subject.to_string();
    let text = text.to_string();
    let html = html.to_string();

    task::spawn_blocking(move || {
        let sender = parse_mailbox(&mail_config.from, "SMTP_FROM")?;
        let recipient = parse_mailbox(&recipient_email, "campaign recipient email")?;

        let email = Message::builder()
            .from(sender)
            .to(recipient)
            .subject(subject)
            .message_id(Some(build_message_id(&mail_config)))
            .raw_header(raw_header(
                "List-Unsubscribe",
                format!("<{}>", unsubscribe_link),
            ))
            .raw_header(raw_header(
                "List-Unsubscribe-Post",
                "List-Unsubscribe=One-Click".to_string(),
            ))
            .multipart(
                MultiPart::alternative()
                    .singlepart(SinglePart::plain(text))
                    .singlepart(SinglePart::html(html)),
            )
            .map_err(|err| format!("Failed to build campaign email: {err}"))?;

        build_smtp_mailer(&mail_config)?
            .send(&email)
            .map_err(|err| format!("Failed to send campaign email: {err}"))?;

        Ok(())
    })
    .await
    .map_err(|err| format!("SMTP worker failed: {err}"))?
}

async fn send_contact_emails_via_smtp(
    mail_config: &MailConfig,
    app_base_url: &str,
    contact_form: &ContactFormCredential,
) -> Result<(), String> {
    let mail_config = mail_config.clone();
    let app_base_url = app_base_url.to_string();
    let contact_form = contact_form.clone();

    task::spawn_blocking(move || {
        let sender = parse_mailbox(&mail_config.from, "SMTP_FROM")?;
        let recipient = parse_mailbox(&contact_form.email, "contact email")?;
        let admin_recipient = parse_mailbox(&mail_config.to, "SMTP_TO")?;
        let reply_to = parse_mailbox(&contact_form.email, "reply-to email")?;

        let (confirmation_text, confirmation_html) =
            templates::contact_confirmation(&app_base_url, &contact_form.name);
        let notification_text = format!(
            "Name: {}\nEmail: {}\n\nMessage:\n{}",
            contact_form.name, contact_form.email, contact_form.content
        );

        let confirmation_email = Message::builder()
            .from(sender.clone())
            .to(recipient)
            .subject("Thanks for reaching out")
            .message_id(Some(build_message_id(&mail_config)))
            .multipart(
                MultiPart::alternative()
                    .singlepart(SinglePart::plain(confirmation_text))
                    .singlepart(SinglePart::html(confirmation_html)),
            )
            .map_err(|err| format!("Failed to build confirmation email: {err}"))?;

        let notification_email = Message::builder()
            .from(sender)
            .to(admin_recipient)
            .reply_to(reply_to)
            .subject("New portfolio contact form submission")
            .message_id(Some(build_message_id(&mail_config)))
            .body(notification_text)
            .map_err(|err| format!("Failed to build notification email: {err}"))?;

        let mailer = build_smtp_mailer(&mail_config)?;

        mailer
            .send(&confirmation_email)
            .map_err(|err| format!("Failed to send confirmation email: {err}"))?;
        mailer
            .send(&notification_email)
            .map_err(|err| format!("Failed to send notification email: {err}"))?;

        Ok(())
    })
    .await
    .map_err(|err| format!("SMTP worker failed: {err}"))?
}

async fn send_brevo_email(api_key: &str, payload: &BrevoEmailPayload<'_>) -> Result<(), String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|err| format!("Failed to build Brevo client: {err}"))?;

    let response = client
        .post("https://api.brevo.com/v3/smtp/email")
        .header("api-key", api_key)
        .header("accept", "application/json")
        .json(payload)
        .send()
        .await
        .map_err(|err| format!("Brevo API request failed: {err}"))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unable to read Brevo error response".to_string());
        Err(format!("Brevo API returned {status}: {body}"))
    }
}

fn build_smtp_mailer(mail_config: &MailConfig) -> Result<SmtpTransport, String> {
    let MailTransportConfig::Smtp {
        host,
        port,
        username,
        password,
    } = &mail_config.transport
    else {
        return Err("Mail transport is not configured for SMTP".to_string());
    };

    SmtpTransport::relay(host)
        .map_err(|err| format!("Invalid SMTP relay host: {err}"))
        .map(|builder| {
            builder
                .credentials(Credentials::new(username.clone(), password.clone()))
                .port(*port)
                .timeout(Some(std::time::Duration::from_secs(10)))
                .build()
        })
}

fn parse_mailbox(value: &str, label: &str) -> Result<Mailbox, String> {
    value
        .parse()
        .map_err(|err| format!("Invalid {label} address: {err}"))
}

fn build_message_id(mail_config: &MailConfig) -> String {
    let domain = mail_config
        .from
        .rsplit('@')
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or("localhost");

    format!("<{}@{}>", uuid::Uuid::new_v4(), domain)
}