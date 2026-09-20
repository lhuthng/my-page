// Tag management: update and delete from the dashboard.
use std::collections::HashMap;

use sqlx::Row;

use crate::application::commands::dashboard::{DeleteTagCommand, UpdateTagCommand};
use crate::domain::errors::dashboard::DashboardError;

use super::rows::{DashboardTagRecordRow, DashTagRow};
use super::DashboardServiceImpl;

#[async_trait::async_trait]
impl DashboardService for DashboardServiceImpl {
    async fn update_tag(
        &self,
        cmd: UpdateDashboardTagCommand,
    ) -> Result<DashboardTagRecord, UserError> {
        let name = cmd.name.trim().to_string();
        if name.len() < 2 {
            return Err(UserError::InvalidData(
                "Tag name must be at least 2 characters.".to_string(),
            ));
        }

        let slug = cmd.slug.trim().to_lowercase();
        if slug.len() < 2 {
            return Err(UserError::InvalidData(
                "Tag slug must be at least 2 characters.".to_string(),
            ));
        }
        if !slug
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_')
        {
            return Err(UserError::InvalidData(
                "Tag slug may only contain lowercase letters, numbers, hyphens, and underscores."
                    .to_string(),
            ));
        }

        let description = cmd
            .description
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let duplicate: Option<i64> =
            sqlx::query_scalar("SELECT id FROM tags WHERE slug = ? AND id != ?")
                .bind(&slug)
                .bind(cmd.id)
                .fetch_optional(&self.pool)
                .await?;

        if duplicate.is_some() {
            return Err(UserError::InvalidData(
                "Another tag already uses that slug.".to_string(),
            ));
        }

        let updated = sqlx::query(
            r#"
            UPDATE tags
            SET name = ?, slug = ?, description = ?
            WHERE id = ?
            "#,
        )
        .bind(&name)
        .bind(&slug)
        .bind(&description)
        .bind(cmd.id)
        .execute(&self.pool)
        .await?;

        if updated.rows_affected() == 0 {
            return Err(UserError::NotFound);
        }

        let row = sqlx::query_as::<_, DashboardTagRecordRow>(
            r#"
            SELECT t.id, t.name, t.slug, t.description,
                   COUNT(pt.post_id) AS post_count
            FROM tags t
            LEFT JOIN post_tags pt ON pt.tag_id = t.id
            WHERE t.id = ?
            GROUP BY t.id
            "#,
        )
        .bind(cmd.id)
        .fetch_one(&self.pool)
        .await?;

        Ok(DashboardTagRecord {
            id: row.id,
            name: row.name,
            slug: row.slug,
            description: row.description,
            post_count: row.post_count,
        })
    }

    async fn delete_tag(&self, cmd: DeleteDashboardTagCommand) -> Result<(), UserError> {
        let usage_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM post_tags WHERE tag_id = ?")
            .bind(cmd.id)
            .fetch_one(&self.pool)
            .await?;

        if usage_count > 0 {
            return Err(UserError::InvalidData(
                "Only unused tags can be deleted.".to_string(),
            ));
        }

        let deleted = sqlx::query("DELETE FROM tags WHERE id = ?")
            .bind(cmd.id)
            .execute(&self.pool)
            .await?;

        if deleted.rows_affected() == 0 {
            return Err(UserError::NotFound);
        }

        Ok(())
    }
}
