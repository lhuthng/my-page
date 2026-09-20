// Cover/audio media storage shared by the write paths.
use std::path::PathBuf;

use sqlx::Sqlite;

use crate::domain::entities::media::MediaType;
use crate::domain::errors::audiobook::AudiobookError;

use super::AudiobookServiceImpl;

pub(super) struct PreparedCover {
    media_type: MediaType,
    file_path: PathBuf,
    short_name: String,
    stored_hash: String,
    filename: String,
    size: i64,
}


impl AudiobookServiceImpl {
    pub(super) async fn store_medium(
        tx: &mut Transaction<'_, Sqlite>,
        uploader_id: i64,
        config: &MediaConfig,
        medium: &crate::domain::entities::media::MediumDetails,
        media_type: MediaType,
        short_name_prefix: String,
        created_path: &mut Option<PathBuf>,
    ) -> Result<i64, AudiobookError> {
        let content_type = media_type.get_content_type().to_string();
        let extension = media_type.get_extension();

        let HashData {
            hash,
            size,
            dir_path,
            file_path,
        } = hash_bytes(&medium.bytes, &config.dir, extension.to_string(), true).await?;

        // Content-addressed storage dedupes identical uploads: two tracks with
        // the same audio share one file on disk.
        if !fs::try_exists(&file_path).await? {
            fs::create_dir_all(&dir_path).await?;
            fs::write(&file_path, &medium.bytes).await?;
            *created_path = Some(file_path.clone());
        }

        // `short_name` is UNIQUE in `media`, so a random tail guarantees
        // uniqueness even when the same bytes are registered more than once.
        let short_name = format!(
            "{}-{}-{}",
            short_name_prefix,
            &hash[..8],
            crate::helper::string::random_suffix()
        );

        let media_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO media
            (hash, short_name, file_name, file_type, url, size, description, uploader_id)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(&hash)
        .bind(&short_name)
        .bind(&medium.filename)
        .bind(&content_type)
        .bind(file_path.to_str().ok_or_else(|| {
            AudiobookError::ExposedInternalError("Failed to get file path".to_string())
        })?)
        .bind(size)
        .bind("Audiobook track")
        .bind(uploader_id)
        .fetch_one(&mut **tx)
        .await?;

        Ok(media_id)
    }
}

