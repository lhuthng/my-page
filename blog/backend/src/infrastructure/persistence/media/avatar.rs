// Avatar replacement: delete the old image, store the new one.
use std::path::PathBuf;

use axum::body::Bytes;
use sqlx::Row;

use crate::application::commands::media::ChangeAvatarCommand;
use crate::domain::entities::media::{MediaDetails, MediaType, MediumDetails};
use crate::domain::errors::media::MediaError;

use super::hashing::{generate_dir_and_name, hash_bytes};
use super::validation;
use super::MediaServiceImpl;

#[async_trait::async_trait]
impl MediaService for MediaServiceImpl {
    async fn change_avatar(
        &self,
        cmd: ChangeAvatarCommand,
        config: &MediaConfig,
    ) -> Result<(), MediaError> {
        let MediumDetails {
            content_type,
            filename,
            bytes,
        } = cmd.medium_details;

        let (bytes, content_type, filename) = convert_to_webp(bytes, &content_type, &filename).await?;

        if !self.is_avatar_supported(&content_type, config).await? {
            return Err(MediaError::InvalidFileType);
        }

        let mut tx = self.pool.begin().await?;

        let hash_row = sqlx::query_as::<_, (Option<i64>, Option<String>, Option<String>)>(
            r#"
            SELECT media.id, media.hash, media.file_type
            FROM users
            JOIN user_meta ON users.id = user_meta.user_id
            LEFT JOIN media on media.id = user_meta.avatar_image_id
            WHERE users.id = ?
            "#,
        )
        .bind(cmd.user_id)
        .fetch_one(&mut *tx)
        .await?;

        if let (Some(id), _, _) = hash_row {
            sqlx::query(
                r#"
                DELETE FROM media
                WHERE id = ?
                "#,
            )
            .bind(id)
            .execute(&mut *tx)
            .await?;
        }

        let extension = MediaType::from_str(&content_type)?.get_extension();

        let root = config.dir.join("avt").join(cmd.user_id.to_string());

        let HashData {
            mut hash,
            size,
            dir_path,
            file_path,
        } = hash_bytes(&bytes, &root, extension.to_string(), false).await?;

        let file_already_exists = fs::try_exists(&file_path).await?;
        let file_was_written = !file_already_exists;
        if !file_already_exists {
            fs::create_dir_all(&dir_path).await?;
            fs::write(&file_path, &bytes).await?;
        }

        let short_name = format!(".avt.{}", cmd.user_id);
        let content_hash = hash.clone();

        hash = format!(".avt.{}.{}", cmd.user_id, hash);

        match sqlx::query_as::<_, (i64,)>(
            r#"
            INSERT INTO media
            (hash, short_name, file_name, file_type, url, size, uploader_id)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(&hash)
        .bind(short_name)
        .bind(&filename)
        .bind(&content_type)
        .bind(file_path.to_str().ok_or(MediaError::ExposedInternalError(
            "Failed to get file path".to_string(),
        ))?)
        .bind(size)
        .bind(cmd.user_id)
        .fetch_one(&mut *tx)
        .await
        {
            Ok((image_id,)) => {
                sqlx::query(
                    r#"
                    UPDATE user_meta
                    SET avatar_image_id = ?
                    WHERE user_id = ?
                    "#,
                )
                .bind(image_id)
                .bind(cmd.user_id)
                .execute(&mut *tx)
                .await?;
            }
            Err(e) => {
                if file_was_written && let Err(remove_err) = fs::remove_file(&file_path).await {
                    return Err(MediaError::ExposedInternalError(format!(
                        "Failed to remove file after DB error {}",
                        remove_err
                    )));
                }
                return Err(MediaError::InternalError(e.to_string()));
            }
        }

        tx.commit().await?;

        if let (_, Some(old_hash), Some(file_type)) = hash_row {
            let old_hash = match old_hash.split('.').nth(3) {
                Some(hash) => hash.to_string(),
                None => {
                    return Err(MediaError::UploadFailed(
                        "Invalid stored hash found".to_string(),
                    ));
                }
            };

            if old_hash != content_hash {
                let extension = match file_type.split('/').nth(1) {
                    Some(extension) => format!(".{}", extension),
                    None => {
                        return Err(MediaError::UploadFailed(
                            "Invalid stored file type found".to_string(),
                        ));
                    }
                };

                let (dir_path, file_name) =
                    generate_dir_and_name(&root, &old_hash, extension.to_string(), false);

                let file_path = dir_path.join(file_name);

                if let Err(remove_err) = fs::remove_file(&file_path).await {
                    return Err(MediaError::ExposedInternalError(format!(
                        "Failed to clean up previous avatar after updated {}",
                        remove_err
                    )));
                }
            }
        }

        Ok(())
    }
}
