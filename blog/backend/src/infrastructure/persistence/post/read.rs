// Post read methods on PostService: listings, search, tags, feeds.
use std::collections::HashMap;

use sqlx::Row;

use crate::application::{
    commands::post::{
        CheckSlugCommand, GetCategoriesCommand, GetFeaturedPostsCommand, GetLatestPostsCommand,
        GetPostsByTagCommand, GetRelatedPostsCommand, SearchPostCommand, SearchTagsCommand,
    },
    services::post::PostService,
};
use crate::domain::entities::post::{
    CategoryResult, PostSnapshot, PostSummary, TagSummary,
};
use crate::domain::errors::post::PostError;

use super::rows::{PostRow, PostSearchRow, TagRow, TagSummaryRow};
use super::PostServiceImpl;
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

}
