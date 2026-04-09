use std::collections::HashMap;

use futures::TryFutureExt;
use sqlx::{SqlitePool, prelude::FromRow};

use crate::{
    application::{
        commands::post::{
            CheckSlugCommand, GetCategoriesCommand, GetCommentsCommand, GetDetailedPostsCommand,
            GetFeaturedPostsCommand, GetLatestPostsCommand, GetPostCommand, NewPostCommand,
            PostNewAnynymouseCommentCommand, PostNewCommentCommand, PublishCommand,
            PushNewLikeCommand, PushNewViewCommand, SearchPostCommand, UpdatePostCommand,
        },
        services::post::PostService,
    },
    domain::{
        entities::post::{
            CategoryResult, Comment, Post, PostDetails, PostSeries, PostSnapshot, PostStats,
            PostSummary,
        },
        errors::post::PostError,
    },
};

macro_rules! set_opt {
    ($fields:expr, $( ($str: expr, $opt:expr) ),* ) => {
        $(
            $opt.is_some().then(|| $fields.push(format!("{} = ?", $str)));
        )*
    };
}

macro_rules! bind_opt {
    ($query:expr, $( $opt:expr ),* ) => {
        $(
            if let Some(val) = $opt {
                $query = $query.bind(val);
            }
        )*
    };
}

pub struct PostServiceImpl {
    pub pool: SqlitePool,
}

impl PostServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, FromRow)]
pub struct PostRow {
    pub post_id: i64,
    pub title: String,
    pub slug: String,
    pub excerpt: String,
    pub author_name: String,
    pub author_slug: String,
    pub url: Option<String>,
    pub status: String,
    pub views: i64,
    pub likes: i64,
    pub comments_count: i64,
}

impl PostRow {
    pub fn into_snapshot(self, tag_names: Vec<String>, tag_slugs: Vec<String>) -> PostSnapshot {
        PostSnapshot {
            id: self.post_id,
            title: self.title,
            slug: self.slug,
            tag_names,
            tag_slugs,
            excerpt: self.excerpt,
            author_name: self.author_name,
            author_slug: self.author_slug,
            status: self.status,
            url: self.url,
            stats: PostStats {
                likes: self.likes,
                views: self.views,
                comments: self.comments_count,
            },
        }
    }
}

#[derive(Debug, FromRow)]
pub struct PostContentRow {
    pub post_id: i64,
    pub author_name: String,
    pub author_slug: String,
    pub author_avatar_url: Option<String>,
    pub title: String,
    pub excerpt: String,
    pub content: String,
    pub draft: String,
    pub published_at: Option<String>,
    pub updated_at: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, FromRow)]
pub struct PostSearchRow {
    pub title: String,
    pub slug: String,
    pub cover_image_url: Option<String>,
    pub score: i32,
}

#[derive(Debug, FromRow)]
pub struct TagRow {
    pub post_id: i64,
    pub tag_name: String,
    pub tag_slug: String,
}

#[derive(Debug, FromRow)]
pub struct MediumUsageRow {
    pub code: i64,
    pub url: String,
}

#[derive(Debug, FromRow)]
pub struct MediumUsageWithNameRow {
    pub code: i64,
    pub url: String,
    pub short_name: String,
}

#[derive(Debug, FromRow)]
pub struct PostDetailsRow {
    pub post_id: i64,
    pub title: String,
    pub slug: String,
    pub excerpt: String,
    pub series_id: Option<i64>,
    pub draft: String,
    pub content: String,
    pub user_id: i64,
    pub is_featured: i64,
    pub cover_image_id: Option<i64>,
}

