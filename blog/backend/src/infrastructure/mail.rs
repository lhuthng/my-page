use lettre::{
    Message, SmtpTransport, Transport, message::Mailbox,
    transport::smtp::authentication::Credentials,
};
use reqwest::Client;
use serde::Serialize;
use tokio::task;

use crate::{
    domain::entities::{
        auth::{PasswordResetMailPayload, VerificationMailPayload},
        mail::ContactFormCredential,
    },
    infrastructure::web::server::{MailConfig, MailTransportConfig},
};

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
}

pub async fn send_contact_emails(
    mail_config: &MailConfig,
    contact_form: &ContactFormCredential,
) -> Result<(), String> {
    match &mail_config.transport {
        MailTransportConfig::BrevoApi { api_key } => {
            let confirmation_body = format!(
                "Hi {},\n\nThanks for your message. I received it and will get back to you soon.\n\n- huuthangle.site",
                contact_form.name
            );
            let notification_body = format!(
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
                    text_content: &confirmation_body,
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
                    text_content: &notification_body,
                },
            )
            .await
        }
        MailTransportConfig::Smtp { .. } => {
            send_contact_emails_via_smtp(mail_config, contact_form).await
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
    let body = format!(
        "Hi {},\n\nPlease verify your account by opening the link below within 30 minutes:\n\n{}\n\nIf you did not create this account, you can ignore this email.\n",
        payload.username, verification_link
    );

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
                    text_content: &body,
                },
            )
            .await
        }
        MailTransportConfig::Smtp { .. } => {
            send_verification_email_via_smtp(mail_config, payload, &body).await
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
    let body = format!(
        "Hi {},\n\nUse the link below within 30 minutes to set a new password:\n\n{}\n\nIf you did not request this change, you can ignore this email.\n",
        payload.username, reset_link
    );

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
                    text_content: &body,
                },
            )
            .await
        }
        MailTransportConfig::Smtp { .. } => {
            send_password_reset_email_via_smtp(mail_config, payload, &body).await
        }
    }
}

async fn send_contact_emails_via_smtp(
    mail_config: &MailConfig,
    contact_form: &ContactFormCredential,
) -> Result<(), String> {
    let mail_config = mail_config.clone();
    let contact_form = contact_form.clone();

    task::spawn_blocking(move || {
        let sender = parse_mailbox(&mail_config.from, "SMTP_FROM")?;
        let recipient = parse_mailbox(&contact_form.email, "contact email")?;
        let admin_recipient = parse_mailbox(&mail_config.to, "SMTP_TO")?;
        let reply_to = parse_mailbox(&contact_form.email, "reply-to email")?;

        let confirmation_email = Message::builder()
            .from(sender.clone())
            .to(recipient)
            .subject("Thanks for reaching out")
            .message_id(Some(build_message_id(&mail_config)))
            .body(format!(
                "Hi {},\n\nThanks for your message. I received it and will get back to you soon.\n\n- huuthangle.site",
                contact_form.name
            ))
            .map_err(|err| format!("Failed to build confirmation email: {err}"))?;

        let notification_email = Message::builder()
            .from(sender)
            .to(admin_recipient)
            .reply_to(reply_to)
            .subject("New portfolio contact form submission")
            .message_id(Some(build_message_id(&mail_config)))
            .body(format!(
                "Name: {}\nEmail: {}\n\nMessage:\n{}",
                contact_form.name, contact_form.email, contact_form.content
            ))
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

async fn send_verification_email_via_smtp(
    mail_config: &MailConfig,
    payload: &VerificationMailPayload,
    body: &str,
) -> Result<(), String> {
    let mail_config = mail_config.clone();
    let payload = payload.clone();
    let body = body.to_string();

    task::spawn_blocking(move || {
        let sender = parse_mailbox(&mail_config.from, "SMTP_FROM")?;
        let recipient = parse_mailbox(&payload.email, "verification email")?;

        let email = Message::builder()
            .from(sender)
            .to(recipient)
            .subject("Verify your huuthangle.site account")
            .message_id(Some(build_message_id(&mail_config)))
            .body(body)
            .map_err(|err| format!("Failed to build verification email: {err}"))?;

        build_smtp_mailer(&mail_config)?
            .send(&email)
            .map_err(|err| format!("Failed to send verification email: {err}"))?;

        Ok(())
    })
    .await
    .map_err(|err| format!("SMTP worker failed: {err}"))?
}

async fn send_password_reset_email_via_smtp(
    mail_config: &MailConfig,
    payload: &PasswordResetMailPayload,
    body: &str,
) -> Result<(), String> {
    let mail_config = mail_config.clone();
    let payload = payload.clone();
    let body = body.to_string();

    task::spawn_blocking(move || {
        let sender = parse_mailbox(&mail_config.from, "SMTP_FROM")?;
        let recipient = parse_mailbox(&payload.email, "password reset email")?;

        let email = Message::builder()
            .from(sender)
            .to(recipient)
            .subject("Reset your huuthangle.site password")
            .message_id(Some(build_message_id(&mail_config)))
            .body(body)
            .map_err(|err| format!("Failed to build password reset email: {err}"))?;

        build_smtp_mailer(&mail_config)?
            .send(&email)
            .map_err(|err| format!("Failed to send password reset email: {err}"))?;

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
