use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use tokio::time::{Duration, timeout};
use validator::Validate;

use crate::{
    application::{
        commands::newsletter::{
            ConfirmSubscriptionCommand, ListSubscribersCommand, SendCampaignCommand,
            SubscribeCommand, UnsubscribeByEmailCommand, UnsubscribeCommand,
        },
        services::newsletter::NewsletterService,
    },
    domain::{
        entities::{newsletter::SubscribeRequest, secret::Claims},
        errors::newsletter::{NewsletterError, NewsletterMessageResponse},
    },
    infrastructure::{mail::send_subscription_confirm_email, web::server::AppState},
};

#[derive(Deserialize)]
pub struct SubscribePayload {
    pub email: String,
}

#[axum::debug_handler]
pub async fn subscribe(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SubscribePayload>,
) -> Result<impl IntoResponse, NewsletterError> {
    let req = SubscribeRequest {
        email: payload.email,
    };

    if let Err(err) = req.validate() {
        return Err(NewsletterError::InternalError(err.to_string()));
    }

    let mail_config = state.config.mail.as_ref().ok_or_else(|| {
        NewsletterError::NotConfigured("Mail is not configured on the server".to_string())
    })?;

    let mail_payload = timeout(
        Duration::from_secs(15),
        state.newsletter_service.subscribe(SubscribeCommand {
            email: req.email,
        }),
    )
    .await
    .map_err(|_| {
        NewsletterError::InternalError("Subscribing timed out. Please try again.".to_string())
    })??;

    timeout(
        Duration::from_secs(15),
        send_subscription_confirm_email(
            mail_config,
            &state.config.app_base_url,
            &crate::domain::entities::newsletter::ConfirmSubscriptionMailPayload {
                email: mail_payload.email,
                token: mail_payload.token,
            },
        ),
    )
    .await
    .map_err(|_| {
        NewsletterError::InternalError(
            "Sending confirmation email timed out. Please try again.".to_string(),
        )
    })?
    .map_err(NewsletterError::InternalError)?;

    Ok(Json(NewsletterMessageResponse {
        message: "Check your inbox to confirm your subscription.".to_string(),
    }))
}

#[derive(Deserialize)]
pub struct TokenQuery {
    token: String,
}

#[axum::debug_handler]
pub async fn confirm(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TokenQuery>,
) -> Result<Json<NewsletterMessageResponse>, NewsletterError> {
    state
        .newsletter_service
        .confirm_subscription(ConfirmSubscriptionCommand { token: query.token })
        .await?;

    Ok(Json(NewsletterMessageResponse {
        message: "Subscription confirmed. Thanks for joining!".to_string(),
    }))
}

#[axum::debug_handler]
pub async fn unsubscribe(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TokenQuery>,
) -> Result<Json<NewsletterMessageResponse>, NewsletterError> {
    // Idempotent: an invalid or already-consumed token reports the friendly
    // already-unsubscribed state instead of an error.
    let unsubscribed = state
        .newsletter_service
        .unsubscribe(UnsubscribeCommand { token: query.token })
        .await?;

    let message = if unsubscribed {
        "You have been unsubscribed."
    } else {
        "You're already unsubscribed."
    };

    Ok(Json(NewsletterMessageResponse {
        message: message.to_string(),
    }))
}

#[derive(serde::Deserialize, validator::Validate)]
pub struct UnsubscribeByEmailPayload {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,
}

#[axum::debug_handler]
pub async fn unsubscribe_by_email(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UnsubscribeByEmailPayload>,
) -> Result<Json<NewsletterMessageResponse>, NewsletterError> {
    if let Err(err) = payload.validate() {
        return Err(NewsletterError::InternalError(err.to_string()));
    }

    let unsubscribed = state
        .newsletter_service
        .unsubscribe_by_email(UnsubscribeByEmailCommand {
            email: payload.email,
        })
        .await?;

    let message = if unsubscribed {
        "You have been unsubscribed."
    } else {
        "No active subscription found for that email."
    };

    Ok(Json(NewsletterMessageResponse {
        message: message.to_string(),
    }))
}

#[axum::debug_handler]
pub async fn list_subscribers(
    State(state): State<Arc<AppState>>,
    axum::Extension(claims): axum::Extension<Claims>,
) -> Result<impl IntoResponse, NewsletterError> {
    let is_admin = claims.role == "admin" || claims.role == "mod";
    let result = state
        .newsletter_service
        .list_subscribers(ListSubscribersCommand { is_admin })
        .await?;

    Ok(Json(result))
}

#[axum::debug_handler]
pub async fn list_campaigns(
    State(state): State<Arc<AppState>>,
    axum::Extension(claims): axum::Extension<Claims>,
) -> Result<impl IntoResponse, NewsletterError> {
    let is_admin = claims.role == "admin" || claims.role == "mod";
    let result = state
        .newsletter_service
        .list_campaigns(ListSubscribersCommand { is_admin })
        .await?;

    Ok(Json(result))
}

#[derive(Deserialize)]
pub struct SendCampaignPayload {
    pub subject: String,
    pub body_html: String,
    pub body_text: String,
    pub post_id: Option<i64>,
}

#[axum::debug_handler]
pub async fn send_campaign(
    State(state): State<Arc<AppState>>,
    axum::Extension(claims): axum::Extension<Claims>,
    Json(payload): Json<SendCampaignPayload>,
) -> Result<Json<NewsletterMessageResponse>, NewsletterError> {
    let sent_by_user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|e| NewsletterError::InternalError(e.to_string()))?;

    let mail_config = state
        .config
        .mail
        .as_ref()
        .ok_or_else(|| {
            NewsletterError::NotConfigured("Mail is not configured on the server".to_string())
        })?
        .clone();

    state
        .newsletter_service
        .send_manual_campaign(
            SendCampaignCommand {
                post_id: payload.post_id,
                subject: payload.subject,
                body_html: payload.body_html,
                body_text: payload.body_text,
                sent_by_user_id,
            },
            mail_config,
            state.config.app_base_url.clone(),
        )
        .await?;

    Ok(Json(NewsletterMessageResponse {
        message: "Campaign sent.".to_string(),
    }))
}
