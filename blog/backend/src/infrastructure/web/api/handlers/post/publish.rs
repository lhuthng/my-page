// Lifecycle transitions: publish (with the newsletter fan-out to
// subscribers) and toggle the featured flag.
use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, State},
    response::IntoResponse,
};

use crate::{
    application::{
        commands::post::{PublishCommand, SetFeaturedPostCommand},
        services::{newsletter::NewsletterService, post::PostService},
    },
    domain::{entities::secret::Claims, errors::post::PostError},
    infrastructure::web::{api::handlers::post::dto::SetPostFeaturedBody, server::AppState},
};

pub async fn publish(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(post_id_str): Path<String>,
) -> Result<(), PostError> {
    let post_id = post_id_str
        .parse::<i64>()
        .map_err(|_| PostError::PostNotFound)?;

    let claims_user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|e| PostError::InternalError(e.to_string()))?;

    let cmd = PublishCommand {
        user_id: claims_user_id,
        post_id,
    };

    state.post_service.publish(cmd).await?;

    // Fire the newsletter campaign in the background; failures here must
    // never affect the already-succeeded publish response.
    {
        let state = state.clone();
        tokio::spawn(async move {
            let Some(mail_config) = state.config.mail.clone() else {
                return;
            };

            let post_row: Option<(String, String, String, Option<String>)> = match sqlx::query_as(
                r#"SELECT p.title, p.slug, p.excerpt, m.short_name
                   FROM posts p
                   LEFT JOIN media m ON m.id = p.cover_media_id
                   WHERE p.id = ? AND p.content_kind = 'post'"#,
            )
            .bind(post_id)
            .fetch_optional(&state.newsletter_service.pool)
            .await
            {
                Ok(row) => row,
                Err(e) => {
                    tracing::error!(
                        "Failed to load post {} for newsletter campaign: {}",
                        post_id,
                        e
                    );
                    return;
                }
            };

            let Some((title, slug, excerpt, cover_short_name)) = post_row else {
                return;
            };

            let post_url = format!(
                "{}/posts/{}",
                state.config.app_base_url.trim_end_matches('/'),
                slug
            );
            let cover_url = cover_short_name.map(|short| {
                format!(
                    "{}/api/media/i/{}",
                    state.config.app_base_url.trim_end_matches('/'),
                    short
                )
            });
            let body_text =
                crate::infrastructure::mail::campaign_post_text(&title, &excerpt, &post_url);
            let body_html = crate::infrastructure::mail::campaign_post_body(
                &title,
                &excerpt,
                cover_url.as_deref(),
                &post_url,
            );

            if let Err(e) = state
                .newsletter_service
                .send_campaign_for_post(
                    post_id,
                    title,
                    body_html,
                    body_text,
                    claims_user_id,
                    mail_config,
                    state.config.app_base_url.clone(),
                )
                .await
            {
                tracing::error!(
                    "Failed to send newsletter campaign for post {}: {:?}",
                    post_id,
                    e
                );
            }
        });
    }

    Ok(())
}

pub async fn set_post_featured(
    State(state): State<Arc<AppState>>,
    Path(post_id): Path<i64>,
    Json(body): Json<SetPostFeaturedBody>,
) -> Result<impl IntoResponse, PostError> {
    let cmd = SetFeaturedPostCommand {
        post_id,
        is_featured: body.is_featured,
    };
    state.post_service.set_post_featured(cmd).await?;
    Ok(())
}
