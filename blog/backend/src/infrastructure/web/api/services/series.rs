use std::{path::PathBuf, str::FromStr};

use sqlx::SqlitePool;
use tokio::fs;

use crate::{
    application::{
        commands::series::{AddPostToSeriesCommand, GetSeriesCommand, NewSeriesCommand},
        services::series::SeriesService,
    },
    domain::{
        entities::{media::MediaType, series::Series},
        errors::{media::MediaError, series::SeriesError},
    },
    infrastructure::web::{
        api::services::media::{HashData, hash_bytes},
        server::MediaConfig,
    },
};

pub struct SeriesServiceImpl {
    pub pool: SqlitePool,
}

impl SeriesServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl SeriesServiceImpl {
    async fn is_cover_supported(
        &self,
        file_type: &str,
        config: &MediaConfig,
    ) -> Result<bool, MediaError> {
        let media_type = MediaType::from_str(file_type)?;

        Ok(config.allowed_cover_types.contains(&media_type))
    }
}

#[async_trait::async_trait]
impl SeriesService for SeriesServiceImpl {
    async fn get_series(&self, cmd: GetSeriesCommand) -> Result<Vec<Series>, SeriesError> {
        let series = sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT title, slug
            FROM series
            WHERE user_id = ?
            "#,
        )
        .bind(&cmd.user_id)
        .fetch_all(&self.pool)
        .await?;

        let result = series
            .into_iter()
            .map(|(title, slug)| Series { title, slug })
            .collect();

        Ok(result)
    }
    async fn new_series(
        &self,
        cmd: NewSeriesCommand,
        config: &MediaConfig,
    ) -> Result<bool, SeriesError> {
        let mut tx = self.pool.begin().await?;

        let mut image_id: Option<i64> = None;
        let mut created_file_path: Option<PathBuf> = None;
        if let Some(cover_image) = cmd.cover_image {
            if !self
                .is_cover_supported(&cover_image.content_type, config)
                .await?
            {
                return Err(SeriesError::Media(MediaError::InvalidFileType));
            }

            let extension = MediaType::from_str(&cover_image.content_type)?.get_extension();

            let root = config.dir.join("srs").join(&cmd.user_id.to_string());

            let HashData {
                mut hash,
                size,
                dir_path,
                file_path,
            } = hash_bytes(&cover_image.bytes, &root, extension.to_string(), false).await?;

            if fs::try_exists(&file_path).await? {
                return Ok(true);
            }

            fs::create_dir_all(&dir_path).await?;
            fs::write(&file_path, &cover_image.bytes).await?;

            let short_name = format!(".srs.{}", hash);

            hash = format!(".avt.{}.{}", &cmd.user_id, hash);

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
            .bind(&cover_image.filename)
            .bind(&cover_image.content_type)
            .bind(&file_path.to_str().ok_or(MediaError::ExposedInternalError(
                "Failed to get file path".to_string(),
            ))?)
            .bind(size)
            .bind(&cmd.user_id)
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

        let exists: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT post_id
            FROM series_posts
            LEFT JOIN series ON series_id = series.id
            LEFT JOIN posts ON post_id = posts.id
            WHERE
                series.user_id = posts.user_id
                AND series_id = ?
                AND post_id = ?
            "#,
        )
        .bind(&cmd.series_id)
        .bind(&cmd.post_id)
        .fetch_optional(&mut *tx)
        .await?;

        if exists.is_none() {
            return Err(SeriesError::PermissionDenied);
        }

        let target = match cmd.number {
            Some(n) => n,
            None => {
                let max: Option<i64> =
                    sqlx::query_scalar("SELECT MAX(number) FROM series_posts WHERE series_id = ?")
                        .bind(&cmd.series_id)
                        .fetch_one(&mut *tx)
                        .await?;
                max.map_or(1, |v| v + 1)
            }
        };

        sqlx::query(
            r#"
            UPDATE series_posts
            SET number = number + 1
            WHERE series_id = ?
            AND number >= ?
            AND number < (
                SELECT MIN(t2.number)
                FROM series_posts t2
                WHERE
                    t2.series_id = ?
                    AND t2.number >= ?
                    AND t2.number + 1
                    NOT IN (
                        SELECT number
                        FROM series_posts
                        WHERE series_id = ?
                    )
            ) + 1
            "#,
        )
        .bind(&cmd.series_id)
        .bind(target)
        .bind(&cmd.series_id)
        .bind(target)
        .bind(&cmd.series_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO series_posts (series_id, post_id, number)
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
}
