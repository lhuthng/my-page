use std::collections::HashMap;

use futures::TryFutureExt;
use sqlx::{SqlitePool, prelude::FromRow};

use crate::{
    application::{
        commands::post::{
            CheckSlugCommand, GetCategoriesCommand, GetCommentsCommand, GetDetailedPostsCommand,
            GetFeaturedPostsCommand, GetLatestPostsCommand, GetPostCommand, PostCommand,
            PostNewAnynymouseCommentCommand, PostNewCommentCommand, PublishCommand,
        },
        services::post::PostService,
    },
    domain::{
        entities::post::{CategoryResult, Comment, Post, PostDetails, PostSnapshot},
        errors::post::PostError,
    },
};

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
}

#[derive(Debug, FromRow)]
pub struct PostContentRow {
    pub post_id: i64,
    pub author_name: String,
    pub author_slug: String,
    pub author_avatar_url: Option<String>,
    pub title: String,
    pub content: String,
    pub published_at: Option<String>,
    pub url: Option<String>,
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
pub struct PostDetailsRow {
    pub post_id: i64,
    pub title: String,
    pub slug: String,
    pub excerpt: String,
    pub draft: String,
    pub content: String,
    pub user_id: i64,
    pub is_featured: i64,
    pub cover_image_id: Option<i64>,
}

impl PostServiceImpl {
    async fn get_posts(
        &self,
        featured: Option<i64>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PostSnapshot>, PostError> {
        let sequel = format!(
            r#"
            SELECT posts.id AS post_id, title, slug, excerpt, username AS author_slug, display_name AS author_name, url
            FROM posts
                JOIN users ON posts.user_id = users.id
                JOIN user_meta ON user_meta.user_id = posts.user_id
                LEFT JOIN media ON posts.cover_image_id = media.id
            {}
            ORDER BY posts.updated_at DESC
            LIMIT ? OFFSET ?
            "#,
            featured
                .map(|v| format!("WHERE is_featured = {}", v))
                .unwrap_or_default()
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
            featured_posts.push(PostSnapshot {
                id: post_row.post_id,
                title: post_row.title,
                slug: post_row.slug,
                excerpt: post_row.excerpt,
                author_name: post_row.author_name,
                author_slug: post_row.author_slug,
                tag_names: vec![],
                tag_slugs: vec![],
                url: post_row.url,
            });
            query = query.bind(post_row.post_id);
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
        let results: Vec<CategoryResult> =
            sqlx::query_as::<_, (String, String)>("SELECT name, slug FROM categories")
                .fetch_all(&self.pool)
                .await?
                .into_iter()
                .map(|(name, slug)| CategoryResult { name, slug })
                .collect();

        Ok(results)
    }
    async fn post(&self, cmd: PostCommand) -> Result<(), PostError> {
        let mut tx = self.pool.begin().await?;
        let post_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO posts (user_id, title, slug, draft, status)
            VALUES (?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(&cmd.user_id)
        .bind(&cmd.title)
        .bind(&cmd.slug)
        .bind(&cmd.content)
        .bind("draft".to_string())
        .fetch_one(&mut *tx)
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

        if !cmd.categories.is_empty() {
            let placeholder = cmd
                .categories
                .iter()
                .map(|_| "?".to_string())
                .collect::<Vec<_>>()
                .join(", ");

            let sequel = format!("SELECT id FROM categories WHERE slug IN ({})", placeholder);

            let mut query = sqlx::query_as::<_, (i64,)>(&sequel);
            for slug in &cmd.categories {
                query = query.bind(slug);
            }

            let category_ids = query.fetch_all(&mut *tx).await?;

            let category_ids = category_ids.iter().map(|(id,)| id).collect::<Vec<_>>();

            let placeholder = cmd
                .categories
                .iter()
                .map(|_| "(?, ?)".to_string())
                .collect::<Vec<_>>()
                .join(", ");

            let sequel = format!(
                "INSERT INTO post_categories (post_id, category_id) VALUES {}",
                placeholder
            );

            let mut query = sqlx::query(&sequel);
            for category_id in category_ids {
                query = query.bind(post_id).bind(category_id);
            }

            query.execute(&mut *tx).await?;
        }

        tx.commit().await?;
        Ok(())
    }
    async fn get_post(&self, cmd: GetPostCommand) -> Result<Post, PostError> {
        let PostContentRow {
            post_id,
            author_name,
            author_slug,
            author_avatar_url,
            title,
            content,
            published_at,
            url,
        } = sqlx::query_as::<_, PostContentRow>(
            r#"
            SELECT posts.id AS post_id, users.username AS author_slug, user_meta.display_name AS author_name, title, content, published_at, m1.url AS url, m2.url AS author_avatar_url
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

        for MediumUsageRow { code, url } in medium_usage_rows {
            if code < 0 || code > len as i64 {
                return Err(PostError::InternalError("Negative index found".to_string()));
            }

            let index = code as usize;

            if index >= len {
                return Err(PostError::InternalError(
                    "Oversized insertion found".to_string(),
                ));
            }

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

        Ok(Post {
            id: post_id,
            title,
            author_name,
            author_slug,
            author_avatar_url,
            tags,
            content,
            published_at,
            medium_urls,
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
            SET content = ?, status = 'published'
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
        let featured_posts = self.get_posts(Some(1), cmd.limit, 0).await?;

        Ok(featured_posts)
    }

    async fn get_latest_post_snapshots(
        &self,
        cmd: GetLatestPostsCommand,
    ) -> Result<Vec<PostSnapshot>, PostError> {
        let latest_posts = self.get_posts(None, cmd.limit, cmd.offset).await?;
        Ok(latest_posts)
    }
    async fn get_post_details(
        &self,
        cmd: GetDetailedPostsCommand,
    ) -> Result<PostDetails, PostError> {
        let post_row = sqlx::query_as::<_, PostDetailsRow>(
            r#"
            SELECT id AS post_id, title, slug, excerpt, content, draft, is_featured, cover_image_id, user_id
            FROM posts
            WHERE id = ?;
            "#
        ).bind(&cmd.post_id).fetch_one(&self.pool).await?;

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
                SELECT short_name
                FROM media
                WHERE id = ?
                "#,
            )
            .bind(&cover_id)
            .fetch_optional(&self.pool)
            .await?;
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
            content: post_row.content,
            draft: post_row.draft,
            is_featured: post_row.is_featured,
            cover_url,
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
}
