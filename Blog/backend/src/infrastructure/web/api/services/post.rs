use futures::TryFutureExt;
use sqlx::{Row, SqlitePool};

use crate::{
    application::{
        commands::post::{CheckSlugCommand, GetCategoriesCommand, PostCommand, PublishCommand},
        services::post::PostService,
    },
    domain::errors::post::PostError,
};

pub struct PostServiceImpl {
    pub pool: SqlitePool,
}

impl PostServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl PostService for PostServiceImpl {
    async fn check_slug(&self, cmd: CheckSlugCommand) -> Result<bool, PostError> {
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM posts WHERE slug = ?
            )
            "#,
        )
        .bind(&cmd.post_slug)
        .fetch_one(&self.pool)
        .await?;

        Ok(exists)
    }
    async fn get_categories(
        &self,
        cmd: GetCategoriesCommand,
    ) -> Result<Vec<(String, String)>, PostError> {
        let slugs: Vec<(String, String)> =
            sqlx::query_as::<_, (String, String)>("SELECT name, slug FROM categories")
                .fetch_all(&self.pool)
                .await?;

        Ok(slugs)
    }
    async fn post(&self, cmd: PostCommand) -> Result<(), PostError> {
        println!("Hello??? {:?}", cmd.categories);
        let mut tx = self.pool.begin().await?;
        let post_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO posts (user_id, title, slug, draft, status)
            VALUES (?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(&cmd.user_id)
        .bind(&cmd.title)
        .bind(&cmd.slug)
        .bind(&cmd.content)
        .bind("draft".to_string())
        .fetch_one(&mut *tx)
        .await?;

        if !cmd.media_usage.is_empty() {
            let placeholder = cmd
                .media_usage
                .iter()
                .map(|_| "?".to_string())
                .collect::<Vec<_>>()
                .join(", ");

            let sequel = format!(
                "SELECT id, short_name FROM media WHERE short_name IN ({})",
                placeholder
            );

            let mut query = sqlx::query_as::<_, (i64, String)>(&sequel);
            for (short_name, _) in &cmd.media_usage {
                query = query.bind(short_name);
            }

            let media: Vec<(i64, String)> = query.fetch_all(&mut *tx).await?;

            // Link post id with medium ids;

            let placeholder = cmd
                .media_usage
                .iter()
                .map(|_| "(?, ?, ?)".to_string())
                .collect::<Vec<_>>()
                .join(", ");

            let sequel = format!(
                "INSERT INTO post_media_usages (post_id, medium_id, code) VALUES {}",
                placeholder
            );

            let mut query = sqlx::query(&sequel);
            for (medium_id, short_name) in media {
                let code = cmd.media_usage.get(&short_name).ok_or_else(|| {
                    PostError::UploadFailed(format!("Failed to map {}", short_name))
                })?;
                query = query.bind(post_id).bind(medium_id).bind(code);
            }

            query.execute(&mut *tx).await?;
        }

        if !cmd.categories.is_empty() {
            let placeholder = cmd
                .categories
                .iter()
                .map(|_| "?".to_string())
                .collect::<Vec<_>>()
                .join(", ");

            let sequel = format!("SELECT id FROM categories WHERE slug IN ({})", placeholder);

            let mut query = sqlx::query_as::<_, (i64,)>(&sequel);
            for slug in &cmd.categories {
                query = query.bind(slug);
            }

            let category_ids = query.fetch_all(&mut *tx).await?;

            let category_ids = category_ids.iter().map(|(id,)| id).collect::<Vec<_>>();

            println!("{}", category_ids.len());

            let placeholder = cmd
                .categories
                .iter()
                .map(|_| "(?, ?)".to_string())
                .collect::<Vec<_>>()
                .join(", ");

            let sequel = format!(
                "INSERT INTO post_categories (post_id, category_id) VALUES {}",
                placeholder
            );

            let mut query = sqlx::query(&sequel);
            for category_id in category_ids {
                query = query.bind(post_id).bind(category_id);
            }

            query.execute(&mut *tx).await?;
        }

        tx.commit().await?;
        Ok(())
    }
    async fn publish(&self, cmd: PublishCommand) -> Result<(), PostError> {
        let mut tx = self.pool.begin().await?;

        let (id, draft): (i64, String) = sqlx::query_as(
            r#"
            SELECT id, draft
            FROM posts
            WHERE slug = ? AND user_id = ?
            "#,
        )
        .bind(&cmd.slug)
        .bind(&cmd.user_id)
        .fetch_optional(&mut *tx)
        .map_err(|e| PostError::InternalError(e.to_string()))
        .await?
        .ok_or(PostError::PostNotFound)?;

        sqlx::query(
            r#"
            UPDATE posts
            SET content = ?, status = 'published'
            WHERE id = ?
            "#,
        )
        .bind(draft)
        .bind(id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }
    // async fn unpublish(&self, cmd: UnpublishCommand) -> Result<(), PostError> {

    // }
}
