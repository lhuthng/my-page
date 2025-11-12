use std::sync::Arc;

use axum::{
    Extension,
    body::Bytes,
    extract::{Multipart, State},
    response::IntoResponse,
};
use serde_json::to_string;

use crate::{
    application::{commands::media::UploadMediaCommand, services::media::MediaService},
    domain::{entities::secret::Claims, errors::media::MediaError},
    infrastructure::web::server::AppState,
};

#[axum::debug_handler]
pub async fn upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    mut multipart: Multipart,
) -> Result<(), MediaError> {
    let mut opt_short_name: Option<String> = None;
    let mut opt_description: Option<String> = None;
    let mut opt_file_name: Option<String> = None;
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
                if opt_file_name.is_some() {
                    return Err(MediaError::UploadFailed(
                        "Only one media is allowed at a time.".to_string(),
                    ));
                }

                let file_name = field
                    .file_name()
                    .ok_or(MediaError::UploadFailed(
                        "Cannot read file name.".to_string(),
                    ))?
                    .to_string();

                let content_type = field
                    .content_type()
                    .ok_or(MediaError::UploadFailed(format!(
                        "Cannot read content type of {}.",
                        file_name
                    )))?
                    .to_string();

                let bytes = field.bytes().await.map_err(|_| {
                    MediaError::UploadFailed(format!("Cannot read bytes of {}.", file_name))
                })?;

                opt_file_name = Some(file_name);
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
        opt_file_name.ok_or_else(|| MediaError::UploadFailed("Missing file".to_string()))?;
    let content_type = opt_content_type
        .ok_or_else(|| MediaError::UploadFailed("Missing content type".to_string()))?;
    let bytes =
        opt_bytes.ok_or_else(|| MediaError::UploadFailed("Missing file bytes".to_string()))?;

    let uploader_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| MediaError::InternalError("Cannot parse id".to_string()))?;

    let cmd = UploadMediaCommand {
        uploader_id,
        short_name,
        description,
        file_name,
        content_type,
        bytes,
    };

    match state.media_service.upload(cmd, &state.media_config).await {
        Ok(_) => {}
        Err(_) => {
            return Err(MediaError::PermissionDenied);
        }
    }

    Err(MediaError::PermissionDenied) // Filler for now
}
