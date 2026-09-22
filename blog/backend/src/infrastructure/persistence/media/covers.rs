// Post cover replacement: stores the new cover medium, rewires the post,
// and cleans up the replaced object.
use std::str::FromStr;

use tokio::fs;

use crate::application::commands::media::ChangePostCoverCommand;
use crate::domain::entities::media::{MediaType, MediumDetails};
use crate::domain::errors::media::MediaError;
use crate::infrastructure::persistence::image_convert::convert_to_webp;
use crate::infrastructure::web::server::MediaConfig;

use super::MediaServiceImpl;
use super::hashing::{HashData, generate_dir_and_name, hash_bytes};

impl MediaServiceImpl {
    pub(super) async fn change_post_cover(
        &self,
        cmd: ChangePostCoverCommand,
        config: &MediaConfig,
    ) -> Result<(), MediaError> {
        let MediumDetails {
            content_type,
            filename,
            bytes,
        } = cmd.medium_details;

        let (bytes, content_type, filename) =
            convert_to_webp(bytes, &content_type, &filename).await?;

        let media_type = MediaType::from_str(&content_type)?;
        if !config.allowed_cover_types.contains(&media_type) {
            return Err(MediaError::InvalidFileType);
        }

        let mut tx = self.pool.begin().await?;
        let exist: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM posts
                WHERE id = ? AND user_id = ?
            )
            "#,
        )
        .bind(cmd.post_id)
        .bind(cmd.user_id)
        .fetch_one(&mut *tx)
        .await?;
        if !exist {
            return Err(MediaError::PermissionDenied);
        }

        let old_cover = sqlx::query_as::<_, (Option<i64>, Option<String>, Option<String>)>(
            r#"
            SELECT media.id, media.hash, media.file_type
            FROM posts
            LEFT JOIN media on media.id = posts.cover_media_id
            WHERE posts.id = ?
            "#,
        )
        .bind(cmd.post_id)
        .fetch_one(&mut *tx)
        .await?;

        if let (Some(id), _, _) = old_cover {
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

        // Delete old video media (by short_name) to avoid collision on re-upload
        let old_video = sqlx::query_as::<_, (i64, String, String)>(
            r#"
            SELECT id, hash, file_type
            FROM media
            WHERE short_name = ?
            "#,
        )
        .bind(format!(".post.{}", cmd.post_id))
        .fetch_optional(&mut *tx)
        .await?;
        if let Some((old_video_id, _, _)) = old_video {
            sqlx::query(
                r#"
                DELETE FROM media
                WHERE id = ?
                "#,
            )
            .bind(old_video_id)
            .execute(&mut *tx)
            .await?;
        }

        let old_thumbnail = sqlx::query_as::<_, (i64, String, String)>(
            r#"
            SELECT id, hash, file_type
            FROM media
            WHERE short_name = ?
            "#,
        )
        .bind(format!(".post.{}.thumbnail", cmd.post_id))
        .fetch_optional(&mut *tx)
        .await?;
        if let Some((old_thumbnail_id, _, _)) = old_thumbnail {
            sqlx::query(
                r#"
                DELETE FROM media
                WHERE id = ?
                "#,
            )
            .bind(old_thumbnail_id)
            .execute(&mut *tx)
            .await?;
        }

        let extension = MediaType::from_str(&content_type)?.get_extension();

        let root = config.dir.join("post").join(cmd.user_id.to_string());

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

        let short_name = format!(".post.{}", cmd.post_id);
        let content_hash = hash.clone();

        hash = format!(".post.{}.{}", cmd.post_id, hash);

        let _video_media_id = match sqlx::query_as::<_, (i64,)>(
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
                    UPDATE posts
                    SET cover_media_id = ?
                    WHERE id = ?
                    "#,
                )
                .bind(image_id)
                .bind(cmd.post_id)
                .execute(&mut *tx)
                .await?;
                image_id
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
        };

        tx.commit().await?;

        // Pre-generate thumbnail for video covers; save as a proper media record
        if content_type.starts_with("video/") {
            let thumb_seconds = cmd.og_image_seconds.unwrap_or(1);
            let output = tokio::process::Command::new("ffmpeg")
                .args(["-ss", &thumb_seconds.to_string()])
                .args(["-i", &file_path.to_string_lossy()])
                .args(["-vframes", "1"])
                .args(["-f", "image2pipe", "-"])
                .output()
                .await
                .map_err(|e| MediaError::InternalError(format!("ffmpeg error: {}", e)))?;

            if !output.status.success() {
                return Err(MediaError::InternalError(format!(
                    "ffmpeg failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }

            let img = image::load_from_memory(&output.stdout)
                .map_err(|e| MediaError::InternalError(format!("image decode error: {}", e)))?;
            let mut webp_bytes = Vec::new();
            img.write_to(
                &mut std::io::Cursor::new(&mut webp_bytes),
                image::ImageFormat::WebP,
            )
            .map_err(|e| MediaError::InternalError(format!("webp encode error: {}", e)))?;

            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(&webp_bytes);
            let thumb_hash = format!("{:x}", hasher.finalize());

            let thumb_dir = config.dir.join("post").join(cmd.user_id.to_string());
            let thumb_file_path = thumb_dir.join(format!("{}.webp", thumb_hash));
            tokio::fs::create_dir_all(&thumb_dir)
                .await
                .map_err(|e| MediaError::InternalError(e.to_string()))?;
            tokio::fs::write(&thumb_file_path, &webp_bytes)
                .await
                .map_err(|e| MediaError::InternalError(e.to_string()))?;

            let thumb_short_name = format!(".post.{}.thumbnail", cmd.post_id);
            let thumb_hash_db = format!(".post.{}.{}", cmd.post_id, thumb_hash);
            let thumb_url = format!("post/{}/{}.webp", cmd.user_id, thumb_hash);
            sqlx::query_as::<_, (i64,)>(
                r#"
                INSERT INTO media
                (hash, short_name, file_name, file_type, url, size, uploader_id)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                RETURNING id
                "#,
            )
            .bind(&thumb_hash_db)
            .bind(&thumb_short_name)
            .bind("cover_thumb.webp")
            .bind("image/webp")
            .bind(&thumb_url)
            .bind(webp_bytes.len() as i64)
            .bind(cmd.user_id)
            .fetch_one(&self.pool)
            .await?;
        }

        // Clean up old cover media file from disk
        if let (_, Some(old_hash), Some(old_file_type)) = old_cover {
            let ext = match old_file_type.split('/').nth(1) {
                Some(e) => format!(".{}", e),
                None => String::new(),
            };
            if old_hash.starts_with('.') {
                let parts: Vec<&str> = old_hash.splitn(4, '.').collect();
                if parts.len() >= 4 {
                    let type_dir = parts[1];
                    let sha256 = parts[3];
                    if sha256 != content_hash {
                        let old_path = config
                            .dir
                            .join(type_dir)
                            .join(cmd.user_id.to_string())
                            .join(format!("{}{}", sha256, ext));
                        let _ = fs::remove_file(&old_path).await;
                    }
                }
            } else {
                let old_root = config.dir.join("post").join(cmd.user_id.to_string());
                let (old_dir_path, old_file_name) =
                    generate_dir_and_name(&old_root, &old_hash, ext, false);
                let _ = fs::remove_file(old_dir_path.join(old_file_name)).await;
            }
        }

        // Clean up old video media file from disk
        if let Some((_, old_video_hash, old_video_file_type)) = old_video {
            let ext = match old_video_file_type.split('/').nth(1) {
                Some(e) => format!(".{}", e),
                None => String::new(),
            };
            if let Some(sha256) = old_video_hash.split('.').nth(3)
                && sha256 != content_hash
            {
                let old_video_path = config
                    .dir
                    .join("post")
                    .join(cmd.user_id.to_string())
                    .join(format!("{}{}", sha256, ext));
                let _ = fs::remove_file(&old_video_path).await;
            }
        }

        // Clean up old generated thumbnail file from disk
        if let Some((_, old_thumbnail_hash, old_thumbnail_file_type)) = old_thumbnail {
            let ext = match old_thumbnail_file_type.split('/').nth(1) {
                Some(e) => format!(".{}", e),
                None => String::new(),
            };
            if let Some(sha256) = old_thumbnail_hash.split('.').nth(3) {
                let old_thumbnail_path = config
                    .dir
                    .join("post")
                    .join(cmd.user_id.to_string())
                    .join(format!("{}{}", sha256, ext));
                let _ = fs::remove_file(&old_thumbnail_path).await;
            }
        }

        Ok(())
    }
}
