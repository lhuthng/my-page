// Tag and category queries.

use async_graphql::{Context, Object};
use sqlx::SqlitePool;

use super::super::rows::{CategoryRow, TagRow};
use super::super::types::{CategoryConnection, GqlCategory, GqlTag, TagConnection};

#[derive(Default)]
pub struct TaxonomyQuery;

#[Object]
impl TaxonomyQuery {
    async fn tags(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> async_graphql::Result<TagConnection> {
        let pool = ctx.data::<SqlitePool>()?;
        let limit = limit.unwrap_or(20) as i64;
        let offset = offset.unwrap_or(0) as i64;

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tags")
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let data_sql = format!(
            r#"
            SELECT t.id, t.name, t.slug, t.description,
                   COUNT(pt.post_id) AS post_count
            FROM tags t
            LEFT JOIN post_tags pt ON pt.tag_id = t.id
            GROUP BY t.id
            ORDER BY post_count DESC
            LIMIT {} OFFSET {}
            "#,
            limit, offset
        );

        let rows = sqlx::query_as::<_, TagRow>(&data_sql)
            .fetch_all(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let items = rows
            .into_iter()
            .map(|r| GqlTag {
                id: r.id,
                name: r.name,
                slug: r.slug,
                description: r.description,
                post_count: r.post_count,
            })
            .collect();

        Ok(TagConnection { items, total })
    }

    async fn categories(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> async_graphql::Result<CategoryConnection> {
        let pool = ctx.data::<SqlitePool>()?;
        let limit = limit.unwrap_or(20) as i64;
        let offset = offset.unwrap_or(0) as i64;

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM categories")
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let data_sql = format!(
            r#"
            SELECT c.id, c.name, c.slug, c.description,
                   COUNT(pc.post_id) AS post_count
            FROM categories c
            LEFT JOIN post_categories pc ON pc.category_id = c.id
            GROUP BY c.id
            ORDER BY post_count DESC
            LIMIT {} OFFSET {}
            "#,
            limit, offset
        );

        let rows = sqlx::query_as::<_, CategoryRow>(&data_sql)
            .fetch_all(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let items = rows
            .into_iter()
            .map(|r| GqlCategory {
                id: r.id,
                name: r.name,
                slug: r.slug,
                description: r.description,
                post_count: r.post_count,
            })
            .collect();

        Ok(CategoryConnection { items, total })
    }
}
