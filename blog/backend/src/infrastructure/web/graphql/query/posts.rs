// Post queries.
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
    async fn posts(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
        search: Option<String>,
        status: Option<String>,
    ) -> async_graphql::Result<PostConnection> {
        let pool = ctx.data::<SqlitePool>()?;
        let limit = limit.unwrap_or(20) as i64;
        let offset = offset.unwrap_or(0) as i64;

        let mut where_parts: Vec<String> = vec!["1=1".to_string()];

        if search.is_some() {
            where_parts.push(
                "(LOWER(p.title) LIKE '%' || LOWER(?) || '%' OR LOWER(p.slug) LIKE '%' || LOWER(?) || '%')".to_string()
            );
        }

        if status.is_some() {
            where_parts.push("p.status = ?".to_string());
        }

        let where_clause = where_parts.join(" AND ");

        let count_sql = format!("SELECT COUNT(*) FROM posts p WHERE {}", where_clause);
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
        if let Some(ref s) = search {
            count_query = count_query.bind(s).bind(s);
        }
        if let Some(ref st) = status {
            count_query = count_query.bind(st);
        }
        let total: i64 = count_query
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let data_sql = format!(
            r#"
            SELECT p.id, p.title, p.slug, p.status, p.view_count, p.is_featured,
                   p.published_at, p.created_at, p.updated_at, p.excerpt,
                   u.username AS author_slug, um.display_name AS author_name,
                   s.title AS series_title
            FROM posts p
            LEFT JOIN users u ON u.id = p.user_id
            LEFT JOIN user_meta um ON um.user_id = p.user_id
            LEFT JOIN series s ON s.id = p.series_id
            WHERE {}
            ORDER BY p.created_at DESC
            LIMIT {} OFFSET {}
            "#,
            where_clause, limit, offset
        );
        let mut data_query = sqlx::query_as::<_, GqlPostRow>(&data_sql);
        if let Some(ref s) = search {
            data_query = data_query.bind(s).bind(s);
        }
        if let Some(ref st) = status {
            data_query = data_query.bind(st);
        }
        let rows = data_query
            .fetch_all(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let items = rows
            .into_iter()
            .map(|r| GqlPost {
                id: r.id,
                title: r.title,
                slug: r.slug,
                status: r.status,
                author_name: r.author_name,
                author_slug: r.author_slug,
                series_title: r.series_title,
                view_count: r.view_count,
                is_featured: r.is_featured != 0,
                published_at: r.published_at,
                created_at: r.created_at,
                updated_at: r.updated_at,
                excerpt: r.excerpt,
            })
            .collect();

        Ok(PostConnection { items, total })
    }

}
