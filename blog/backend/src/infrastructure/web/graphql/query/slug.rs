// Slug availability checks.

use async_graphql::{Context, Object};
use sqlx::SqlitePool;

#[derive(Default)]
pub struct SlugQuery;

#[Object]
impl SlugQuery {
    async fn check_slug(&self, ctx: &Context<'_>, slug: String) -> async_graphql::Result<bool> {
        let pool = ctx.data::<SqlitePool>()?;

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE slug = ?")
            .bind(&slug)
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(count == 0)
    }

    async fn check_project_slug(
        &self,
        ctx: &Context<'_>,
        slug: String,
    ) -> async_graphql::Result<bool> {
        let pool = ctx.data::<SqlitePool>()?;

        let count: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM posts p JOIN projects ON projects.post_id = p.id WHERE p.slug = ?"#,
        )
        .bind(&slug)
        .fetch_one(pool).await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(count == 0)
    }
}
