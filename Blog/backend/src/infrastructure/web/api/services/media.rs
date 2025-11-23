use std::str::FromStr;

use sha2::{Digest, Sha256};
use sqlx::{SqlitePool, prelude::FromRow};
use tokio::fs;

use crate::{
    application::{
        commands::media::{
            AddAliasCommand, ChangeAliasCommand, ChangeMediaDetailsCommand, DeleteAliasCommand,
            GetAliasesCommand, GetLinkCommand, GetMediaDetailsCommand, SearchMediaCommand,
            UploadMediaCommand,
        },
        services::media::MediaService,
    },
    domain::{
        entities::media::{LinkResult, MediaDetailResult, MediaType},
        errors::media::MediaError,
    },
    infrastructure::web::server::MediaConfig,
};

#[derive(FromRow, Debug)]
struct MediaSearchRow {
    pub short_name: String,
    pub url: String,
}

pub struct MediaServiceImpl {
    pub pool: SqlitePool,
}

impl MediaServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl MediaService for MediaServiceImpl {
    async fn is_supported(
        &self,
        file_type: &str,
        config: &MediaConfig,
    ) -> Result<bool, MediaError> {
        let media_type = MediaType::from_str(file_type)?;

        Ok(config.allowed_file_types.contains(&media_type))
    }
    async fn upload(
        &self,
        cmd: UploadMediaCommand,
        config: &MediaConfig,
    ) -> Result<(), MediaError> {
        if !self.is_supported(&cmd.content_type, config).await? {
            return Err(MediaError::InvalidFileType);
        }
        let media_type = MediaType::from_str(&cmd.content_type)?;
        let extension = media_type.get_extension();

        let hash = format!("{:x}", Sha256::digest(&cmd.bytes));

        if hash.len() < 4 {
            return Err(MediaError::ExposedInternalError(
                "Hash too short".to_string(),
            ));
        }
        let dir1 = &hash[0..2];
        let dir2 = &hash[2..4];

        let dir_path = config.dir.join(dir1).join(dir2);
        let h_filename = format!("{}{}", hash, extension);
        let file_path = dir_path.join(h_filename);
        let size = cmd.bytes.len() as i64;

        if fs::try_exists(&dir_path).await? {
            return Err(MediaError::Duplication);
        }

        fs::create_dir_all(&dir_path).await?;
        fs::write(&file_path, &cmd.bytes).await?;

        let mut tx = self.pool.begin().await?;

        let result = sqlx::query(
            r#"
            INSERT INTO media
            (hash, short_name, file_name, file_type, url, size, description, uploader_id)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&hash)
        .bind(&cmd.short_name)
        .bind(&cmd.filename)
        .bind(&cmd.content_type)
        .bind(&file_path.to_str().ok_or(MediaError::ExposedInternalError(
            "Failed to get file path".to_string(),
        ))?)
        .bind(size)
        .bind(&cmd.description)
        .bind(cmd.uploader_id)
        .execute(&mut *tx)
        .await;

        if let Err(e) = result {
            if let Err(remove_err) = fs::remove_file(&file_path).await {
                return Err(MediaError::ExposedInternalError(format!(
                    "Failed to remove file after DB error {}",
                    remove_err
                )));
            }
            return Err(MediaError::ExposedInternalError(e.to_string()));
        }

        tx.commit().await?;

        Ok(())
    }
    async fn get_link(&self, cmd: GetLinkCommand) -> Result<LinkResult, MediaError> {
        match sqlx::query_as::<_, (String,)>(
            r#"
                SELECT url FROM media WHERE short_name = ?;
            "#,
        )
        .bind(&cmd.short_name)
        .fetch_one(&self.pool)
        .await
        {
            Ok(row) => Ok(LinkResult {
                short_name: None,
                url: row.0,
            }),
            Err(e) => Err(MediaError::InternalError(e.to_string())),
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
        .bind(&row.0)
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
    async fn search(&self, cmd: SearchMediaCommand) -> Result<Vec<LinkResult>, MediaError> {
        let rows = sqlx::query_as::<_, MediaSearchRow>(
            r#"
            SELECT DISTINCT
                m.short_name,
                m.url,
                CASE
                    WHEN LOWER(m.short_name) = LOWER(?1) THEN 3
                    WHEN LOWER(m.short_name) LIKE LOWER(?1) || '%' THEN 2
                    WHEN LOWER(m.short_name) LIKE '%' || LOWER(?1) || '%' THEN 1
                    ELSE 0
                END AS score
            FROM media AS m
            LEFT JOIN media_aliases AS ma ON ma.media_id = m.id
            WHERE
                LOWER(m.short_name) LIKE '%' || LOWER(?1) || '%'
                OR LOWER(COALESCE(ma.alias, '')) LIKE '%' || LOWER(?1) || '%'
            ORDER BY score DESC, m.created_at DESC
            LIMIT ?2 OFFSET ?3;
            "#,
        )
        .bind(&cmd.term)
        .bind(&cmd.size)
        .bind(&cmd.skip)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| MediaError::InternalError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| LinkResult {
                short_name: Some(r.short_name),
                url: r.url,
            })
            .collect())
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
