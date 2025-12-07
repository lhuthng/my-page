use std::{path::PathBuf, str::FromStr};

use axum::body::Bytes;
use futures::{
    TryFutureExt,
    future::{join_all, try_join_all},
};
use sha2::{Digest, Sha256};
use sqlx::{SqlitePool, prelude::FromRow};
use tokio::fs;

use crate::{
    application::{
        commands::media::{
            AddAliasCommand, ChangeAliasCommand, ChangeMediaDetailsCommand, DeleteAliasCommand,
            GetAliasesCommand, GetLinkCommand, GetMediaDetailsCommand, SearchMediaCommand,
            UploadMediaWithoutDescriptionCommand, UploadMediumCommand,
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

struct HashData {
    pub hash: String,
    pub size: i64,
    pub dir_path: PathBuf,
    pub file_path: PathBuf,
}

async fn hash_bytes(
    bytes: &Bytes,
    root: &PathBuf,
    extension: String,
) -> Result<HashData, MediaError> {
    let hash = format!("{:x}", Sha256::digest(&bytes));
    if hash.len() < 4 {
        return Err(MediaError::InternalError("Hash too short.".to_string()));
    }

    let dir1 = &hash[0..2];
    let dir2 = &hash[2..4];

    let dir_path = root.join(dir1).join(dir2);
    let file_name = format!("{}{}", hash, extension);
    let file_path = dir_path.join(file_name);
    let size = bytes.len() as i64;

    Ok(HashData {
        hash,
        size,
        dir_path,
        file_path,
    })
}

async fn clean_up_files(file_paths: &Vec<PathBuf>) -> Result<(), MediaError> {
    let futures = file_paths.iter().map(|p| fs::remove_file(p));
    let mut errors: Vec<String> = Vec::new();
    let results = join_all(futures).await;
    for result in results {
        if let Some(e) = result.err() {
            errors.push(format!("{}", e.to_string()));
        }
    }
    if !errors.is_empty() {
        let msg = errors.join(" | ");
        return Err(MediaError::UploadFailed(msg));
    }
    Ok(())
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
        cmd: UploadMediumCommand,
        config: &MediaConfig,
    ) -> Result<(), MediaError> {
        if !self.is_supported(&cmd.content_type, config).await? {
            return Err(MediaError::InvalidFileType);
        }
        let extension = MediaType::from_str(&cmd.content_type)?.get_extension();

        let HashData {
            hash,
            size,
            dir_path,
            file_path,
        } = hash_bytes(&cmd.bytes, &config.dir, extension.to_string()).await?;

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
        .bind(&cmd.file_name)
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
    async fn bulk_upload(
        &self,
        cmd: UploadMediaWithoutDescriptionCommand,
        config: &MediaConfig,
    ) -> Result<(), MediaError> {
        for content_type in &cmd.content_types {
            if !self.is_supported(&content_type, config).await? {
                return Err(MediaError::InvalidFileType);
            }
        }

        let mut tx = self.pool.begin().await?;
        let mut got_error: Option<MediaError> = None;
        let mut file_paths = Vec::<PathBuf>::new();

        for i in 0..cmd.number_of_files {
            let extension = match MediaType::from_str(&cmd.content_types[i]) {
                Ok(media_type) => media_type.get_extension(),
                Err(e) => {
                    got_error = Some(e);
                    break;
                }
            };

            let HashData {
                hash,
                size,
                dir_path,
                file_path,
            } = match hash_bytes(&cmd.bytes_list[i], &config.dir, extension.to_string()).await {
                Ok(hash_data) => hash_data,
                Err(e) => {
                    got_error = Some(e);
                    break;
                }
            };

            if fs::try_exists(&dir_path).await? {
                got_error = Some(MediaError::Duplication);
                break;
            }

            fs::create_dir_all(&dir_path).await?;
            fs::write(&file_path, &cmd.bytes_list[i]).await?;

            file_paths.push(file_path.clone());

            let file_path_str = match file_path.to_str() {
                Some(file_path_str) => file_path_str.to_string(),
                None => {
                    got_error = Some(MediaError::InternalError(
                        "Failed to get file path".to_string(),
                    ));
                    break;
                }
            };

            if let Err(e) = sqlx::query(
                r#"
                INSERT INTO media
                (hash, short_name, file_name, file_type, url, size, uploader_id)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&hash)
            .bind(&cmd.short_names[i])
            .bind(&cmd.file_names[i])
            .bind(&cmd.content_types[i])
            .bind(&file_path_str)
            .bind(size)
            .bind(cmd.uploader_id)
            .execute(&mut *tx)
            .await
            {
                got_error = Some(MediaError::InternalError(e.to_string()));
                break;
            }
        }

        if let Some(error) = got_error {
            if let Err(e) = clean_up_files(&file_paths).await {
                return Err(e);
            }

            return Err(error);
        }

        match tx.commit().await {
            Ok(()) => Ok(()),
            Err(e) => {
                if let Err(e) = clean_up_files(&file_paths).await {
                    return Err(e);
                }
                return Err(MediaError::from(e));
            }
        }
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
