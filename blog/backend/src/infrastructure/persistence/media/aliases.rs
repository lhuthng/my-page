// Media alias management: add, change, delete, list.
use std::path::PathBuf;

use sqlx::Row;

use crate::application::{
    commands::media::{
        AddAliasCommand, ChangeAliasCommand, DeleteAliasCommand, GetAliasesCommand,
    },
    services::media::MediaService,
};
use crate::domain::errors::media::MediaError;

use super::MediaServiceImpl;

#[async_trait::async_trait]
impl MediaService for MediaServiceImpl {
    async fn get_aliases(&self, cmd: GetAliasesCommand) -> Result<Vec<String>, MediaError> {
        let aliases: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT alias
            FROM media_aliases
            WHERE media_id = (SELECT id FROM media WHERE short_name = ?)
            "#,
        )
        .bind(&cmd.short_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| MediaError::InternalError(e.to_string()))?;

        Ok(aliases.into_iter().map(|r| r.0).collect())
    }

    async fn add_alias(&self, cmd: AddAliasCommand) -> Result<(), MediaError> {
        sqlx::query(
            r#"
            INSERT INTO media_aliases (media_id, alias)
            VALUES ((SELECT id FROM media WHERE short_name = ?), ?)
            "#,
        )
        .bind(&cmd.short_name)
        .bind(&cmd.alias)
        .execute(&self.pool)
        .await
        .map_err(|e| MediaError::InternalError(e.to_string()))?;

        Ok(())
    }

    async fn change_alias(&self, cmd: ChangeAliasCommand) -> Result<(), MediaError> {
        sqlx::query(
            r#"
            UPDATE media_aliases
            SET alias = ?
            WHERE media_id = (SELECT id FROM media WHERE short_name = ?)
            AND alias = ?
            "#,
        )
        .bind(&cmd.new_alias)
        .bind(&cmd.short_name)
        .bind(&cmd.old_alias)
        .execute(&self.pool)
        .await
        .map_err(|e| MediaError::InternalError(e.to_string()))?;

        Ok(())
    }

    async fn delete_alias(&self, cmd: DeleteAliasCommand) -> Result<(), MediaError> {
        sqlx::query(
            r#"
            DELETE FROM media_aliases
            WHERE media_id = (SELECT id FROM media WHERE short_name = ?)
            AND alias = ?
            "#,
        )
        .bind(&cmd.short_name)
        .bind(&cmd.alias)
        .execute(&self.pool)
        .await
        .map_err(|e| MediaError::InternalError(e.to_string()))?;

        Ok(())
    }
}
