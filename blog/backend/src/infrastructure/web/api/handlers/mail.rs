use std::sync::Arc;

use axum::{Json, extract::State, response::IntoResponse};
use lettre::{Message, SmtpTransport, Transport, transport::smtp::authentication::Credentials};
use serde::Deserialize;
use validator::Validate;

use crate::{
    domain::{entities::mail::ContactFormCredential, errors::mail::MailError},
    infrastructure::web::server::AppState,
};

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
        name,
        email,
        content,
    } = cred;

    // 2. Prepare emails
    let confirmation_email = Message::builder()
        .from("Thắng <contact@huuthang.site>".parse().unwrap())
        .to(email.parse().unwrap())
        .subject("Thanks for contacting !")
        .body(format!(
            "Hi {},\n\nThanks for your message!\n\n- huuthang.site",
            name
        ))
        .unwrap();
    let notification_email = Message::builder()
        .from("System <contact@huuthang.site>".parse().unwrap())
        .to("huuthang.l@outlook.com".parse().unwrap())
        .subject("New Contact Form Submission")
        .body(format!(
            "Name: {}\nEmail: {}\nContent:\n{}",
            name, email, content
        ))
        .unwrap();
    let mailer = SmtpTransport::relay("host.docker.internal")
        .unwrap()
        .port(25) // Standard Postfix port
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
