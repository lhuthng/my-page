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

#[cfg(debug_assertions)]
#[axum::debug_handler]
pub async fn preview_email_templates(
    State(state): State<Arc<AppState>>,
) -> axum::response::Html<String> {
    let app_base_url = state.config.app_base_url.trim_end_matches('/').to_string();

    // Pick a random published post so the campaign sample shows real content.
    let sample = {
        let row: Option<(String, String, Option<String>, String)> = sqlx::query_as(
            r#"
            SELECT p.title, p.excerpt, m.short_name, p.slug
            FROM posts p
            LEFT JOIN media m ON m.id = p.cover_media_id
            WHERE p.status = 'published' AND p.content_kind = 'post'
            ORDER BY RANDOM() LIMIT 1
            "#,
        )
        .fetch_optional(&state.newsletter_service.pool)
        .await
        .ok()
        .flatten();

        row.map(|(title, excerpt, short_name, slug)| {
            crate::infrastructure::mail::CampaignPostData {
                title,
                excerpt,
                cover_url: short_name
                    .map(|short| format!("{app_base_url}/api/media/i/{short}")),
                post_url: format!("{app_base_url}/posts/{slug}"),
            }
        })
    };

    axum::response::Html(crate::infrastructure::mail::preview_page(
        &app_base_url,
        sample.as_ref(),
    ))
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
        send_contact_emails(mail_config, &state.config.app_base_url, &cred),
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
