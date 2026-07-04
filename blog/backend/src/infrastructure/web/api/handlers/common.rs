use axum::{body::Bytes, extract::multipart::Field};

use crate::{
    application::commands::{
        media::ChangePostCoverCommand,
        post::UpdatePostCoverCommand,
    },
    application::services::{media::MediaService, post::PostService},
    domain::{
        entities::media::MediumDetails,
        errors::{media::MediaError, post::PostError},
    },
    infrastructure::web::server::AppState,
};

pub struct MediumData {
    pub filename: String,
    pub content_type: String,
    pub bytes: Bytes,
}

#[derive(Default)]
pub struct CreateCoverUpload {
    pub medium: Option<MediumData>,
    pub og_image_seconds: Option<i64>,
}

pub async fn extract_medium(field: Field<'_>) -> Result<MediumData, MediaError> {
    let filename = field
        .file_name()
        .ok_or(MediaError::UploadFailed(
            "Cannot read file name.".to_string(),
        ))?
        .to_string();

    let content_type = field
        .content_type()
        .ok_or(MediaError::UploadFailed(format!(
            "Cannot read content type of {}.",
            filename
        )))?
        .to_string();

    let bytes = field
        .bytes()
        .await
        .map_err(|_| MediaError::UploadFailed(format!("Cannot read bytes of {}.", filename)))?;

    Ok(MediumData {
        filename,
        content_type,
        bytes,
    })
}

pub async fn try_collect_create_cover_field(
    field_name: &str,
    field: Field<'_>,
    create_cover: &mut CreateCoverUpload,
) -> Result<bool, MediaError> {
    match field_name {
        "cover_file" => {
            if create_cover.medium.is_some() {
                return Err(MediaError::UploadFailed(
                    "Only one cover file is allowed.".to_string(),
                ));
            }
            create_cover.medium = Some(extract_medium(field).await?);
            Ok(true)
        }
        "cover_og_image_seconds" => {
            let text = field.text().await.map_err(|e| {
                MediaError::UploadFailed(format!("Failed to read cover_og_image_seconds: {}", e))
            })?;
            create_cover.og_image_seconds = text.trim().parse::<i64>().ok();
            Ok(true)
        }
        _ => Ok(false),
    }
}

pub async fn apply_created_cover_upload(
    state: &AppState,
    user_id: i64,
    post_id: i64,
    create_cover: CreateCoverUpload,
) -> Result<(), PostError> {
    let has_cover_medium = create_cover.medium.is_some();

    if let Some(cover_medium) = create_cover.medium {
        state
            .media_service
            .change_post_cover(
                ChangePostCoverCommand {
                    post_id,
                    user_id,
                    medium_details: MediumDetails {
                        filename: cover_medium.filename,
                        content_type: cover_medium.content_type,
                        bytes: cover_medium.bytes,
                    },
                    og_image_seconds: create_cover.og_image_seconds,
                },
                &state.media_config,
            )
            .await?;
    }

    if has_cover_medium && create_cover.og_image_seconds.is_some() {
        state
            .post_service
            .update_post_cover(UpdatePostCoverCommand {
                user_id,
                post_id,
                og_image_seconds: create_cover.og_image_seconds,
            })
            .await?;
    }

    Ok(())
}
