// Media upload via multipart.
use std::sync::Arc;

use axum::{
    Extension,
    body::Bytes,
    extract::{Multipart, State},
};

use crate::{
    application::{commands::media::UploadMediumCommand, services::media::MediaService},
    domain::{entities::secret::Claims, errors::media::MediaError},
    infrastructure::web::{
        api::handlers::support::cover::{MediumData, extract_medium},
        server::AppState,
    },
};

#[axum::debug_handler]
pub async fn upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    mut multipart: Multipart,
) -> Result<(), MediaError> {
    let mut opt_short_name: Option<String> = None;
    let mut opt_description: Option<String> = None;
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

        match field_name {
            "short_name" => {
                opt_short_name = Some(field.text().await.map_err(|_| {
                    MediaError::UploadFailed("Cannot read short name.".to_string())
                })?);
            }
            "description" => {
                opt_description = Some(field.text().await.map_err(|_| {
                    MediaError::UploadFailed("Cannot read description.".to_string())
                })?);
            }
            "file" => {
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
            _ => {
                return Err(MediaError::UploadFailed(
                    "Unknown field detected.".to_string(),
                ));
            }
        };
    }

    let short_name =
        opt_short_name.ok_or_else(|| MediaError::UploadFailed("Missing short_name".to_string()))?;
    let description = opt_description
        .ok_or_else(|| MediaError::UploadFailed("Missing description".to_string()))?;
    let file_name =
        opt_filename.ok_or_else(|| MediaError::UploadFailed("Missing file".to_string()))?;
    let content_type = opt_content_type
        .ok_or_else(|| MediaError::UploadFailed("Missing content type".to_string()))?;
    let bytes =
        opt_bytes.ok_or_else(|| MediaError::UploadFailed("Missing file bytes".to_string()))?;

    let uploader_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| MediaError::InternalError("Cannot parse id".to_string()))?;

    let cmd = UploadMediumCommand {
        uploader_id,
        short_name,
        description,
        file_name,
        content_type,
        bytes,
    };

    match state.media_service.upload(cmd, &state.media_config).await {
        Ok(()) => Ok(()),
        Err(e) => Err(e),
    }
}
