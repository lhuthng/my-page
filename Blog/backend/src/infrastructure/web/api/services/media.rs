use std::str::FromStr;

use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use tokio::fs;

use crate::{
    application::{commands::media::UploadMediaCommand, services::media::MediaService},
    domain::{entities::media::MediaType, errors::media::MediaError},
    infrastructure::web::server::MediaConfig,
};

pub struct MediaServiceImpl {
    pub pool: SqlitePool,
}

impl MediaServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl MediaService for MediaServiceImpl {
    async fn is_supported(
        &self,
        file_type: &str,
        config: &MediaConfig,
    ) -> Result<bool, MediaError> {
        let media_type = MediaType::from_str(file_type)?;

        Ok(config.allowed_file_types.contains(&media_type))
    }
    async fn upload(
        &self,
        cmd: UploadMediaCommand,
        config: &MediaConfig,
    ) -> Result<(), MediaError> {
        if !self.is_supported(&cmd.content_type, config).await? {
            return Err(MediaError::InvalidFileType);
        }
        let media_type = MediaType::from_str(&cmd.content_type)?;
        let extension = media_type.get_extension();

        let hash = format!("{:x}", Sha256::digest(&cmd.bytes));

        if hash.len() < 4 {
            return Err(MediaError::ExposedInternalError(
                "Hash too short".to_string(),
            ));
        }
        let dir1 = &hash[0..2];
        let dir2 = &hash[2..4];

        let dir_path = config.dir.join(dir1).join(dir2);
        let h_filename = format!("{}{}", hash, extension);
        let file_path = dir_path.join(h_filename);
        let size = cmd.bytes.len() as i64;

        if fs::try_exists(&dir_path).await? {
            return Err(MediaError::Duplication);
        }

        fs::create_dir_all(&dir_path).await?;
        fs::write(&file_path, &cmd.bytes).await?;

        let result = sqlx::query(
            r#"
            INSERT INTO media
            (hash, short_name, file_name, file_type, url, size, description, uploader_id)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&hash)
        .bind(&cmd.short_name)
        .bind(&cmd.filename)
        .bind(&cmd.content_type)
        .bind(&file_path.to_str().ok_or(MediaError::ExposedInternalError(
            "Failed to get file path".to_string(),
        ))?)
        .bind(size)
        .bind(&cmd.description)
        .bind(cmd.uploader_id)
        .execute(&self.pool)
        .await;

        if let Err(e) = result {
            if let Err(remove_err) = fs::remove_file(&file_path).await {
                return Err(MediaError::ExposedInternalError(format!(
                    "Failed to remove file after DB error {}",
                    remove_err
                )));
            }
            return Err(MediaError::ExposedInternalError(e.to_string()));
        }

        Ok(())
    }
}
