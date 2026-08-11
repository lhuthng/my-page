use crate::domain::errors::media::MediaError;
use axum::body::Bytes;

pub async fn convert_to_webp(
    bytes: Bytes,
    content_type: &str,
    filename: &str,
) -> Result<(Bytes, String, String), MediaError> {
    if content_type != "image/png" && content_type != "image/jpeg" {
        return Ok((bytes, content_type.to_string(), filename.to_string()));
    }

    let filename = filename.to_string();

    tokio::task::spawn_blocking(move || {
        let img = image::load_from_memory(&bytes)
            .map_err(|e| MediaError::UploadFailed(format!("Failed to load image: {}", e)))?;

        let mut webp_bytes = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut webp_bytes),
            image::ImageFormat::WebP,
        )
        .map_err(|e| MediaError::UploadFailed(format!("Failed to encode to WebP: {}", e)))?;

        let path = std::path::Path::new(&filename);
        let new_filename = path.with_extension("webp").to_string_lossy().to_string();

        Ok((Bytes::from(webp_bytes), "image/webp".to_string(), new_filename))
    })
    .await
    .map_err(|e| MediaError::UploadFailed(format!("Image processing task failed: {}", e)))?
}
