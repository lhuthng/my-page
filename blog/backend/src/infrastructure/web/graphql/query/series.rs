// Series and series-post queries.
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
    async fn series(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> async_graphql::Result<SeriesConnection> {
        let pool = ctx.data::<SqlitePool>()?;
        let limit = limit.unwrap_or(20) as i64;
        let offset = offset.unwrap_or(0) as i64;

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM series")
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let data_sql = format!(
            r#"
            SELECT s.id, s.title, s.slug, s.description, s.created_at,
                   'media/i/' || m.short_name AS cover_url,
                   u.username AS owner_username,
                   COUNT(sp.post_id) AS post_count
            FROM series s
            LEFT JOIN series_post sp ON sp.series_id = s.id
            LEFT JOIN media m ON m.id = s.cover_image_id
            LEFT JOIN users u ON u.id = s.user_id
            GROUP BY s.id
            ORDER BY s.created_at DESC
            LIMIT {} OFFSET {}
            "#,
            limit, offset
        );

        let rows = sqlx::query_as::<_, SeriesRow>(&data_sql)
            .fetch_all(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let items = rows
            .into_iter()
            .map(|r| GqlSeries {
                id: r.id,
                title: r.title,
                slug: r.slug,
                description: r.description,
                cover_url: r.cover_url,
                owner_username: r.owner_username,
                post_count: r.post_count,
                created_at: r.created_at,
            })
            .collect();

        Ok(SeriesConnection { items, total })
    }

    async fn series_posts(
        &self,
        ctx: &Context<'_>,
        series_id: i64,
    ) -> async_graphql::Result<Vec<GqlSeriesPost>> {
        let pool = ctx.data::<SqlitePool>()?;

        let rows = sqlx::query_as::<_, SeriesPostRow>(
            r#"
            SELECT sp.post_id, p.title, p.slug, p.status, sp.number,
                   'media/i/' || m.short_name AS url
            FROM series_post sp
            JOIN posts p ON p.id = sp.post_id
            LEFT JOIN media m ON m.id = p.cover_media_id
            WHERE sp.series_id = ?
            ORDER BY sp.number ASC
            "#,
        )
        .bind(series_id)
        .fetch_all(pool)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let items = rows
            .into_iter()
            .map(|r| GqlSeriesPost {
                post_id: r.post_id,
                title: r.title,
                slug: r.slug,
                status: r.status,
                number: r.number,
                cover_url: r.url,
            })
            .collect();

        Ok(items)
    }

}
