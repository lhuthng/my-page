use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    str::FromStr,
};

use sqlx::SqlitePool;
use tokio::fs;

use crate::{
    application::{
        commands::series::{
            AddPostToSeriesCommand, GetAllSeriesCommand, GetSeriesCommand, NewSeriesCommand,
            RemovePostFromSeriesCommand,
        },
        services::series::SeriesService,
    },
    domain::{
        entities::{
            media::MediaType,
            post::PostSnapshot,
            series::{SeriesSnapshot, SeriesWithPosts},
        },
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
    async fn get_series(&self, cmd: GetSeriesCommand) -> Result<Vec<SeriesSnapshot>, SeriesError> {
        let series = sqlx::query_as::<_, (i64, String, String, String, Option<String>)>(
            r#"
            SELECT series.id, series.title, series.slug, series.description, media.url
            FROM series
            LEFT JOIN media ON series.cover_image_id = media.id
            WHERE user_id = ?
            "#,
        )
        .bind(&cmd.user_id)
        .fetch_all(&self.pool)
        .await?;

        let result = series
            .into_iter()
            .map(|(id, title, slug, description, url)| SeriesSnapshot {
                id,
                title,
                slug,
                description,
                url,
            })
            .collect();

        Ok(result)
    }
    async fn get_all_series(
        &self,
        cmd: GetAllSeriesCommand,
    ) -> Result<Vec<SeriesWithPosts>, SeriesError> {
        let mut tx = self.pool.begin().await?;
        let all_series_posts = sqlx::query_as::<_, (i64, i64, i64)>(
            r#"
            SELECT sp.series_id, sp.post_id, sp.number
            FROM series_post sp
            JOIN posts ON posts.id = sp.post_id
            WHERE posts.status = 'published'
              AND sp.series_id IN (
                  SELECT DISTINCT sp2.series_id
                  FROM series_post sp2
                  JOIN posts p2 ON p2.id = sp2.post_id
                  WHERE p2.status = 'published'
                  ORDER BY sp2.series_id DESC
                  LIMIT ?
                  OFFSET ?
              )
            ORDER BY sp.series_id DESC, sp.number DESC;
            "#,
        )
        .bind(cmd.limit)
        .bind(cmd.offset)
        .fetch_all(&mut *tx)
        .await?;

        let mut series_set = HashSet::<i64>::new();
        let mut post_set = HashSet::<i64>::new();
        let mut series_post_hash_map = HashMap::<i64, Vec<(i64, i64)>>::new();

        for &(series_id, post_id, number) in &all_series_posts {
            series_set.insert(series_id);
            post_set.insert(post_id);
            series_post_hash_map
                .entry(series_id)
                .or_default()
                .push((post_id, number));
        }

        // Create series hash map
        let all_series_ids = series_set
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!(
            r#"
            SELECT s.id, s.title, s.slug, u.username, um.display_name, s.description, m.url
            FROM series s
            LEFT JOIN users u ON u.id = s.user_id
            LEFT JOIN user_meta um ON um.user_id = u.id
            LEFT JOIN media m ON s.cover_image_id = m.id
            WHERE s.id IN ({})
            "#,
            all_series_ids
        );
        let all_series = sqlx::query_as::<
            _,
            (i64, String, String, String, String, String, Option<String>),
        >(&sql)
        .fetch_all(&mut *tx)
        .await?;

        let mut series_hash_map =
            HashMap::<i64, (String, String, String, String, String, Option<String>)>::new();
        for (id, title, slug, username, display_name, description, url) in all_series {
            series_hash_map.insert(id, (title, slug, username, display_name, description, url));
        }

        // Create post hash map
        let all_post_ids = post_set
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!(
            r#"
            SELECT p.id, p.title, p.slug, p.excerpt, um.display_name, u.username, p.status, m.url
            FROM posts p
            LEFT JOIN users u ON u.id = p.user_id
            LEFT JOIN user_meta um ON um.user_id = u.id
            LEFT JOIN media m ON m.id = p.cover_image_id
            WHERE p.id IN ({})
            "#,
            all_post_ids
        );
        let all_posts = sqlx::query_as::<
            _,
            (
                i64,
                String,
                String,
                String,
                String,
                String,
                String,
                Option<String>,
            ),
        >(&sql)
        .fetch_all(&mut *tx)
        .await?;

        let mut post_hash_map = HashMap::<
            i64,
            (
                String,
                String,
                String,
                String,
                String,
                String,
                Option<String>,
            ),
        >::new();
        for (id, title, slug, excerpt, display_name, username, status, url) in all_posts {
            post_hash_map.insert(
                id,
                (title, slug, excerpt, display_name, username, status, url),
            );
        }

        // Create tag hash_map
        let sql = format!(
            r#"
            SELECT pt.post_id, t.name, t.slug
            FROM post_tags pt
            LEFT JOIN tags t ON pt.tag_id = t.id
            WHERE pt.post_id IN ({})
            "#,
            all_post_ids
        );
        let all_post_tags = sqlx::query_as::<_, (i64, String, String)>(&sql)
            .fetch_all(&mut *tx)
            .await?;

        let mut tag_name_hash_map = HashMap::<i64, Vec<(String)>>::new();
        let mut tag_slug_hash_map = HashMap::<i64, Vec<(String)>>::new();
        for (id, name, slug) in all_post_tags {
            tag_name_hash_map.entry(id).or_default().push(name);
            tag_slug_hash_map.entry(id).or_default().push(slug);
        }

        // You have series_hash_map, series_post_hash_map, post_hash_map, and tag_hash_map
        let mut result = Vec::<SeriesWithPosts>::new();

        let mut series_ids: Vec<_> = series_post_hash_map.keys().cloned().collect();
        series_ids.sort_by(|a, b| b.cmp(a));

        for series_id in series_ids {
            if let Some(posts_with_numbers) = series_post_hash_map.get(&series_id) {
                let mut numbers = Vec::<i64>::new();
                let mut posts = Vec::<PostSnapshot>::new();

                // String, String, String, String, String, Option<String>
                if let Some((title, slug, username, display_name, description, url)) =
                    series_hash_map.get(&series_id)
                {
                    for &(post_id, number) in posts_with_numbers {
                        numbers.push(number);

                        if let Some((title, slug, excerpt, author_name, author_slug, status, url)) =
                            post_hash_map.get(&post_id)
                        {
                            if let Some((tag_names, tag_slugs)) = tag_name_hash_map
                                .get(&post_id)
                                .zip(tag_slug_hash_map.get(&post_id))
                            {
                                posts.push(PostSnapshot {
                                    id: post_id,
                                    title: title.clone(),
                                    slug: slug.clone(),
                                    tag_names: tag_names.clone(),
                                    tag_slugs: tag_slugs.clone(),
                                    excerpt: excerpt.clone(),
                                    author_name: author_name.clone(),
                                    author_slug: author_slug.clone(),
                                    status: status.clone(),
                                    url: url.clone(),
                                });
                            }
                        }
                    }
                    result.push(SeriesWithPosts {
                        id: series_id,
                        title: title.clone(),
                        slug: slug.clone(),
                        username: username.clone(),
                        display_name: display_name.clone(),
                        description: description.clone(),
                        url: url.clone(),
                        posts,
                        numbers,
                    })
                }
            }
        }

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
        // 2. PERMISSION CHECK
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM posts p, series s
                WHERE p.id = ? AND s.id = ? AND p.user_id = s.user_id AND p.user_id = ?
            )"#,
        )
        .bind(&cmd.post_id)
        .bind(&cmd.series_id)
        .bind(&cmd.user_id)
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
        .bind(&cmd.post_id)
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
                .bind(&cmd.series_id)
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
        .bind(&cmd.series_id)
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
            FROM series_posts sp
            JOIN series s ON sp.series_id = s.id
            WHERE sp.series_id = ? AND sp.post_id = ? AND s.user_id = ?
            "#,
        )
        .bind(&cmd.series_id)
        .bind(&cmd.post_id)
        .bind(&cmd.user_id)
        .fetch_optional(&mut *tx)
        .await?;

        let position = match current_position {
            Some(pos) => pos,
            None => return Err(SeriesError::PermissionDenied), // Or NotFound
        };

        // 3. DELETE THE POST FROM THE SERIES
        sqlx::query("DELETE FROM series_post WHERE series_id = ? AND post_id = ?")
            .bind(&cmd.series_id)
            .bind(&cmd.post_id)
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
        .bind(&cmd.series_id)
        .bind(position)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(true)
    }
}