impl PostServiceImpl {
    async fn get_posts(
        &self,
        is_public: bool,
        featured: Option<i64>,
        limit: i64,
        offset: i64,
        order_by: String,
    ) -> Result<Vec<PostSnapshot>, PostError> {
        let mut placeholder: Vec<String> = vec![];

        if is_public {
            placeholder.push("status = 'published'".to_string());
        }

        if let Some(feature) = featured {
            placeholder.push(format!("is_featured = {}", feature));
        }

        let mut placeholder = placeholder.join(" AND ");
        if !placeholder.is_empty() {
            placeholder.insert_str(0, "WHERE ");
        }

        let order_by = match order_by.as_str() {
            "created" => "created_at",
            "updated" => "updated_at",
            _ => "created_at",
        };

        let sequel = format!(
            r#"
            SELECT p.id AS post_id, title, slug, excerpt, username AS author_slug, display_name AS author_name, 'media/i/' || m.short_name AS url, status, views, likes, comments_count
            FROM posts p
                JOIN users u ON u.id = p.user_id
                JOIN user_meta um ON um.user_id = p.user_id
                JOIN post_stats ps ON ps.post_id = p.id
                LEFT JOIN media m ON m.id = p.cover_image_id
            {}
            ORDER BY p.{} DESC
            LIMIT ?
            OFFSET ?
            "#,
            placeholder, order_by
        );

        let post_rows = sqlx::query_as::<_, PostRow>(&sequel)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        if post_rows.len() == 0 {
            return Ok(vec![]);
        }

        let placeholder = post_rows
            .iter()
            .map(|_| "?".to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let sequel = format!(
            r#"
            SELECT post_id, tag_id, tags.name AS tag_name, tags.slug AS tag_slug
            FROM post_tags
            JOIN tags ON tags.id = post_tags.tag_id
            WHERE post_tags.post_id IN ({})
            "#,
            placeholder
        );

        let mut query = sqlx::query_as::<_, TagRow>(&sequel);

        let mut posts_map: HashMap<i64, usize> = HashMap::new();

        let mut featured_posts = vec![];

        for post_row in post_rows {
            posts_map.insert(post_row.post_id, featured_posts.len());
            query = query.bind(post_row.post_id.clone());
            featured_posts.push(post_row.into_snapshot(vec![], vec![]));
        }

        let tag_rows = query.fetch_all(&self.pool).await?;

        for tag_row in tag_rows {
            if let Some(index) = posts_map.get_mut(&tag_row.post_id) {
                if let Some(post) = featured_posts.get_mut(*index) {
                    post.tag_names.push(tag_row.tag_name);
                    post.tag_slugs.push(tag_row.tag_slug);
                }
            }
        }

        Ok(featured_posts)
    }
}

#[async_trait::async_trait]
impl PostService for PostServiceImpl {
    async fn check_slug(&self, cmd: CheckSlugCommand) -> Result<bool, PostError> {
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM posts WHERE slug = ?
            )
            "#,
        )
        .bind(&cmd.post_slug)
        .fetch_one(&self.pool)
        .await?;

