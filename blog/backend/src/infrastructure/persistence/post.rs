use std::collections::HashMap;
use std::collections::HashSet;

use futures::TryFutureExt;
use once_cell::sync::Lazy;
use regex::Regex;
use sqlx::{SqlitePool, prelude::FromRow};

static MENTION_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"@([A-Za-z0-9_-]+)").unwrap());

use crate::{
    application::{
        commands::post::{
            CheckSlugCommand, GetCategoriesCommand, GetCommentsCommand, GetDetailedPostsCommand,
            GetFeaturedPostsCommand, GetLatestPostsCommand, GetPostCommand, GetPostsByTagCommand,
            GetRelatedPostsCommand, NewPostCommand, PostNewAnynymouseCommentCommand,
            PostNewCommentCommand, PublishCommand, PushNewLikeCommand, PushNewViewCommand,
            SearchPostCommand, SearchTagsCommand, SetFeaturedPostCommand, SetRelatedPostsCommand,
            UpdatePostCommand, UpdatePostCoverCommand,
        },
        services::post::PostService,
    },
    domain::{
        entities::post::{
            CategoryResult, Comment, CommentPage, Post, PostDetails, PostSeries, PostSnapshot,
            PostSnapshotPage, PostStats, PostSummary, TagSummary,
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

/// Upper bound on how many tags one post may carry. The tag SQL is built by
/// joining one placeholder per tag, so this also bounds the generated statement.
const MAX_TAGS_PER_POST: usize = 30;

/// Validate, deduplicate, insert-if-missing, and resolve `tags` to tag ids.
///
/// Both `new_post` and `update_post` previously built their placeholder lists
/// from the *input* tag count while binding from the *resolved* rows. Any tag
/// that did not resolve 1:1 (a tag that did not exist yet, or a duplicate in the
/// input) produced a placeholder/bind mismatch and a sqlx error. Deriving the
/// placeholders from the resolved ids keeps the two counts in step by
/// construction.
async fn resolve_tag_ids(
    tx: &mut sqlx::SqliteConnection,
    tags: &[String],
) -> Result<Vec<i64>, PostError> {
    if tags.is_empty() {
        return Ok(Vec::new());
    }
    if tags.len() > MAX_TAGS_PER_POST {
        return Err(PostError::Validation(format!(
            "A post may have at most {} tags.",
            MAX_TAGS_PER_POST
        )));
    }

    // Validate up front so a bad tag is a 400 rather than a trigger-raised 500.
    let mut unique = Vec::<String>::new();
    for tag in tags {
        let slug = crate::helper::string::validate_slug(tag).map_err(PostError::Validation)?;
        if !unique.contains(&slug) {
            unique.push(slug);
        }
    }

    let values = unique
        .iter()
        .map(|_| "(?, ?)")
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("INSERT OR IGNORE INTO tags (slug, name) VALUES {}", values);
    let mut query = sqlx::query(&sql);
    for tag in &unique {
        query = query.bind(tag).bind(tag);
    }
    query.execute(&mut *tx).await?;

    let placeholder = unique.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!("SELECT id FROM tags WHERE slug IN ({})", placeholder);
    let mut query = sqlx::query_scalar::<_, i64>(&sql);
    for tag in &unique {
        query = query.bind(tag);
    }
    Ok(query.fetch_all(&mut *tx).await?)
}

/// Replace a post's tag links with `tag_ids`.
async fn link_post_tags(
    tx: &mut sqlx::SqliteConnection,
    post_id: i64,
    tag_ids: &[i64],
) -> Result<(), PostError> {
    sqlx::query("DELETE FROM post_tags WHERE post_id = ?")
        .bind(post_id)
        .execute(&mut *tx)
        .await?;

    if tag_ids.is_empty() {
        return Ok(());
    }

    let values = tag_ids
        .iter()
        .map(|_| "(?, ?)")
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("INSERT INTO post_tags (post_id, tag_id) VALUES {}", values);
    let mut query = sqlx::query(&sql);
    for tag_id in tag_ids {
        query = query.bind(post_id).bind(tag_id);
    }
    query.execute(&mut *tx).await?;
    Ok(())
}

/// Link a post to the media rows named by `media_usage` (short_name -> code).
///
/// Placeholders are derived from the rows that actually resolved, not from the
/// requested short names, so a short name with no matching media row is skipped
/// instead of desynchronising the bind list.
async fn link_post_media(
    tx: &mut sqlx::SqliteConnection,
    post_id: i64,
    media_usage: &HashMap<String, i64>,
) -> Result<(), PostError> {
    if media_usage.is_empty() {
        return Ok(());
    }

    let placeholder = media_usage
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT id, short_name FROM media WHERE short_name IN ({})",
        placeholder
    );
    let mut query = sqlx::query_as::<_, (i64, String)>(&sql);
    for short_name in media_usage.keys() {
        query = query.bind(short_name);
    }
    let media: Vec<(i64, String)> = query.fetch_all(&mut *tx).await?;

    let resolved: Vec<(i64, i64)> = media
        .into_iter()
        .filter_map(|(medium_id, short_name)| {
            media_usage.get(&short_name).map(|code| (medium_id, *code))
        })
        .collect();

    if resolved.is_empty() {
        return Ok(());
    }

    let values = resolved
        .iter()
        .map(|_| "(?, ?, ?)")
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "INSERT INTO post_media_usages (post_id, medium_id, code) VALUES {}",
        values
    );
    let mut query = sqlx::query(&sql);
    for (medium_id, code) in &resolved {
        query = query.bind(post_id).bind(medium_id).bind(code);
    }
    query.execute(&mut *tx).await?;
    Ok(())
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
    pub cover_media_type: Option<String>,
    pub status: String,
    pub views: i64,
    pub likes: i64,
    pub comments_count: i64,
    pub reading_time_minutes: i64,
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
            cover_media_type: self.cover_media_type,
            stats: PostStats {
                likes: self.likes,
                views: self.views,
                comments: self.comments_count,
            },
            reading_time_minutes: self.reading_time_minutes,
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
    pub cover_media_type: Option<String>,
    pub cover_video_url: Option<String>,
    pub cover_video_type: Option<String>,
    pub og_image_seconds: i64,
    pub reading_time_minutes: i64,
}

#[derive(Debug, FromRow)]
pub struct PostSearchRow {
    pub title: String,
    pub slug: String,
    pub cover_image_url: Option<String>,
    #[allow(dead_code)]
    pub score: i32,
}

#[derive(Debug, FromRow)]
pub struct TagRow {
    pub post_id: i64,
    pub tag_name: String,
    pub tag_slug: String,
}

#[derive(Debug, FromRow)]
pub struct TagSummaryRow {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub post_count: i64,
    #[allow(dead_code)]
    pub score: i32,
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
    pub updated_at: Option<String>,
    pub title: String,
    pub slug: String,
    pub excerpt: String,
    pub series_id: Option<i64>,
    pub draft: String,
    pub content: String,
    pub user_id: i64,
    pub is_featured: i64,
    pub cover_url: Option<String>,
    pub cover_media_type: Option<String>,
    pub og_image_seconds: i64,
}

impl PostServiceImpl {
    async fn hydrate_post_rows(
        &self,
        post_rows: Vec<PostRow>,
    ) -> Result<Vec<PostSnapshot>, PostError> {
        if post_rows.is_empty() {
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
        let mut snapshots = vec![];

        for post_row in post_rows {
            posts_map.insert(post_row.post_id, snapshots.len());
            query = query.bind(post_row.post_id);
            snapshots.push(post_row.into_snapshot(vec![], vec![]));
        }

        let tag_rows = query.fetch_all(&self.pool).await?;

        for tag_row in tag_rows {
            if let Some(index) = posts_map.get(&tag_row.post_id)
                && let Some(post) = snapshots.get_mut(*index)
            {
                post.tag_names.push(tag_row.tag_name);
                post.tag_slugs.push(tag_row.tag_slug);
            }
        }

        Ok(snapshots)
    }

    async fn get_posts(
        &self,
        is_public: bool,
        featured: Option<i64>,
        limit: i64,
        offset: i64,
        order_by: String,
    ) -> Result<Vec<PostSnapshot>, PostError> {
        let mut placeholder: Vec<String> = vec![];

        placeholder.push("content_kind = 'post'".to_string());

        if is_public {
            placeholder.push("status = 'published' AND deleted_at IS NULL".to_string());
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
            SELECT p.id AS post_id, title, slug, excerpt, username AS author_slug, display_name AS author_name, 'media/i/' || m.short_name AS url, m.file_type AS cover_media_type, status, views, likes, comments_count, reading_time_minutes
            FROM posts p
                JOIN users u ON u.id = p.user_id
                JOIN user_meta um ON um.user_id = p.user_id
                JOIN post_stats ps ON ps.post_id = p.id
                LEFT JOIN media m ON m.id = p.cover_media_id
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

        self.hydrate_post_rows(post_rows).await
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
        _cmd: GetCategoriesCommand,
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
        let title = crate::helper::string::validate_text(&cmd.title, "Title", 200)
            .map_err(PostError::Validation)?;
        let slug =
            crate::helper::string::validate_slug(&cmd.slug).map_err(PostError::Validation)?;
        let excerpt = crate::helper::string::validate_text(&cmd.excerpt, "Excerpt", 400)
            .map_err(PostError::Validation)?;
        let content = crate::helper::string::validate_body(&cmd.content, "Content")
            .map_err(PostError::Validation)?;

        let mut tx = self.pool.begin().await?;
        let reading_time_minutes =
            crate::helper::reading_time::estimate_reading_time_minutes(&content);
        let post_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO posts (user_id, title, slug, excerpt, draft, status, content_kind, reading_time_minutes)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(cmd.user_id)
        .bind(&title)
        .bind(&slug)
        .bind(&excerpt)
        .bind(&content)
        .bind("draft".to_string())
        .bind(&cmd.content_kind)
        .bind(reading_time_minutes)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query("INSERT INTO post_stats (post_id) VALUES (?)")
            .bind(post_id)
            .execute(&mut *tx)
            .await?;

        link_post_media(&mut tx, post_id, &cmd.media_usage).await?;

        let tag_ids = resolve_tag_ids(&mut tx, &cmd.tags).await?;
        link_post_tags(&mut tx, post_id, &tag_ids).await?;

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
            LEFT JOIN media AS m ON m.id = p.cover_media_id
            WHERE p.content_kind = 'post'
                AND (
                    LOWER(p.title) LIKE '%' || LOWER(?1) || '%'
                    OR LOWER(p.slug) LIKE '%' || LOWER(?1) || '%'
                )
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
    async fn search_tags(&self, cmd: SearchTagsCommand) -> Result<Vec<TagSummary>, PostError> {
        let rows = sqlx::query_as::<_, TagSummaryRow>(
            r#"
            SELECT
                t.name,
                t.slug,
                t.description,
                COUNT(DISTINCT p.id) AS post_count,
                CASE
                    WHEN ?1 IS NULL THEN 0
                    WHEN LOWER(t.slug) = LOWER(?1) THEN 3
                    WHEN LOWER(t.slug) LIKE LOWER(?1) || '%' THEN 2
                    ELSE 1
                END AS score
            FROM tags t
            LEFT JOIN post_tags pt ON pt.tag_id = t.id
            LEFT JOIN posts p ON p.id = pt.post_id AND p.status = 'published' AND p.deleted_at IS NULL
            WHERE
                ?1 IS NULL
                OR LOWER(t.slug) LIKE '%' || LOWER(?1) || '%'
            GROUP BY t.id, t.name, t.slug, t.description
            HAVING COUNT(DISTINCT p.id) > 0
            ORDER BY score DESC, post_count DESC, t.name ASC
            LIMIT ?2 OFFSET ?3
            "#,
        )
        .bind(cmd.term)
        .bind(cmd.size)
        .bind(cmd.offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| TagSummary {
                name: row.name,
                slug: row.slug,
                description: row.description,
                post_count: row.post_count,
            })
            .collect())
    }
    async fn get_posts_by_tag(
        &self,
        cmd: GetPostsByTagCommand,
    ) -> Result<(TagSummary, Vec<PostSnapshot>), PostError> {
        let tag = sqlx::query_as::<_, TagSummaryRow>(
            r#"
            SELECT
                t.name,
                t.slug,
                t.description,
                COUNT(DISTINCT p.id) AS post_count,
                0 AS score
            FROM tags t
            LEFT JOIN post_tags pt ON pt.tag_id = t.id
            LEFT JOIN posts p ON p.id = pt.post_id AND p.status = 'published' AND p.deleted_at IS NULL
            WHERE t.slug = ?1
            GROUP BY t.id, t.name, t.slug, t.description
            "#,
        )
        .bind(&cmd.slug)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(PostError::TagNotFound)?;

        let post_rows = sqlx::query_as::<_, PostRow>(
            r#"
            SELECT
                p.id AS post_id,
                title,
                slug,
                excerpt,
                username AS author_slug,
                display_name AS author_name,
                'media/i/' || m.short_name AS url,
                m.file_type AS cover_media_type,
                status,
                views,
                likes,
                comments_count,
                reading_time_minutes
            FROM posts p
                JOIN users u ON u.id = p.user_id
                JOIN user_meta um ON um.user_id = p.user_id
                JOIN post_stats ps ON ps.post_id = p.id
                LEFT JOIN media m ON m.id = p.cover_media_id
            WHERE p.status = 'published' AND p.deleted_at IS NULL
                AND p.content_kind = 'post'
                AND EXISTS (
                    SELECT 1
                    FROM post_tags pt
                    JOIN tags t ON t.id = pt.tag_id
                    WHERE pt.post_id = p.id AND t.slug = ?1
                )
            ORDER BY p.created_at DESC
            LIMIT ?2 OFFSET ?3
            "#,
        )
        .bind(&cmd.slug)
        .bind(cmd.limit)
        .bind(cmd.offset)
        .fetch_all(&self.pool)
        .await?;

        let posts = self.hydrate_post_rows(post_rows).await?;

        Ok((
            TagSummary {
                name: tag.name,
                slug: tag.slug,
                description: tag.description,
                post_count: tag.post_count,
            },
            posts,
        ))
    }
    async fn update_post(&self, cmd: UpdatePostCommand) -> Result<String, PostError> {
        use crate::{application::commands::post::UpdatePostCommand as C, helper::string::*};
        let cmd = C {
            title: cmd
                .title
                .map(|v| validate_text(&v, "Title", 200).map_err(PostError::Validation))
                .transpose()?,
            slug: cmd
                .slug
                .map(|v| validate_slug(&v).map_err(PostError::Validation))
                .transpose()?,
            excerpt: cmd
                .excerpt
                .map(|v| validate_text(&v, "Excerpt", 400).map_err(PostError::Validation))
                .transpose()?,
            content: cmd
                .content
                .map(|v| validate_body(&v, "Content").map_err(PostError::Validation))
                .transpose()?,
            draft: cmd
                .draft
                .map(|v| validate_body(&v, "Draft").map_err(PostError::Validation))
                .transpose()?,
            ..cmd
        };

        let mut tx = self.pool.begin().await?;

        // Authorise before writing anything. This has to be an explicit check
        // rather than an `AND user_id = ?` predicate on the UPDATE, because a
        // patch that only changes tags or media never reaches the UPDATE at all
        // and would otherwise skip the check entirely.
        let current = sqlx::query_as::<_, (i64, Option<String>)>(
            "SELECT user_id, updated_at FROM posts WHERE id = ?",
        )
        .bind(cmd.post_id)
        .fetch_optional(&mut *tx)
        .await?;
        let (owner_id, current_updated_at) = current.ok_or(PostError::PostNotFound)?;
        if let Some(required) = cmd.required_author_id
            && required != owner_id
        {
            return Err(PostError::Forbidden);
        }

        // Optimistic lock: refuse to overwrite a row that moved under us.
        let current_updated_at = current_updated_at.unwrap_or_default();
        if let Some(expected) = &cmd.expected_updated_at
            && crate::helper::time::normalize_utc_timestamp(expected)
                != crate::helper::time::normalize_utc_timestamp(&current_updated_at)
        {
            return Err(PostError::Conflict(current_updated_at));
        }

        let mut set_fields: Vec<String> = vec![];

        let reading_time_source = cmd.content.clone().or_else(|| cmd.draft.clone());
        let reading_time_opt: Option<i64> = reading_time_source
            .map(|text| crate::helper::reading_time::estimate_reading_time_minutes(&text));

        set_opt!(
            set_fields,
            ("title", cmd.title),
            ("slug", cmd.slug),
            ("excerpt", cmd.excerpt),
            ("content", cmd.content),
            ("draft", cmd.draft),
            ("reading_time_minutes", reading_time_opt)
        );

        // Always bump updated_at, even for a tags-or-media-only patch: it is the
        // optimistic-lock token, so every accepted write has to move it. It
        // carries no placeholder, so appending it last keeps the positional
        // binds below aligned with the fields set_opt! pushed.
        set_fields.push("updated_at = CURRENT_TIMESTAMP".to_string());

        let set_stn = set_fields.join(", ");
        let sql = format!(
            r#"
            UPDATE posts
            SET {}
            WHERE id = ?
            RETURNING updated_at
            "#,
            set_stn
        );
        let mut query = sqlx::query_scalar::<_, String>(&sql);

        bind_opt!(
            query,
            cmd.title,
            cmd.slug,
            cmd.excerpt,
            cmd.content,
            cmd.draft,
            reading_time_opt
        );

        query = query.bind(cmd.post_id);

        let new_updated_at: String = query.fetch_one(&mut *tx).await?;
        if let Some(media_usage) = &cmd.media_usage {
            sqlx::query("DELETE FROM post_media_usages WHERE post_id = ?")
                .bind(cmd.post_id)
                .execute(&mut *tx)
                .await?;

            link_post_media(&mut tx, cmd.post_id, media_usage).await?;
        }
        if let Some(tags) = cmd.tags {
            let tag_ids = resolve_tag_ids(&mut tx, &tags).await?;
            link_post_tags(&mut tx, cmd.post_id, &tag_ids).await?;
        }
        tx.commit().await?;
        Ok(crate::helper::time::normalize_utc_timestamp(new_updated_at))
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
            .bind(id)
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
            cover_media_type,
            cover_video_url,
            cover_video_type,
            og_image_seconds,
            reading_time_minutes,
        } = sqlx::query_as::<_, PostContentRow>(
            r#"
            SELECT posts.id AS post_id, users.username AS author_slug, user_meta.display_name AS author_name, title, excerpt, content, draft, published_at, posts.updated_at AS updated_at, 'media/i/' || m1.short_name AS url, m1.file_type AS cover_media_type, 'media/i/' || video.short_name AS cover_video_url, video.file_type AS cover_video_type, posts.og_image_seconds, posts.reading_time_minutes, 'media/i/' || m2.short_name AS author_avatar_url
            FROM posts
            JOIN users ON posts.user_id = users.id
            JOIN user_meta ON user_meta.user_id = users.id
            LEFT JOIN media m1 ON m1.id = posts.cover_media_id
            LEFT JOIN media m2 ON m2.id = user_meta.avatar_image_id
            LEFT JOIN media video ON video.short_name = '.post.' || posts.id
            WHERE posts.slug = ? AND status = 'published' AND posts.deleted_at IS NULL AND posts.content_kind = 'post'
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
        .bind(post_id)
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
        .bind(post_id)
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
        .bind(post_id)
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
                LEFT JOIN media m ON m.id = p.cover_media_id
                WHERE s.id = ? AND sp.number < ? AND p.content_kind = 'post'
                ORDER BY sp.number DESC
                LIMIT 1
                "#,
            )
            .bind(id)
            .bind(number)
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
                LEFT JOIN media m ON m.id = p.cover_media_id
                WHERE s.id = ? AND sp.number > ? AND p.content_kind = 'post'
                ORDER BY sp.number ASC
                LIMIT 1
                "#,
            )
            .bind(id)
            .bind(number)
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
            cover_media_type,
            cover_video_url,
            cover_video_type,
            og_image_seconds,
            reading_time_minutes,
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
        .bind(cmd.post_id)
        .bind(cmd.user_id)
        .fetch_optional(&mut *tx)
        .map_err(|e| PostError::InternalError(e.to_string()))
        .await?
        .ok_or(PostError::PostNotFound)?;

        let reading_time_minutes =
            crate::helper::reading_time::estimate_reading_time_minutes(&draft);

        sqlx::query(
            r#"
            UPDATE posts
            SET
                content = ?,
                reading_time_minutes = ?,
                published_at = CASE
                    WHEN status = 'draft' THEN CURRENT_TIMESTAMP
                    ELSE published_at
                END,
                updated_at = CURRENT_TIMESTAMP,
                status = 'published' AND deleted_at IS NULL
            WHERE id = ?
            "#,
        )
        .bind(draft)
        .bind(reading_time_minutes)
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
    ) -> Result<PostSnapshotPage, PostError> {
        let mut latest_posts = self
            .get_posts(true, None, cmd.limit + 1, cmd.offset, cmd.sorted_by)
            .await?;
        let has_more = latest_posts.len() as i64 > cmd.limit;
        if has_more {
            latest_posts.truncate(cmd.limit as usize);
        }
        Ok(PostSnapshotPage {
            posts: latest_posts,
            has_more,
        })
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
                user_id,
                cover.url AS cover_url,
                cover.file_type AS cover_media_type,
                posts.og_image_seconds,
                posts.updated_at AS updated_at
            FROM posts
            LEFT JOIN series_post ON series_post.post_id = posts.id
            LEFT JOIN media cover ON cover.id = posts.cover_media_id
            WHERE posts.id = ?;
            "#,
        )
        .bind(cmd.post_id)
        .fetch_one(&self.pool)
        .await?;

        if let Some(user_id) = cmd.required_author_id
            && user_id != post_row.user_id
        {
            return Err(PostError::Forbidden);
        }

        let tag_rows = sqlx::query_as::<_, TagRow>(
            r#"
            SELECT post_id, name AS tag_name, slug AS tag_slug
            FROM post_tags
            JOIN tags ON tags.id = post_tags.tag_id
            WHERE post_id = ?
            "#,
        )
        .bind(post_row.post_id)
        .fetch_all(&self.pool)
        .await?;

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
            .bind(series_id)
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
        .bind(post_row.post_id)
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

        let cover_url = post_row.cover_url;
        let cover_media_type = post_row.cover_media_type;
        let og_image_url = Some(format!("media/i/.post.{}.thumbnail", post_row.post_id));

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
            cover_media_type,
            og_image_seconds: post_row.og_image_seconds,
            is_owner: post_row.user_id == cmd.viewing_user_id,
            og_image_url,
            updated_at: crate::helper::time::normalize_optional_utc_timestamp(post_row.updated_at),
        })
    }
    async fn post_new_comment(&self, cmd: PostNewCommentCommand) -> Result<i64, PostError> {
        let content = crate::helper::string::validate_text(&cmd.content, "Comment", 2000)
            .map_err(PostError::Validation)?;
        let cmd = PostNewCommentCommand { content, ..cmd };

        let mut tx = self.pool.begin().await?;

        if let Some(parent_id) = cmd.parent_id {
            let parent_post_id: Option<i64> = sqlx::query_scalar(
                r#"
                SELECT post_id
                FROM comments
                WHERE id = ?
                "#,
            )
            .bind(parent_id)
            .fetch_optional(&mut *tx)
            .await?;

            if parent_post_id != Some(cmd.post_id) {
                return Err(PostError::PostNotFound);
            }
        }

        let id = sqlx::query_scalar(
            r#"
            INSERT INTO comments (post_id, user_id, parent_id, content, guest_identity)
            VALUES (?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(cmd.post_id)
        .bind(cmd.user_id)
        .bind(cmd.parent_id)
        .bind(&cmd.content)
        .bind(&cmd.guest_identity)
        .fetch_one(&mut *tx)
        .await?;

        if let Some(parent_id) = cmd.parent_id {
            let recipient_user_id = sqlx::query_scalar::<_, Option<i64>>(
                r#"
                SELECT user_id
                FROM comments
                WHERE id = ?
                "#,
            )
            .bind(parent_id)
            .fetch_optional(&mut *tx)
            .await?
            .flatten();

            if let Some(recipient_user_id) = recipient_user_id
                && recipient_user_id != cmd.user_id
            {
                sqlx::query(
                    r#"
                    INSERT INTO notifications (
                        recipient_user_id,
                        actor_user_id,
                        post_id,
                        comment_id,
                        type
                    )
                    VALUES (?, ?, ?, ?, 'reply')
                    "#,
                )
                .bind(recipient_user_id)
                .bind(cmd.user_id)
                .bind(cmd.post_id)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            }
        }

        // Notify any @mentioned registered users
        let mentioned: HashSet<String> = MENTION_RE
            .captures_iter(&cmd.content)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .collect();
        if !mentioned.is_empty() {
            let placeholders = mentioned.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
            let sql = format!("SELECT id FROM users WHERE username IN ({})", placeholders);
            let mut q = sqlx::query_scalar::<_, i64>(&sql);
            for username in &mentioned {
                q = q.bind(username);
            }
            let mentioned_ids: Vec<i64> = q.fetch_all(&mut *tx).await?;
            for mentioned_id in mentioned_ids {
                if mentioned_id != cmd.user_id {
                    sqlx::query(
                        r#"
                        INSERT INTO notifications (recipient_user_id, actor_user_id, post_id, comment_id, type)
                        VALUES (?, ?, ?, ?, 'mention')
                        "#,
                    )
                    .bind(mentioned_id)
                    .bind(cmd.user_id)
                    .bind(cmd.post_id)
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
                }
            }
        }

        tx.commit().await?;
        Ok(id)
    }
    async fn post_new_anonymous_comment(
        &self,
        cmd: PostNewAnynymouseCommentCommand,
    ) -> Result<i64, PostError> {
        let content = crate::helper::string::validate_text(&cmd.content, "Comment", 2000)
            .map_err(PostError::Validation)?;
        let cmd = PostNewAnynymouseCommentCommand { content, ..cmd };
        let mut tx = self.pool.begin().await?;

        if let Some(parent_id) = cmd.parent_id {
            let parent_post_id: Option<i64> = sqlx::query_scalar(
                r#"
                SELECT post_id
                FROM comments
                WHERE id = ?
                "#,
            )
            .bind(parent_id)
            .fetch_optional(&mut *tx)
            .await?;

            if parent_post_id != Some(cmd.post_id) {
                return Err(PostError::PostNotFound);
            }
        }

        let id = sqlx::query_scalar(
            r#"
            INSERT INTO comments (post_id, parent_id, content, guest_identity)
            VALUES (?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(cmd.post_id)
        .bind(cmd.parent_id)
        .bind(&cmd.content)
        .bind(&cmd.guest_identity)
        .fetch_one(&mut *tx)
        .await?;

        // Anonymous commenter CAN write @mentions; notify matched registered users
        // (actor_user_id is NULL since there is no authenticated user)
        let mentioned: HashSet<String> = MENTION_RE
            .captures_iter(&cmd.content)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .collect();
        if !mentioned.is_empty() {
            let placeholders = mentioned.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
            let sql = format!("SELECT id FROM users WHERE username IN ({})", placeholders);
            let mut q = sqlx::query_scalar::<_, i64>(&sql);
            for username in &mentioned {
                q = q.bind(username);
            }
            let mentioned_ids: Vec<i64> = q.fetch_all(&mut *tx).await?;
            for mentioned_id in mentioned_ids {
                sqlx::query(
                    r#"
                    INSERT INTO notifications (recipient_user_id, actor_user_id, post_id, comment_id, type)
                    VALUES (?, NULL, ?, ?, 'mention')
                    "#,
                )
                .bind(mentioned_id)
                .bind(cmd.post_id)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(id)
    }
    async fn get_comments(&self, cmd: GetCommentsCommand) -> Result<CommentPage, PostError> {
        if let Some(parent_id) = cmd.parent_id {
            let parent_post_id: Option<i64> = sqlx::query_scalar(
                r#"
                SELECT post_id
                FROM comments
                WHERE id = ?
                "#,
            )
            .bind(parent_id)
            .fetch_optional(&self.pool)
            .await?;

            if parent_post_id != Some(cmd.post_id) {
                return Err(PostError::PostNotFound);
            }

            let sequel = format!(
                r#"
                SELECT
                    comments.id,
                    comments.parent_id,
                    content,
                    comments.created_at,
                    users.username,
                    user_meta.display_name,
                    media.url,
                    users.role,
                    comments.guest_identity,
                    (
                        SELECT COUNT(*)
                        FROM comments AS replies
                        WHERE replies.parent_id = comments.id
                          AND replies.is_deleted = 0
                    ) AS direct_reply_count
                FROM comments
                LEFT JOIN users ON users.id = comments.user_id
                LEFT JOIN user_meta ON user_meta.user_id = comments.user_id
                LEFT JOIN media ON media.id = user_meta.avatar_image_id
                WHERE comments.post_id = ?
                  AND comments.parent_id = ?
                  AND comments.is_deleted = 0
                  {}
                ORDER BY comments.id DESC
                LIMIT ?
                "#,
                cmd.before.map(|_| "AND comments.id < ?").unwrap_or("")
            );

            let mut query = sqlx::query_as::<
                _,
                (
                    i64,
                    Option<i64>,
                    String,
                    String,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                    i64,
                ),
            >(&sequel)
            .bind(cmd.post_id)
            .bind(parent_id);

            if let Some(before) = &cmd.before {
                query = query.bind(before)
            }

            query = query.bind(cmd.limit + 1);

            let mut comment_rows = query.fetch_all(&self.pool).await?;
            let has_more = comment_rows.len() as i64 > cmd.limit;
            if has_more {
                comment_rows.truncate(cmd.limit as usize);
            }

            let comments = comment_rows
                .into_iter()
                .map(
                    |(
                        id,
                        parent_id,
                        content,
                        created_at,
                        username,
                        display_name,
                        avatar_url,
                        user_role,
                        guest_identity,
                        direct_reply_count,
                    )| {
                        let (username, display_name, avatar_url, user_role) =
                            if guest_identity.is_some() {
                                (None, None, None, None)
                            } else {
                                (username, display_name, avatar_url, user_role)
                            };
                        Comment {
                            id,
                            parent_id,
                            direct_reply_count: Some(direct_reply_count),
                            content,
                            created_at,
                            username,
                            display_name,
                            avatar_url,
                            user_role,
                            guest_identity,
                        }
                    },
                )
                .collect();

            return Ok(CommentPage { comments, has_more });
        }

        let sequel = format!(
            r#"
            SELECT
                comments.id,
                comments.parent_id,
                comments.content,
                comments.created_at,
                users.username,
                user_meta.display_name,
                media.url,
                users.role,
                comments.guest_identity,
                (
                    SELECT COUNT(*)
                    FROM comments AS replies
                    WHERE replies.parent_id = comments.id
                      AND replies.is_deleted = 0
                ) AS direct_reply_count
            FROM comments
            LEFT JOIN users ON users.id = comments.user_id
            LEFT JOIN user_meta ON user_meta.user_id = comments.user_id
            LEFT JOIN media ON media.id = user_meta.avatar_image_id
            WHERE comments.post_id = ?
              AND comments.parent_id IS NULL
              AND comments.is_deleted = 0
              {}
            ORDER BY comments.id DESC
            LIMIT ?
            "#,
            cmd.before.map(|_| "AND comments.id < ?").unwrap_or("")
        );

        let mut query = sqlx::query_as::<
            _,
            (
                i64,
                Option<i64>,
                String,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                i64,
            ),
        >(&sequel)
        .bind(cmd.post_id);

        if let Some(before) = &cmd.before {
            query = query.bind(before)
        }

        query = query.bind(cmd.limit + 1);

        let mut comment_rows = query.fetch_all(&self.pool).await?;
        let has_more = comment_rows.len() as i64 > cmd.limit;
        if has_more {
            comment_rows.truncate(cmd.limit as usize);
        }

        let comments = comment_rows
            .into_iter()
            .map(
                |(
                    id,
                    parent_id,
                    content,
                    created_at,
                    username,
                    display_name,
                    avatar_url,
                    user_role,
                    guest_identity,
                    direct_reply_count,
                )| {
                    let (username, display_name, avatar_url, user_role) =
                        if guest_identity.is_some() {
                            (None, None, None, None)
                        } else {
                            (username, display_name, avatar_url, user_role)
                        };
                    Comment {
                        id,
                        parent_id,
                        direct_reply_count: Some(direct_reply_count),
                        content,
                        created_at,
                        username,
                        display_name,
                        avatar_url,
                        user_role,
                        guest_identity,
                    }
                },
            )
            .collect();

        Ok(CommentPage { comments, has_more })
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

    async fn get_related_posts(
        &self,
        cmd: GetRelatedPostsCommand,
    ) -> Result<Vec<PostSummary>, PostError> {
        let rows = sqlx::query_as::<_, (String, String, Option<String>)>(
            r#"
            SELECT p.title, p.slug, 'media/i/' || m.short_name AS cover_url
            FROM related_posts rp
            JOIN posts p ON rp.related_post_id = p.id
            LEFT JOIN media m ON m.id = p.cover_media_id
            WHERE rp.post_id = ? AND p.status = 'published' AND p.deleted_at IS NULL
            ORDER BY rp.display_order ASC
            "#,
        )
        .bind(cmd.post_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(title, slug, cover_url)| PostSummary {
                title,
                slug,
                cover_url,
            })
            .collect())
    }

    async fn set_related_posts(&self, cmd: SetRelatedPostsCommand) -> Result<(), PostError> {
        let mut tx = self.pool.begin().await?;

        let owner_id: Option<i64> = sqlx::query_scalar("SELECT user_id FROM posts WHERE id = ?")
            .bind(cmd.post_id)
            .fetch_optional(&mut *tx)
            .await?;
        if owner_id.ok_or(PostError::PostNotFound)? != cmd.user_id {
            return Err(PostError::Forbidden);
        }

        sqlx::query("DELETE FROM related_posts WHERE post_id = ?")
            .bind(cmd.post_id)
            .execute(&mut *tx)
            .await?;

        for (order, slug) in cmd.related_post_slugs.iter().enumerate() {
            sqlx::query(
                "INSERT OR IGNORE INTO related_posts (post_id, related_post_id, display_order) \
                 SELECT ?, id, ? FROM posts WHERE slug = ? LIMIT 1",
            )
            .bind(cmd.post_id)
            .bind(order as i64)
            .bind(slug)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn set_post_featured(&self, cmd: SetFeaturedPostCommand) -> Result<(), PostError> {
        let is_featured_val = if cmd.is_featured { 1 } else { 0 };
        sqlx::query(
            r#"
            UPDATE posts
            SET is_featured = ?
            WHERE id = ?
            "#,
        )
        .bind(is_featured_val)
        .bind(cmd.post_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_post_cover(&self, cmd: UpdatePostCoverCommand) -> Result<(), PostError> {
        let mut set_fields: Vec<String> = vec![];
        if cmd.og_image_seconds.is_some() {
            set_fields.push("og_image_seconds = ?".to_string());
        }
        if !set_fields.is_empty() {
            let sql = format!(
                "UPDATE posts SET {} WHERE id = ? AND user_id = ?",
                set_fields.join(", ")
            );
            let mut query = sqlx::query(&sql);
            if let Some(seconds) = cmd.og_image_seconds {
                query = query.bind(seconds);
            }
            query = query.bind(cmd.post_id).bind(cmd.user_id);
            query.execute(&self.pool).await?;
        }
        Ok(())
    }
}
