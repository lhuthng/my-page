// Comment queries.

use async_graphql::{Context, Object};
use sqlx::SqlitePool;

use super::super::rows::CommentRow;
use super::super::types::{CommentConnection, GqlComment};

#[derive(Default)]
pub struct CommentsQuery;

#[Object]
impl CommentsQuery {
    async fn comments(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
        post_id: Option<i64>,
        include_deleted: Option<bool>,
    ) -> async_graphql::Result<CommentConnection> {
        let pool = ctx.data::<SqlitePool>()?;
        let limit = limit.unwrap_or(20) as i64;
        let offset = offset.unwrap_or(0) as i64;

        let mut where_parts: Vec<String> = vec!["1=1".to_string()];

        if post_id.is_some() {
            where_parts.push("c.post_id = ?".to_string());
        }

        if !include_deleted.unwrap_or(false) {
            where_parts.push("c.is_deleted = 0".to_string());
        }

        let where_clause = where_parts.join(" AND ");

        let count_sql = format!(
            "SELECT COUNT(*) FROM comments c JOIN posts p ON p.id = c.post_id WHERE {}",
            where_clause
        );
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
        if let Some(pid) = post_id {
            count_query = count_query.bind(pid);
        }
        let total: i64 = count_query
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let data_sql = format!(
            r#"
            SELECT c.id, c.content, c.parent_id, c.is_deleted, c.created_at,
                   p.title AS post_title, p.slug AS post_slug,
                   u.username AS author_username, um.display_name AS author_name
            FROM comments c
            JOIN posts p ON p.id = c.post_id
            LEFT JOIN users u ON u.id = c.user_id
            LEFT JOIN user_meta um ON um.user_id = c.user_id
            WHERE {}
            ORDER BY c.created_at DESC
            LIMIT {} OFFSET {}
            "#,
            where_clause, limit, offset
        );
        let mut data_query = sqlx::query_as::<_, CommentRow>(&data_sql);
        if let Some(pid) = post_id {
            data_query = data_query.bind(pid);
        }
        let rows = data_query
            .fetch_all(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let items = rows
            .into_iter()
            .map(|r| GqlComment {
                id: r.id,
                content: r.content,
                post_title: r.post_title,
                post_slug: r.post_slug,
                author_name: r.author_name,
                author_username: r.author_username,
                parent_id: r.parent_id,
                is_deleted: r.is_deleted != 0,
                created_at: r.created_at,
            })
            .collect();

        Ok(CommentConnection { items, total })
    }
}
