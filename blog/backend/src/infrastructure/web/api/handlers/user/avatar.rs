// Avatar changes: the upload flows through the media aggregate.
use std::sync::Arc;

use axum::{
    Extension,
    body::Bytes,
    extract::{Multipart, State},
    response::IntoResponse,
};

use crate::{
    application::{commands::media::ChangeAvatarCommand, services::media::MediaService},
    domain::{
        entities::{media::MediumDetails, secret::Claims},
        errors::media::MediaError,
    },
    infrastructure::web::{
        api::handlers::support::cover::{MediumData, extract_medium},
        server::AppState,
    },
};

#[axum::debug_handler]
pub async fn change_avatar(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, MediaError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| MediaError::InternalError("Cannot parse id.".to_string()))?;

    let mut opt_filename: Option<String> = None;
    let mut opt_content_type: Option<String> = None;
    let mut opt_bytes: Option<Bytes> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| MediaError::InternalError(e.to_string()))?
    {
        let field_name = field.name().ok_or(MediaError::UploadFailed(
            "Empty field detected.".to_string(),
        ))?;

        if field_name == "file" {
            if opt_filename.is_some() {
                return Err(MediaError::UploadFailed(
                    "Only one media is allowed at a time.".to_string(),
                ));
            }

            let MediumData {
                filename,
                content_type,
                bytes,
            } = extract_medium(field).await?;

            opt_filename = Some(filename);
            opt_content_type = Some(content_type);
            opt_bytes = Some(bytes);
        }
    }

    let filename =
        opt_filename.ok_or_else(|| MediaError::UploadFailed("Missing file".to_string()))?;
    let content_type = opt_content_type
        .ok_or_else(|| MediaError::UploadFailed("Missing content type".to_string()))?;
    let bytes =
        opt_bytes.ok_or_else(|| MediaError::UploadFailed("Missing file bytes".to_string()))?;

    state
        .media_service
        .change_avatar(
            ChangeAvatarCommand {
                user_id,
                medium_details: MediumDetails {
                    filename,
                    content_type,
                    bytes,
                },
            },
            &state.media_config,
        )
        .await?;

    Ok(())
}
