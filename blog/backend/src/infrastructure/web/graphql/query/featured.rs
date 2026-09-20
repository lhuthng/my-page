// Featured content queries.
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
    async fn featured_posts(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
    ) -> async_graphql::Result<Vec<GqlDashboardPost>> {
        let pool = ctx.data::<SqlitePool>()?;
        let limit = limit.unwrap_or(100) as i64;

        let sql = format!(
            r#"SELECT {} {} WHERE p.is_featured = 1 AND p.status = 'published' ORDER BY p.updated_at DESC LIMIT {}"#,
            DASHBOARD_POST_COLUMNS, DASHBOARD_POST_JOINS, limit
        );
        let rows = sqlx::query_as::<_, DashboardPostRow>(&sql)
            .fetch_all(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        attach_tags_to_posts(pool, rows)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))
    }

    async fn featured_projects(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
    ) -> async_graphql::Result<Vec<GqlDashboardProject>> {
        let pool = ctx.data::<SqlitePool>()?;
        let limit = limit.unwrap_or(100) as i64;

        let sql = format!(
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
            WHERE p.is_featured = 1 AND p.status = 'published'
            ORDER BY p.updated_at DESC
            LIMIT {}
            "#,
            limit
        );
        let rows = sqlx::query_as::<_, DashboardProjectRow>(&sql)
            .fetch_all(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        attach_tags_to_projects(pool, rows)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))
    }

}
