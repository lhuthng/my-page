// Media queries.

use async_graphql::{Context, Object};
use sqlx::SqlitePool;

use super::super::rows::MediaRow;
use super::super::types::{GqlMedia, MediaConnection};

#[derive(Default)]
pub struct MediaQuery;

#[Object]
impl MediaQuery {
    async fn media(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
        search: Option<String>,
    ) -> async_graphql::Result<MediaConnection> {
        let pool = ctx.data::<SqlitePool>()?;
        let limit = limit.unwrap_or(20) as i64;
        let offset = offset.unwrap_or(0) as i64;

        let mut where_parts: Vec<String> = vec!["1=1".to_string()];

        if search.is_some() {
            where_parts.push(
                "(LOWER(m.short_name) LIKE '%' || LOWER(?) || '%' OR LOWER(m.file_name) LIKE '%' || LOWER(?) || '%')".to_string()
            );
        }

        let where_clause = where_parts.join(" AND ");

        let count_sql = format!("SELECT COUNT(*) FROM media m WHERE {}", where_clause);
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
            SELECT m.id, m.short_name, m.file_name, m.file_type, m.url, m.size,
                   m.description, m.use_count, m.created_at,
                   um.display_name AS uploader_name
            FROM media m
            LEFT JOIN users u ON u.id = m.uploader_id
            LEFT JOIN user_meta um ON um.user_id = m.uploader_id
            WHERE {}
            ORDER BY m.created_at DESC
            LIMIT {} OFFSET {}
            "#,
            where_clause, limit, offset
        );
        let mut data_query = sqlx::query_as::<_, MediaRow>(&data_sql);
        if let Some(ref s) = search {
            data_query = data_query.bind(s).bind(s);
        }
        let rows = data_query
            .fetch_all(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let items = rows
            .into_iter()
            .map(|r| GqlMedia {
                id: r.id,
                short_name: r.short_name,
                file_name: r.file_name,
                file_type: r.file_type,
                url: r.url,
                size: r.size,
                description: r.description,
                use_count: r.use_count,
                created_at: r.created_at,
                uploader_name: r.uploader_name,
            })
            .collect();

        Ok(MediaConnection { items, total })
    }
}
