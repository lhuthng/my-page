use axum::{body::Bytes, extract::multipart::Field};

use crate::domain::errors::media::MediaError;

pub struct MediumData {
    pub filename: String,
    pub content_type: String,
    pub bytes: Bytes,
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
