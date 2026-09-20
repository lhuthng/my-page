// Series write methods: create, attach and detach posts.
use std::collections::HashMap;

use sqlx::Row;

use crate::application::{
    commands::series::{
        AddPostToSeriesCommand, NewSeriesCommand, RemovePostFromSeriesCommand,
    },
    services::series::SeriesService,
};
use crate::domain::errors::series::SeriesError;

use super::SeriesServiceImpl;

#[async_trait::async_trait]
impl SeriesService for SeriesServiceImpl {
    async fn new_series(
        &self,
        cmd: NewSeriesCommand,
        config: &MediaConfig,
) -> Result<bool, SeriesError> {
        use crate::{
            application::commands::series::NewSeriesCommand as C, helper::string::*,
        };
        let description = {
            let trimmed = cmd.description.trim();
            if trimmed.chars().count() > 1000 {
                return Err(SeriesError::Validation(
                    "Description must be at most 1000 characters.".into(),
                ));
            }
            trimmed.to_string()
        };
        let cmd = C {
            title: validate_text(&cmd.title, "Title", 300).map_err(SeriesError::Validation)?,
            slug: validate_slug(&cmd.slug).map_err(SeriesError::Validation)?,
            description,
            ..cmd
        };

        let mut tx = self.pool.begin().await?;

        let mut image_id: Option<i64> = None;
        let mut created_file_path: Option<PathBuf> = None;
        if let Some(cover_image) = cmd.cover_image {
            let (bytes, content_type, filename) = convert_to_webp(
                cover_image.bytes,
                &cover_image.content_type,
                &cover_image.filename,
            )
            .await?;

            if !self.is_cover_supported(&content_type, config).await? {
                return Err(SeriesError::Media(MediaError::InvalidFileType));
            }

            let extension = MediaType::from_str(&content_type)?.get_extension();

            let root = config.dir.join("srs").join(cmd.user_id.to_string());

            let HashData {
                mut hash,
                size,
                dir_path,
                file_path,
            } = hash_bytes(&bytes, &root, extension.to_string(), false).await?;

            if fs::try_exists(&file_path).await? {
                return Ok(true);
            }

            fs::create_dir_all(&dir_path).await?;
            fs::write(&file_path, &bytes).await?;

            let short_name = format!(".srs.{}", hash);

            hash = format!(".srs.{}.{}", &cmd.user_id, hash);

            image_id = match sqlx::query_as::<_, (i64,)>(
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
                    created_file_path = Some(file_path);
                    Some(image_id)
                }
                Err(e) => {
                    if let Err(remove_err) = fs::remove_file(&file_path).await {
                        return Err(SeriesError::Media(MediaError::ExposedInternalError(
                            format!("Failed to remove file after DB error {}", remove_err),
                        )));
                    }
                    return Err(SeriesError::Media(MediaError::InternalError(e.to_string())));
                }
            };
        };

        sqlx::query(
            r#"
            INSERT INTO series (title, slug, description, user_id, cover_image_id)
            VALUES (?, ?, ?, ?, ?);
            "#,
        )
        .bind(cmd.title)
        .bind(cmd.slug)
        .bind(cmd.description)
        .bind(cmd.user_id)
        .bind(image_id)
        .execute(&mut *tx)
        .await?;

        match tx.commit().await {
            Ok(()) => Ok(true),
            Err(e) => {
                if let Some(file_path) = created_file_path
                    && let Err(remove_err) = fs::remove_file(&file_path).await
                {
                    return Err(SeriesError::Media(MediaError::ExposedInternalError(
                        format!("Failed to remove file after DB error {}", remove_err),
                    )));
                }
                return Err(SeriesError::Media(MediaError::InternalError(e.to_string())));
            }
        }
    }
    async fn add_post_to_series(&self, cmd: AddPostToSeriesCommand) -> Result<bool, SeriesError> {
        let mut tx = self.pool.begin().await?;
        // 2. PERMISSION CHECK
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM posts p, series s
                WHERE p.id = ? AND s.id = ? AND p.user_id = s.user_id AND p.user_id = ? AND p.content_kind = 'post'
            )"#,
        )
        .bind(cmd.post_id)
        .bind(cmd.series_id)
        .bind(cmd.user_id)
        .fetch_one(&mut *tx)
        .await?;

        if !exists {
            return Err(SeriesError::PermissionDenied);
        }

        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM series_post
                WHERE post_id = ?
            )
            "#,
        )
        .bind(cmd.post_id)
        .fetch_one(&mut *tx)
        .await?;

        if exists {
            return Err(SeriesError::Duplication);
        }

        // 4. CALCULATE TARGET
        let target = match cmd.number {
            Some(n) => n,
            None => {
                let max: i64 = sqlx::query_scalar(
                    r#"
                    SELECT IFNULL(MAX(number), 0)
                    FROM series_post
                    WHERE series_id = ?"#,
                )
                .bind(cmd.series_id)
                .fetch_one(&mut *tx)
                .await?;
                max + 1
            }
        };

        // 5. SHIFT EXISTING POSTS
        sqlx::query(
            r#"
            UPDATE series_post
            SET number = number + 1
            WHERE series_id = ? AND number >= ?"#,
        )
        .bind(cmd.series_id)
        .bind(target)
        .execute(&mut *tx)
        .await?;

        // 6. INSERT NEW POST
        sqlx::query(
            r#"
            INSERT INTO series_post (series_id, post_id, number)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(cmd.series_id)
        .bind(cmd.post_id)
        .bind(target)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(true)
    }

    async fn remove_post_from_series(
        &self,
        cmd: RemovePostFromSeriesCommand,
    ) -> Result<bool, SeriesError> {
        // 1. START A TRANSACTION
        let mut tx = self.pool.begin().await?;

        // 2. PERMISSION CHECK & GET CURRENT POSITION
        let current_position: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT sp.number
            FROM series_post sp
            JOIN series s ON sp.series_id = s.id
            WHERE sp.series_id = ? AND sp.post_id = ? AND s.user_id = ?
            "#,
        )
        .bind(cmd.series_id)
        .bind(cmd.post_id)
        .bind(cmd.user_id)
        .fetch_optional(&mut *tx)
        .await?;

        let position = match current_position {
            Some(pos) => pos,
            None => return Err(SeriesError::PermissionDenied), // Or NotFound
        };

        // 3. DELETE THE POST FROM THE SERIES
        sqlx::query("DELETE FROM series_post WHERE series_id = ? AND post_id = ?")
            .bind(cmd.series_id)
            .bind(cmd.post_id)
            .execute(&mut *tx)
            .await?;

        // 4. CLOSE THE GAP
        sqlx::query(
            r#"
            UPDATE series_post
            SET number = number - 1
            WHERE series_id = ? AND number > ?
            "#,
        )
        .bind(cmd.series_id)
        .bind(position)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(true)
    }
}
