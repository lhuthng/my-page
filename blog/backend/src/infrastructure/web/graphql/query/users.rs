// User queries.

use async_graphql::{Context, Object};
use sqlx::SqlitePool;

use super::super::rows::UserRow;
use super::super::types::{GqlUser, UserConnection};

#[derive(Default)]
pub struct UsersQuery;

#[Object]
impl UsersQuery {
    async fn users(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
        search: Option<String>,
        role: Option<String>,
    ) -> async_graphql::Result<UserConnection> {
        let pool = ctx.data::<SqlitePool>()?;
        let limit = limit.unwrap_or(20) as i64;
        let offset = offset.unwrap_or(0) as i64;

        let mut where_parts: Vec<String> = vec!["1=1".to_string()];

        if search.is_some() {
            where_parts.push(
                "(LOWER(u.username) LIKE '%' || LOWER(?) || '%' OR LOWER(um.display_name) LIKE '%' || LOWER(?) || '%')".to_string()
            );
        }

        if role.is_some() {
            where_parts.push("u.role = ?".to_string());
        }

        let where_clause = where_parts.join(" AND ");

        let count_sql = format!(
            "SELECT COUNT(*) FROM users u JOIN user_meta um ON um.user_id = u.id WHERE {}",
            where_clause
        );
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
        if let Some(ref s) = search {
            count_query = count_query.bind(s).bind(s);
        }
        if let Some(ref r) = role {
            count_query = count_query.bind(r);
        }
        let total: i64 = count_query
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let data_sql = format!(
            r#"
            SELECT u.id, u.username, u.email, u.role, u.created_at,
                   um.display_name, um.bio, m.url AS avatar_url
            FROM users u
            JOIN user_meta um ON um.user_id = u.id
            LEFT JOIN media m ON m.id = um.avatar_image_id
            WHERE {}
            ORDER BY u.created_at DESC
            LIMIT {} OFFSET {}
            "#,
            where_clause, limit, offset
        );
        let mut data_query = sqlx::query_as::<_, UserRow>(&data_sql);
        if let Some(ref s) = search {
            data_query = data_query.bind(s).bind(s);
        }
        if let Some(ref r) = role {
            data_query = data_query.bind(r);
        }
        let rows = data_query
            .fetch_all(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let items = rows
            .into_iter()
            .map(|r| GqlUser {
                id: r.id,
                username: r.username,
                email: r.email,
                role: r.role,
                display_name: r.display_name,
                bio: r.bio,
                avatar_url: r.avatar_url,
                created_at: r.created_at,
            })
            .collect();

        Ok(UserConnection { items, total })
    }
}