        Ok(exists)
    }
    async fn get_categories(
        &self,
        cmd: GetCategoriesCommand,
    ) -> Result<Vec<CategoryResult>, PostError> {
        let results: Vec<CategoryResult> = sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT name, slug
            FROM categories
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|(name, slug)| CategoryResult { name, slug })
        .collect();

        Ok(results)
    }
    async fn new_post(&self, cmd: NewPostCommand) -> Result<i64, PostError> {
        let mut tx = self.pool.begin().await?;
        let post_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO posts (user_id, title, slug, excerpt, draft, status)
            VALUES (?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(&cmd.user_id)
        .bind(&cmd.title)
        .bind(&cmd.slug)
        .bind(&cmd.excerpt)
        .bind(&cmd.content)
        .bind("draft".to_string())
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query("INSERT INTO post_stats (post_id) VALUES (?)")
            .bind(post_id)
            .execute(&mut *tx)
            .await?;

        if !cmd.media_usage.is_empty() {
            let placeholder = cmd
                .media_usage
                .iter()
                .map(|_| "?".to_string())
                .collect::<Vec<_>>()
                .join(", ");

            let sequel = format!(
                "SELECT id, short_name FROM media WHERE short_name IN ({})",
                placeholder
            );

            let mut query = sqlx::query_as::<_, (i64, String)>(&sequel);
            for (short_name, _) in &cmd.media_usage {
                query = query.bind(short_name);
            }

            let media: Vec<(i64, String)> = query.fetch_all(&mut *tx).await?;

            // Link post id with medium ids;

            let placeholder = cmd
                .media_usage
                .iter()
                .map(|_| "(?, ?, ?)".to_string())
                .collect::<Vec<_>>()
                .join(", ");

            let sequel = format!(
                "INSERT INTO post_media_usages (post_id, medium_id, code) VALUES {}",
                placeholder
            );

            let mut query = sqlx::query(&sequel);
            for (medium_id, short_name) in media {
                let code = cmd.media_usage.get(&short_name).ok_or_else(|| {
                    PostError::UploadFailed(format!("Failed to map {}", short_name))
                })?;
                query = query.bind(post_id).bind(medium_id).bind(code);
            }

            query.execute(&mut *tx).await?;
        }

        if !cmd.tags.is_empty() {
            let placeholder = cmd
                .tags
                .iter()
                .map(|_| "?".to_string())
                .collect::<Vec<_>>()
                .join(", ");

            let sequel = format!("SELECT id FROM tags WHERE slug IN ({})", placeholder);

            let mut query = sqlx::query_as::<_, (i64,)>(&sequel);
            for slug in &cmd.tags {
                query = query.bind(slug);
            }

            let tag_ids = query.fetch_all(&mut *tx).await?;

            let tag_ids = tag_ids.iter().map(|(id,)| id).collect::<Vec<_>>();

            let placeholder = cmd
                .tags
                .iter()
                .map(|_| "(?, ?)".to_string())
                .collect::<Vec<_>>()
                .join(", ");

            let sequel = format!(
                "INSERT INTO post_tags (post_id, tag_id) VALUES {}",
                placeholder
            );

            let mut query = sqlx::query(&sequel);
            for tag_id in tag_ids {
                query = query.bind(post_id).bind(tag_id);
            }

            query.execute(&mut *tx).await?;
        }

        tx.commit().await?;
        Ok(post_id)
    }
    async fn search(&self, cmd: SearchPostCommand) -> Result<Vec<PostSummary>, PostError> {
        let rows = sqlx::query_as::<_, PostSearchRow>(
            r#"
            SELECT
                p.title,
                p.slug,
                'media/i/' || m.short_name AS cover_image_url,
                CASE
                    WHEN LOWER(p.title) = LOWER(?1) THEN 3
                    WHEN LOWER(p.title) LIKE LOWER(?1) || '%' THEN 2
                    WHEN LOWER(p.title) LIKE '%' || LOWER(?1) || '%' THEN 1
                    ELSE 0
                END AS score
            FROM posts AS p
            LEFT JOIN media AS m ON m.id = p.cover_image_id
            WHERE
                LOWER(p.title) LIKE '%' || LOWER(?1) || '%'
                OR LOWER(p.slug) LIKE '%' || LOWER(?1) || '%'
            ORDER BY score DESC, p.created_at DESC
            LIMIT ?2 OFFSET ?3;
            "#,
        )
        .bind(&cmd.term)
        .bind(cmd.size)
        .bind(cmd.offset)
        .fetch_all(&self.pool)
        .await?;

        let summaries = rows
            .into_iter()
            .map(
                |PostSearchRow {
                     title,
                     slug,
                     cover_image_url,
                     score: _,
                 }| PostSummary {
                    title,
                    slug,
                    cover_url: cover_image_url,
                },
            )
            .collect::<Vec<_>>();

        Ok(summaries)
    }
    async fn update_post(&self, cmd: UpdatePostCommand) -> Result<(), PostError> {
        let mut tx = self.pool.begin().await?;

        let mut set_fields: Vec<String> = vec![];
        set_opt!(
            set_fields,
            ("title", cmd.title),
            ("slug", cmd.slug),
            ("excerpt", cmd.excerpt),
            ("content", cmd.content),
            ("draft", cmd.draft)
        );

        if !set_fields.is_empty() {
            let set_stn = set_fields.join(", ");

            let sql = format!(
                r#"
                UPDATE posts
                SET {}
                WHERE id = ?
                "#,
                set_stn
            );
            let mut query = sqlx::query(&sql);

            bind_opt!(
                query,
                cmd.title,
                cmd.slug,
                cmd.excerpt,
                cmd.content,
                cmd.draft
            );

            query = query.bind(&cmd.post_id);

            query.execute(&mut *tx).await?;
        }
        if let Some(media_usage) = &cmd.media_usage {
            sqlx::query(
                r#"
                DELETE FROM post_media_usages
                WHERE post_id = ?
                "#,
            )
            .bind(&cmd.post_id)
            .execute(&mut *tx)
            .await?;

            if !media_usage.is_empty() {
                let placeholder = &media_usage
                    .iter()
                    .map(|_| "?".to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                let sequel = format!(
                    r#"
                    SELECT id, short_name
                    FROM media
                    WHERE short_name IN ({})
                    "#,
                    placeholder
                );

                let mut query = sqlx::query_as::<_, (i64, String)>(&sequel);
                for (short_name, _) in media_usage {
                    query = query.bind(short_name);
                }

                let media: Vec<(i64, String)> = query.fetch_all(&mut *tx).await?;

                let placeholder = media_usage
                    .iter()
                    .map(|_| "(?, ?, ?)".to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                let sequel = format!(
                    "INSERT INTO post_media_usages (post_id, medium_id, code) VALUES {}",
                    placeholder
                );

                let mut query = sqlx::query(&sequel);
                for (medium_id, short_name) in media {
                    let code = media_usage.get(&short_name).ok_or_else(|| {
                        PostError::UploadFailed(format!("Failed to map {}", short_name))
                    })?;
                    query = query.bind(&cmd.post_id).bind(medium_id).bind(code);
                }

                query.execute(&mut *tx).await?;
            }
        }
        if let Some(tags) = cmd.tags {
            sqlx::query(
                r#"
                DELETE FROM post_tags
                WHERE post_id = ?
                "#,
            )
            .bind(&cmd.post_id)
            .execute(&mut *tx)
            .await?;

            if !tags.is_empty() {
                let placeholder = tags
                    .iter()
                    .map(|_| "(?, ?)".to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                let sql = format!(
                    r#"
                    INSERT OR IGNORE INTO tags (slug, name)
                    VALUES {}
                    "#,
                    placeholder
                );
                let mut query = sqlx::query(&sql);

                for tag in &tags {
                    query = query.bind(tag).bind(tag)
                }

                query.execute(&mut *tx).await?;

                let placeholder = tags
                    .iter()
                    .map(|_| "?".to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                let sql = format!(
                    r#"
                    SELECT id FROM tags WHERE slug IN ({})
                    "#,
                    placeholder
                );

                let mut query = sqlx::query_scalar(&sql);

                for tag in &tags {
                    query = query.bind(tag);
                }

                let tag_ids: Vec<i64> = query.fetch_all(&mut *tx).await?;

                sqlx::query(
                    r#"
                    DELETE FROM post_tags
                    WHERE post_id = ?
                    "#,
                )
                .bind(&cmd.post_id)
                .execute(&mut *tx)
                .await?;

                let placeholder = tags
                    .iter()
                    .map(|_| "(?, ?)".to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                let sql = format!(
                    r#"
                    INSERT INTO post_tags (post_id, tag_id)
                    VALUES {}
                    "#,
                    placeholder
                );

                let mut query = sqlx::query(&sql);
                for tag_id in tag_ids {
                    query = query.bind(&cmd.post_id).bind(tag_id);
                }

                query.execute(&mut *tx).await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }
    async fn get_post(&self, cmd: GetPostCommand) -> Result<Post, PostError> {
        if let Some(id) = cmd.as_id {
            let res = sqlx::query(
                r#"
                SELECT 1
                FROM posts
                WHERE user_id = ? AND slug = ?
                "#,
            )
            .bind(&id)
            .bind(&cmd.slug)
            .fetch_optional(&self.pool)
            .await?;

            if res.is_none() {
                return Err(PostError::Forbidden);
            }
        }

        let PostContentRow {
            post_id,
            author_name,
            author_slug,
            author_avatar_url,
            title,
            excerpt,
            content,
            draft,
            published_at,
            updated_at,
            url,
        } = sqlx::query_as::<_, PostContentRow>(
            r#"
            SELECT posts.id AS post_id, users.username AS author_slug, user_meta.display_name AS author_name, title, excerpt, content, draft, published_at, posts.updated_at AS updated_at, 'media/i/' || m1.short_name AS url, 'media/i/' || m2.short_name AS author_avatar_url
            FROM posts
            JOIN users ON posts.user_id = users.id
            JOIN user_meta ON user_meta.user_id = users.id
            LEFT JOIN media m1 ON m1.id = posts.cover_image_id
            LEFT JOIN media m2 ON m2.id = user_meta.avatar_image_id
            WHERE posts.slug = ? AND status = 'published'
            "#,
        )
        .bind(&cmd.slug)
        .fetch_one(&self.pool)
        .await?;

        let medium_usage_rows = sqlx::query_as::<_, MediumUsageRow>(
            r#"
            SELECT code, url
            FROM post_media_usages
            JOIN media on media.id = medium_id
            WHERE post_media_usages.post_id = ?
            "#,
        )
        .bind(&post_id)
        .fetch_all(&self.pool)
        .await?;

        let len = medium_usage_rows.len();

        let mut medium_urls = vec![String::new(); len];

        let len = len as i64;

        for MediumUsageRow { code, url } in medium_usage_rows {
            if code < 0 || code > len {
                return Err(PostError::InternalError(
                    "Out of range index found".to_string(),
                ));
            }

            let index = code;

            if index >= len {
                return Err(PostError::InternalError(
                    "Oversized insertion found".to_string(),
                ));
            }

            let index = index as usize;

            medium_urls[index] = url;
        }

        let tags: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT slug
            FROM post_tags
            JOIN tags ON tags.id = post_tags.tag_id
            WHERE post_id = ?
            "#,
        )
        .bind(&post_id)
        .fetch_all(&self.pool)
        .await?;

        let series_opt = sqlx::query_as::<_, (i64, String, String, String, i64)>(
            r#"
            SELECT s.id, s.title, s.slug, 'media/i/' || m.short_name, sp.number
            FROM series_post sp
            JOIN series s ON s.id = sp.series_id
            LEFT JOIN media m ON m.id = s.cover_image_id
            WHERE sp.post_id = ?
            LIMIT 1
            "#,
        )
        .bind(&post_id)
        .fetch_optional(&self.pool)
        .await?;

        let mut post_series = None;

        if let Some((id, series_title, series_slug, series_cover_url, number)) = series_opt {
            let previous_post_opt = sqlx::query_as::<_, (String, String, Option<String>)>(
                r#"
                SELECT p.title, p.slug, 'media/i/' || m.short_name
                FROM series_post sp
                JOIN series s ON s.id = sp.series_id
                JOIN posts p ON p.id = sp.post_id
                LEFT JOIN media m ON m.id = p.cover_image_id
                WHERE s.id = ? AND sp.number < ?
                ORDER BY sp.number DESC
                LIMIT 1
                "#,
            )
            .bind(&id)
            .bind(&number)
            .fetch_optional(&self.pool)
            .await?;

            let mut previous_post: Option<PostSummary> = None;
            if let Some((title, slug, cover_url)) = previous_post_opt {
                previous_post = Some(PostSummary {
                    title,
                    slug,
                    cover_url,
                });
            }

            let next_post_opt = sqlx::query_as::<_, (String, String, Option<String>)>(
                r#"
                SELECT p.title, p.slug, 'media/i/' || m.short_name
                FROM series_post sp
                JOIN series s ON s.id = sp.series_id
                JOIN posts p ON p.id = sp.post_id
                LEFT JOIN media m ON m.id = p.cover_image_id
                WHERE s.id = ? AND sp.number > ?
                ORDER BY sp.number ASC
                LIMIT 1
                "#,
            )
            .bind(&id)
            .bind(&number)
            .fetch_optional(&self.pool)
            .await?;

            let mut next_post: Option<PostSummary> = None;
            if let Some((title, slug, cover_url)) = next_post_opt {
                next_post = Some(PostSummary {
                    title,
                    slug,
                    cover_url,
                });
            }
            post_series = Some(PostSeries {
                series_title,
                series_slug,
                series_cover_url,
                previous_post,
                next_post,
            });
        }

        Ok(Post {
            id: post_id,
            title,
            author_name,
            author_slug,
            author_avatar_url,
            tags,
            excerpt,
            content,
            draft,
            published_at,
            updated_at,
            medium_urls,
            post_series,
            cover_url: url,
        })
    }
    async fn publish(&self, cmd: PublishCommand) -> Result<(), PostError> {
        let mut tx = self.pool.begin().await?;

        let (id, draft): (i64, String) = sqlx::query_as(
            r#"
            SELECT id, draft
            FROM posts
            WHERE id = ? AND user_id = ?
            "#,
        )
        .bind(&cmd.post_id)
        .bind(&cmd.user_id)
        .fetch_optional(&mut *tx)
        .map_err(|e| PostError::InternalError(e.to_string()))
        .await?
        .ok_or(PostError::PostNotFound)?;

        sqlx::query(
            r#"
            UPDATE posts
            SET
                content = ?,
                published_at = CASE
                    WHEN status = 'draft' THEN CURRENT_TIMESTAMP
                    ELSE published_at
                END,
                updated_at = CURRENT_TIMESTAMP,
                status = 'published'
            WHERE id = ?
            "#,
        )
        .bind(draft)
        .bind(id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }
    // async fn unpublish(&self, cmd: UnpublishCommand) -> Result<(), PostError> {

    // }
    async fn get_featured_post_snapshots(
        &self,
        cmd: GetFeaturedPostsCommand,
    ) -> Result<Vec<PostSnapshot>, PostError> {
        let featured_posts = self
            .get_posts(true, Some(1), cmd.limit, 0, "created".to_string())
            .await?;

        Ok(featured_posts)
    }

    async fn get_latest_post_snapshots(
        &self,
        cmd: GetLatestPostsCommand,
    ) -> Result<Vec<PostSnapshot>, PostError> {
        let latest_posts = self
            .get_posts(true, None, cmd.limit, cmd.offset, cmd.sorted_by)
            .await?;
        Ok(latest_posts)
    }
    async fn get_post_details(
        &self,
        cmd: GetDetailedPostsCommand,
    ) -> Result<PostDetails, PostError> {
        let post_row = sqlx::query_as::<_, PostDetailsRow>(
            r#"
            SELECT
                posts.id AS post_id,
                title,
                posts.slug AS slug,
                excerpt,
                series_post.series_id AS series_id,
                content,
                draft,
                is_featured,
                cover_image_id,
                user_id
            FROM posts
            LEFT JOIN series_post ON series_post.post_id = posts.id
            WHERE id = ?;
            "#,
        )
        .bind(&cmd.post_id)
        .fetch_one(&self.pool)
        .await?;

        if let Some(user_id) = cmd.required_author_id {
            if user_id != post_row.user_id {
                return Err(PostError::Forbidden);
            }
        }

        let tag_rows = sqlx::query_as::<_, TagRow>(
            r#"
            SELECT post_id, name AS tag_name, slug AS tag_slug
            FROM post_tags
            JOIN tags ON tags.id = post_tags.tag_id
            WHERE post_id = ?
            "#,
        )
        .bind(&post_row.post_id)
        .fetch_all(&self.pool)
        .await?;

        let mut cover_url: Option<String> = None;
        if let Some(cover_id) = post_row.cover_image_id {
            cover_url = sqlx::query_scalar(
                r#"
                SELECT url
                FROM media
                WHERE id = ?
                "#,
            )
            .bind(&cover_id)
            .fetch_optional(&self.pool)
            .await?;
        }

        let mut series_slug: Option<String> = None;
        let mut series_cover_url: Option<String> = None;
        if let Some(series_id) = post_row.series_id {
            let series = sqlx::query_as::<_, (String, Option<String>)>(
                r#"
                SELECT series.slug, url
                FROM series
                LEFT JOIN media ON media.id = cover_image_id
                WHERE series.id = ?
                "#,
            )
            .bind(&series_id)
            .fetch_one(&self.pool)
            .await?;

            series_slug = Some(series.0);
            series_cover_url = series.1;
        }

        let medium_usage_rows = sqlx::query_as::<_, MediumUsageWithNameRow>(
            r#"
            SELECT code, url, short_name
            FROM post_media_usages
            JOIN media ON media.id = medium_id
            WHERE post_media_usages.post_id = ?
            "#,
        )
        .bind(&post_row.post_id)
        .fetch_all(&self.pool)
        .await?;

        let len = medium_usage_rows.len();

        let mut medium_urls = vec![String::new(); len];
        let mut medium_short_names = vec![String::new(); len];

        let len = len as i64;

        for MediumUsageWithNameRow {
            code,
            url,
            short_name,
        } in medium_usage_rows
        {
            if code < 0 || code > len {
                return Err(PostError::InternalError(
                    "Out of range index found".to_string(),
                ));
            }

            let index = code;

            if index >= len {
                return Err(PostError::InternalError(
                    "Oversized insertion found".to_string(),
                ));
            }

            let index = index as usize;

            medium_urls[index] = url;
            medium_short_names[index] = short_name;
        }

        Ok(PostDetails {
            id: post_row.post_id,
            title: post_row.title,
            slug: post_row.slug,
            tags: tag_rows
                .into_iter()
                .map(|tag_row| tag_row.tag_slug)
                .collect(),
            excerpt: post_row.excerpt,
            series_slug,
            series_cover_url,
            content: post_row.content,
            draft: post_row.draft,
            is_featured: post_row.is_featured,
            medium_urls,
            medium_short_names,
            cover_url,
            is_owner: post_row.user_id == cmd.viewing_user_id,
        })
    }
    async fn post_new_comment(&self, cmd: PostNewCommentCommand) -> Result<i64, PostError> {
        let id = sqlx::query_scalar(
            r#"
            INSERT INTO comments (post_id, user_id, content)
            VALUES (?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(&cmd.post_id)
        .bind(&cmd.user_id)
        .bind(&cmd.content)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }
    async fn post_new_anonymous_comment(
        &self,
        cmd: PostNewAnynymouseCommentCommand,
    ) -> Result<i64, PostError> {
        let id = sqlx::query_scalar(
            r#"
            INSERT INTO comments (post_id, content)
            VALUES (?, ?)
            RETURNING id
            "#,
        )
        .bind(&cmd.post_id)
        .bind(&cmd.content)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }
    async fn get_comments(&self, cmd: GetCommentsCommand) -> Result<Vec<Comment>, PostError> {
        let sequel = format!(
            r#"
            SELECT comments.id, content, comments.created_at, users.username, user_meta.display_name, media.url, users.role
            FROM comments
            LEFT JOIN users ON users.id = comments.user_id
            LEFT JOIN user_meta ON user_meta.user_id = comments.user_id
            LEFT JOIN media ON media.id = user_meta.avatar_image_id
            WHERE post_id = ? {}
            ORDER BY comments.id DESC
            LIMIT ?
            "#,
            cmd.before.map(|_| "AND comments.id < ?").unwrap_or("")
        );
        let mut query = sqlx::query_as::<
            _,
            (
                i64,
                String,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
            ),
        >(&sequel)
        .bind(&cmd.post_id);

        if let Some(before) = &cmd.before {
            query = query.bind(before)
        }

        query = query.bind(&cmd.limit);

        let comment_rows = query.fetch_all(&self.pool).await?;

        let comments = comment_rows
            .into_iter()
            .map(
                |(id, content, created_at, username, display_name, avatar_url, user_role)| {
                    Comment {
                        id,
                        content,
                        created_at,
                        username,
                        display_name,
                        avatar_url,
                        user_role,
                    }
                },
            )
            .collect();

        Ok(comments)
    }

    async fn push_new_view(&self, cmd: PushNewViewCommand) -> Result<(), PostError> {
        sqlx::query(
            r#"
            INSERT INTO post_stats (post_id, views, updated_at)
            VALUES (?, 1, CURRENT_TIMESTAMP)
            ON CONFLICT(post_id) DO UPDATE
              SET views = post_stats.views + 1,
                  updated_at = CURRENT_TIMESTAMP
        "#,
        )
        .bind(cmd.post_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn push_new_like(&self, cmd: PushNewLikeCommand) -> Result<(), PostError> {
        sqlx::query(
            r#"
            INSERT INTO post_stats (post_id, likes, updated_at)
            VALUES (?, 1, CURRENT_TIMESTAMP)
            ON CONFLICT(post_id) DO UPDATE
              SET likes = post_stats.likes + 1,
                  updated_at = CURRENT_TIMESTAMP
        "#,
        )
        .bind(cmd.post_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
