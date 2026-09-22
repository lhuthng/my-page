// Media ingestion: single upload and bulk inline-media upload.
use std::path::PathBuf;
use std::str::FromStr;

use tokio::fs;

use crate::application::commands::media::{
    UploadMediaWithoutDescriptionCommand, UploadMediumCommand,
};
use crate::domain::entities::media::MediaType;
use crate::domain::errors::media::MediaError;
use crate::infrastructure::persistence::image_convert::convert_to_webp;
use crate::infrastructure::web::server::MediaConfig;

use super::MediaServiceImpl;
use super::files::clean_up_files;
use super::hashing::{HashData, hash_bytes};

impl MediaServiceImpl {
    pub(super) async fn upload(
        &self,
        cmd: UploadMediumCommand,
        config: &MediaConfig,
    ) -> Result<(), MediaError> {
        let (bytes, content_type, file_name) =
            convert_to_webp(cmd.bytes, &cmd.content_type, &cmd.file_name).await?;
        if !self.is_supported(&content_type, &file_name, config).await? {
            return Err(MediaError::InvalidFileType);
        }
        let media_type = MediaType::from_upload(&content_type, &file_name)?;
        let content_type = media_type.get_content_type().to_string();
        let extension = media_type.get_extension();

        let HashData {
            hash,
            size,
            dir_path,
            file_path,
        } = hash_bytes(&bytes, &config.dir, extension.to_string(), true).await?;

        if fs::try_exists(&file_path).await? {
            return Err(MediaError::Duplication);
        }

        fs::create_dir_all(&dir_path).await?;
        fs::write(&file_path, &bytes).await?;

        let mut tx = self.pool.begin().await?;

        let result = sqlx::query(
            r#"
            INSERT INTO media
            (hash, short_name, file_name, file_type, url, size, description, uploader_id)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&hash)
        .bind(&cmd.short_name)
        .bind(&file_name)
        .bind(&content_type)
        .bind(file_path.to_str().ok_or(MediaError::ExposedInternalError(
            "Failed to get file path".to_string(),
        ))?)
        .bind(size)
        .bind(&cmd.description)
        .bind(cmd.uploader_id)
        .execute(&mut *tx)
        .await;

        if let Err(e) = result {
            if let Err(remove_err) = fs::remove_file(&file_path).await {
                return Err(MediaError::InternalError(format!(
                    "Failed to remove file after DB error {}",
                    remove_err
                )));
            }
            return Err(MediaError::InternalError(e.to_string()));
        }

        tx.commit().await?;

        Ok(())
    }
    pub(super) async fn bulk_upload(
        &self,
        cmd: UploadMediaWithoutDescriptionCommand,
        config: &MediaConfig,
    ) -> Result<(), MediaError> {
        let mut cmd = cmd;
        for i in 0..cmd.number_of_files {
            let bytes = cmd.bytes_list[i].clone();
            let (new_bytes, new_content_type, new_filename) =
                convert_to_webp(bytes, &cmd.content_types[i], &cmd.file_names[i]).await?;
            cmd.bytes_list[i] = new_bytes;
            let media_type = MediaType::from_upload(&new_content_type, &new_filename)?;
            cmd.content_types[i] = media_type.get_content_type().to_string();
            cmd.file_names[i] = new_filename;
        }

        for i in 0..cmd.number_of_files {
            if !self
                .is_supported(&cmd.content_types[i], &cmd.file_names[i], config)
                .await?
            {
                println!("InvalidFileType?");
                return Err(MediaError::InvalidFileType);
            }
        }

        let mut tx = self.pool.begin().await?;
        let mut got_error: Option<MediaError> = None;
        let mut file_paths = Vec::<PathBuf>::new();

        for i in 0..cmd.number_of_files {
            let extension = match MediaType::from_str(&cmd.content_types[i]) {
                Ok(media_type) => media_type.get_extension(),
                Err(e) => {
                    got_error = Some(e);
                    break;
                }
            };

            let HashData {
                hash,
                size,
                dir_path,
                file_path,
            } = match hash_bytes(&cmd.bytes_list[i], &config.dir, extension.to_string(), true).await
            {
                Ok(hash_data) => hash_data,
                Err(e) => {
                    got_error = Some(e);
                    break;
                }
            };

            if !fs::try_exists(&file_path).await? {
                fs::create_dir_all(&dir_path).await?;
                fs::write(&file_path, &cmd.bytes_list[i]).await?;

                file_paths.push(file_path.clone());
            }

            let file_path_str = match file_path.to_str() {
                Some(file_path_str) => file_path_str.to_string(),
                None => {
                    got_error = Some(MediaError::InternalError(
                        "Failed to get file path".to_string(),
                    ));
                    break;
                }
            };

            if let Err(e) = sqlx::query(
                r#"
                INSERT INTO media
                (hash, short_name, file_name, file_type, url, size, uploader_id)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(short_name) DO UPDATE SET
                    hash = excluded.hash,
                    file_name = excluded.file_name,
                    file_type = excluded.file_type,
                    url = excluded.url,
                    size = excluded.size,
                    uploader_id = excluded.uploader_id
                "#,
            )
            .bind(&hash)
            .bind(&cmd.short_names[i])
            .bind(&cmd.file_names[i])
            .bind(&cmd.content_types[i])
            .bind(&file_path_str)
            .bind(size)
            .bind(cmd.uploader_id)
            .execute(&mut *tx)
            .await
            {
                got_error = Some(MediaError::InternalError(e.to_string()));
                break;
            }
        }

        if let Some(error) = got_error {
            clean_up_files(&file_paths).await?;
            return Err(error);
        }

        match tx.commit().await {
            Ok(()) => Ok(()),
            Err(e) => {
                clean_up_files(&file_paths).await?;
                Err(MediaError::from(e))
            }
        }
    }
}
