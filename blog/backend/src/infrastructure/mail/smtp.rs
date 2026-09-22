// SMTP transport: message construction and lettre delivery.
use lettre::{
    Message, SmtpTransport, Transport,
    message::{
        Mailbox, MultiPart, SinglePart,
        header::{HeaderName, HeaderValue},
    },
    transport::smtp::authentication::Credentials,
};

use tokio::task;

use crate::domain::entities::mail::ContactFormCredential;
use crate::infrastructure::web::server::{MailConfig, MailTransportConfig};

use super::templates;

/// Builds a raw (unparsed) email header. Used for `List-Unsubscribe` /
/// `List-Unsubscribe-Post`, which lettre does not model as dedicated header types.
pub(super) fn raw_header(name: &str, value: String) -> HeaderValue {
    HeaderValue::dangerous_new_pre_encoded(
        HeaderName::new_from_ascii(name.to_string()).expect("valid ascii header name"),
        value.clone(),
        value,
    )
}

pub(super) async fn send_html_email_via_smtp(
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
pub(super) async fn send_campaign_email_via_smtp(
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

pub(super) async fn send_contact_emails_via_smtp(
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

pub(super) fn build_smtp_mailer(mail_config: &MailConfig) -> Result<SmtpTransport, String> {
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

pub(super) fn parse_mailbox(value: &str, label: &str) -> Result<Mailbox, String> {
    value
        .parse()
        .map_err(|err| format!("Invalid {label} address: {err}"))
}

pub(super) fn build_message_id(mail_config: &MailConfig) -> String {
    let domain = mail_config
        .from
        .rsplit('@')
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or("localhost");

    format!("<{}@{}>", uuid::Uuid::new_v4(), domain)
}
