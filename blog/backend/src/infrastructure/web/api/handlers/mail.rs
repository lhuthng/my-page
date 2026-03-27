use std::sync::Arc;

use axum::{Json, response::IntoResponse};
use lettre::{Message, SmtpTransport, Transport, transport::smtp::client::Tls};
use serde::Deserialize;
use validator::Validate;

use crate::domain::{entities::mail::ContactFormCredential, errors::mail::MailError};

#[derive(Deserialize)]
pub struct ReceiveContactForm {
    pub name: String,
    pub email: String,
    pub content: String,
}

#[axum::debug_handler]
pub async fn receive_contact_form(
    Json(payload): Json<ReceiveContactForm>,
) -> Result<impl IntoResponse, MailError> {
    let ReceiveContactForm {
        name,
        email,
        content,
    } = payload;

    let cred = ContactFormCredential {
        name,
        email,
        content,
    };

    if let Err(err) = cred.validate() {
        return Err(MailError::UploadFailed(err.to_string()));
    }
    let ContactFormCredential {
        mut name,
        email,
        content,
    } = cred;

    if name == "portfolio" {
        name = "there".to_string();
    }

    // 2. Prepare emails
    //
    let message_id = format!("<{}@huuthangle.site>", uuid::Uuid::new_v4());
    let confirmation_email = Message::builder()
        .from("Thắng <contact@huuthangle.site>".parse().unwrap())
        .to(email.parse().unwrap())
        .subject("Thanks for contacting !")
        .message_id(Some(message_id))
        .body(format!(
            "Hi {},\n\nThanks for your message!\n\n- huuthangle.site",
            name
        ))
        .unwrap();

    let message_id = format!("<{}@huuthangle.site>", uuid::Uuid::new_v4());
    let notification_email = Message::builder()
        .from("System <contact@huuthangle.site>".parse().unwrap())
        .to("huuthang.l@outlook.com".parse().unwrap())
        .subject("New Contact Form Submission")
        .message_id(Some(message_id))
        .body(format!(
            "Name: {}\nEmail: {}\nContent:\n{}",
            name, email, content
        ))
        .unwrap();
    let mailer = SmtpTransport::relay("host.docker.internal")
        .unwrap()
        .port(25) // Standard Postfix port
        .timeout(Some(std::time::Duration::from_secs(5)))
        .tls(Tls::None)
        .build();
    // 4. Send emails
    mailer
        .send(&confirmation_email)
        .map_err(|e| e.to_string())?;
    mailer
        .send(&notification_email)
        .map_err(|e| e.to_string())?;
    Ok("Emails sent successfully")
}
