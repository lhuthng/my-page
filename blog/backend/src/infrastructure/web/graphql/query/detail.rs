// Single-item detail queries.
use std::collections::HashMap;

use async_graphql::{Context, Object};
use sqlx::SqlitePool;

use super::super::helpers::{
    DASHBOARD_POST_COLUMNS, DASHBOARD_POST_JOINS, attach_tags_to_posts, attach_tags_to_projects,
};
use super::super::rows::{
    CategoryRow, CommentRow, DashboardPostRow, DashboardProjectRow, GqlPostRow, GrowthDayRow,
    MediaRow, PostDetailRow, RoleCountRow, SeriesPostRow, SeriesRow, TagRow, UserInfoRow, UserRow,
};
use super::super::types::{
    CategoryConnection, CommentConnection, DashboardPostConnection, DbStats, GqlCategory,
    GqlComment, GqlDashboardOverview, GqlDashboardPost, GqlDashboardProject, GqlDashboardUser,
    GqlGrowthPoint, GqlMedia, GqlPost, GqlPostDetail, GqlRoleCounts, GqlSeries, GqlSeriesPost,
    GqlTag, GqlUser, MediaConnection, PostConnection, ProjectConnection, SeriesConnection,
    TagConnection, UserConnection,
};

use super::QueryRoot;

#[Object]
impl QueryRoot {
    async fn post_detail(
        &self,
        ctx: &Context<'_>,
        id: i64,
    ) -> async_graphql::Result<GqlPostDetail> {
        let pool = ctx.data::<SqlitePool>()?;

        let row = sqlx::query_as::<_, PostDetailRow>(
            r#"
            SELECT p.id, p.title, p.slug, p.excerpt, p.content, p.status,
                   p.is_featured, p.view_count, p.published_at, p.created_at, p.updated_at,
                   u.username AS author_slug, um.display_name AS author_name,
                   'media/i/' || m.short_name AS cover_url, m.file_type AS cover_media_type,
                   s.title AS series_title, s.slug AS series_slug, p.og_image_seconds
            FROM posts p
            JOIN users u ON u.id = p.user_id
            JOIN user_meta um ON um.user_id = p.user_id
            LEFT JOIN media m ON m.id = p.cover_media_id
            LEFT JOIN series s ON s.id = p.series_id
            WHERE p.id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?
        .ok_or_else(|| async_graphql::Error::new("Post not found"))?;

        let tag_rows = sqlx::query_as::<_, (String, String)>(
            r#"SELECT tags.name, tags.slug FROM post_tags JOIN tags ON tags.id = post_tags.tag_id WHERE post_tags.post_id = ?"#,
        )
        .bind(id)
        .fetch_all(pool).await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let (tag_names, tag_slugs): (Vec<String>, Vec<String>) = tag_rows.into_iter().unzip();

        let medium_rows = sqlx::query_as::<_, (String, String)>(
            r#"SELECT m.short_name, 'media/i/' || m.short_name AS url FROM post_media pm JOIN media m ON m.id = pm.media_id WHERE pm.post_id = ?"#,
        )
        .bind(id)
        .fetch_all(pool).await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let (medium_short_names, medium_urls): (Vec<String>, Vec<String>) =
            medium_rows.into_iter().unzip();

        Ok(GqlPostDetail {
            id: row.id,
            title: row.title,
            slug: row.slug,
            excerpt: row.excerpt,
            content: row.content,
            status: row.status,
            is_featured: row.is_featured != 0,
            author_name: row.author_name,
            author_slug: row.author_slug,
            tag_names,
            tag_slugs,
            cover_url: row.cover_url,
            cover_media_type: row.cover_media_type,
            series_title: row.series_title,
            series_slug: row.series_slug,
            views: row.view_count,
            likes: 0,
            comments_count: 0,
            published_at: row.published_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
            medium_urls,
            medium_short_names,
            og_image_seconds: row.og_image_seconds,
        })
    }

    async fn project_detail(
        &self,
        ctx: &Context<'_>,
        id: i64,
    ) -> async_graphql::Result<GqlDashboardProject> {
        let pool = ctx.data::<SqlitePool>()?;

        let row = sqlx::query_as::<_, DashboardProjectRow>(
            r#"
            SELECT
                projects.id AS project_id, p.id AS post_id, p.title, p.slug, p.excerpt,
                u.username AS author_slug, um.display_name AS author_name, p.status,
                'media/i/' || m.short_name AS url, m.file_type AS cover_media_type,
                projects.demo_type, ps.views, ps.likes, ps.comments_count
            FROM projects
            JOIN posts p ON p.id = projects.post_id
            JOIN users u ON u.id = p.user_id
            JOIN user_meta um ON um.user_id = p.user_id
            JOIN post_stats ps ON ps.post_id = p.id
            LEFT JOIN media m ON m.id = p.cover_media_id
            WHERE projects.id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?
        .ok_or_else(|| async_graphql::Error::new("Project not found"))?;

        let tag_rows = sqlx::query_as::<_, (String, String)>(
            r#"SELECT tags.name, tags.slug FROM post_tags JOIN tags ON tags.id = post_tags.tag_id WHERE post_tags.post_id = ?"#,
        )
        .bind(row.post_id)
        .fetch_all(pool).await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let (tag_names, tag_slugs): (Vec<String>, Vec<String>) = tag_rows.into_iter().unzip();

        Ok(GqlDashboardProject {
            id: row.project_id,
            post_id: row.post_id,
            title: row.title,
            slug: row.slug,
            excerpt: row.excerpt,
            author_name: row.author_name,
            author_slug: row.author_slug,
            tag_names,
            tag_slugs,
            status: row.status,
            cover_url: row.url,
            cover_media_type: row.cover_media_type,
            demo_type: row.demo_type,
            views: row.views,
            likes: row.likes,
            comments_count: row.comments_count,
        })
    }

    async fn related_posts(
        &self,
        ctx: &Context<'_>,
        post_id: i64,
    ) -> async_graphql::Result<Vec<GqlDashboardPost>> {
        let pool = ctx.data::<SqlitePool>()?;

        let sql = format!(
            r#"SELECT {} {} JOIN related_posts rp ON rp.related_post_id = p.id WHERE rp.post_id = ? ORDER BY p.updated_at DESC"#,
            DASHBOARD_POST_COLUMNS, DASHBOARD_POST_JOINS
        );
        let rows = sqlx::query_as::<_, DashboardPostRow>(&sql)
            .bind(post_id)
            .fetch_all(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        attach_tags_to_posts(pool, rows)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))
    }

}
