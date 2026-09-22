// Dashboard listing queries.

use async_graphql::{Context, Object};
use sqlx::SqlitePool;

use super::super::helpers::{
    DASHBOARD_POST_COLUMNS, DASHBOARD_POST_JOINS, attach_tags_to_posts, attach_tags_to_projects,
};
use super::super::rows::{DashboardPostRow, DashboardProjectRow};
use super::super::types::{DashboardPostConnection, ProjectConnection};

#[derive(Default)]
pub struct DashboardQuery;

#[Object]
impl DashboardQuery {
    async fn dashboard_posts(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
        search: Option<String>,
    ) -> async_graphql::Result<DashboardPostConnection> {
        let pool = ctx.data::<SqlitePool>()?;
        let limit = limit.unwrap_or(20) as i64;
        let offset = offset.unwrap_or(0) as i64;

        let mut where_parts: Vec<String> = vec!["p.content_kind = 'post'".to_string()];
        if search.is_some() {
            where_parts.push(
                "(LOWER(p.title) LIKE '%' || LOWER(?) || '%' OR LOWER(p.slug) LIKE '%' || LOWER(?) || '%')".to_string()
            );
        }
        let where_clause = where_parts.join(" AND ");

        let count_sql = format!("SELECT COUNT(*) FROM posts p WHERE {}", where_clause);
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
        if let Some(ref s) = search {
            count_query = count_query.bind(s).bind(s);
        }
        let total: i64 = count_query
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let data_sql = format!(
            r#"SELECT {} {} WHERE {} ORDER BY p.updated_at DESC LIMIT {} OFFSET {}"#,
            DASHBOARD_POST_COLUMNS, DASHBOARD_POST_JOINS, where_clause, limit, offset
        );
        let mut data_query = sqlx::query_as::<_, DashboardPostRow>(&data_sql);
        if let Some(ref s) = search {
            data_query = data_query.bind(s).bind(s);
        }
        let rows = data_query
            .fetch_all(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let items = attach_tags_to_posts(pool, rows)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(DashboardPostConnection { items, total })
    }

    async fn dashboard_projects(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
        search: Option<String>,
    ) -> async_graphql::Result<ProjectConnection> {
        let pool = ctx.data::<SqlitePool>()?;
        let limit = limit.unwrap_or(20) as i64;
        let offset = offset.unwrap_or(0) as i64;

        let mut where_parts: Vec<String> = vec![];
        if search.is_some() {
            where_parts.push(
                "(LOWER(p.title) LIKE '%' || LOWER(?) || '%' OR LOWER(p.slug) LIKE '%' || LOWER(?) || '%')".to_string()
            );
        }
        let where_clause = if where_parts.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_parts.join(" AND "))
        };

        let count_sql = format!(
            r#"SELECT COUNT(*) FROM projects JOIN posts p ON p.id = projects.post_id {}"#,
            where_clause
        );
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
        if let Some(ref s) = search {
            count_query = count_query.bind(s).bind(s);
        }
        let total: i64 = count_query
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let data_sql = format!(
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
            {}
            ORDER BY p.updated_at DESC
            LIMIT {} OFFSET {}
            "#,
            where_clause, limit, offset
        );
        let mut data_query = sqlx::query_as::<_, DashboardProjectRow>(&data_sql);
        if let Some(ref s) = search {
            data_query = data_query.bind(s).bind(s);
        }
        let rows = data_query
            .fetch_all(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let items = attach_tags_to_projects(pool, rows)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(ProjectConnection { items, total })
    }
}
