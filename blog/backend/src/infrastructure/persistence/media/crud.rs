// Media CRUD: short-link resolution, detail fetch, detail change.
use std::path::PathBuf;

use sqlx::Row;

use crate::application::{
    commands::media::{ChangeMediaDetailsCommand, GetLinkCommand, GetMediaDetailsCommand},
    services::media::MediaService,
};
use crate::domain::entities::media::{LinkResult, MediaDetails};
use crate::domain::errors::media::MediaError;

use super::rows::MediaSearchRow;
use super::MediaServiceImpl;

#[async_trait::async_trait]
impl MediaService for MediaServiceImpl {
    async fn get_link(&self, cmd: GetLinkCommand) -> Result<LinkResult, MediaError> {
        match sqlx::query_as::<_, (String, String, String, i64)>(
            r#"
                SELECT url, file_type, hash, uploader_id FROM media WHERE short_name = ?;
            "#,
        )
        .bind(&cmd.short_name)
        .fetch_one(&self.pool)
        .await
        {
            Ok(row) => Ok(LinkResult {
                short_name: None,
                url: row.0,
                file_type: row.1,
                hash: row.2,
                uploader_id: row.3,
            }),
            Err(_) => Err(MediaError::FileNotFound),
        }
    }
    async fn get_details(
        &self,
        cmd: GetMediaDetailsCommand,
    ) -> Result<MediaDetailResult, MediaError> {
        let row: (i64, String, String, String) = sqlx::query_as(
            r#"
            SELECT id, short_name, file_type, description
            FROM media
            WHERE short_name = ?
            "#,
        )
        .bind(&cmd.short_name)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => MediaError::FileNotFound,
            other => MediaError::InternalError(other.to_string()),
        })?;

        let aliases: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT alias
            FROM media_aliases
            WHERE media_id = ?
            "#,
        )
        .bind(row.0)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| MediaError::InternalError(e.to_string()))?;

        let aliases = aliases.into_iter().map(|r| r.0).collect();

        Ok(MediaDetailResult {
            short_name: row.1,
            file_type: row.2,
            description: row.3,
            aliases,
        })
    }
    async fn change_details(&self, cmd: ChangeMediaDetailsCommand) -> Result<(), MediaError> {
        let mut tx = self.pool.begin().await?;

        if let Some(description) = cmd.description {
            sqlx::query(
                r#"
                UPDATE media
                SET description = ?
                WHERE short_name = ?
                "#,
            )
            .bind(description)
            .bind(&cmd.short_name)
            .execute(&mut *tx)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => MediaError::FileNotFound,
                other => MediaError::InternalError(other.to_string()),
            })?;
        }

        if let Some(new_short_name) = cmd.new_short_name {
            sqlx::query(
                r#"
                UPDATE media
                SET short_name = ?
                WHERE short_name = ?
                "#,
            )
            .bind(new_short_name)
            .bind(&cmd.short_name)
            .execute(&mut *tx)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => MediaError::FileNotFound,
                other => MediaError::InternalError(other.to_string()),
            })?;
        }

        tx.commit().await?;

        Ok(())
    }

}
