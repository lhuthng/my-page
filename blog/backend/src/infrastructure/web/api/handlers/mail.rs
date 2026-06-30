use std::sync::Arc;

use axum::{Json, extract::State, response::IntoResponse};
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, timeout};
use validator::Validate;

use crate::{
    domain::{entities::mail::ContactFormCredential, errors::mail::MailError},
    infrastructure::{mail::send_contact_emails, web::server::AppState},
};

#[derive(Deserialize)]
pub struct ReceiveContactForm {
    pub name: String,
    pub email: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct ContactFormResponse {
    message: String,
}

#[axum::debug_handler]
pub async fn receive_contact_form(
    State(state): State<Arc<AppState>>,
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
    let mail_config = state.config.mail.as_ref().ok_or_else(|| {
        MailError::InternalError("SMTP is not configured on the server".to_string())
    })?;

    let send_result = timeout(
        Duration::from_secs(15),
        send_contact_emails(mail_config, &cred),
    )
    .await
    .map_err(|_| {
        MailError::InternalError(
            "Sending email timed out. Please try again in a moment.".to_string(),
        )
    })?;

    send_result.map_err(MailError::InternalError)?;

    Ok(Json(ContactFormResponse {
        message: "Message sent. Check your inbox for the confirmation email.".to_string(),
    }))
}
