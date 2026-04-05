use std::{path::PathBuf, str::FromStr};

use axum::body::Bytes;
use futures::future::join_all;
use sha2::{Digest, Sha256};
use sqlx::{SqlitePool, prelude::FromRow};
use tokio::fs;

use crate::{
    application::{
        commands::media::{
            AddAliasCommand, ChangeAliasCommand, ChangeAvatarCommand, ChangeMediaDetailsCommand,
            ChangePostCoverCommand, DeleteAliasCommand, GetAliasesCommand, GetLinkCommand,
            GetMediaDetailsCommand, SearchMediaCommand, UploadMediaWithoutDescriptionCommand,
            UploadMediumCommand,
        },
        services::media::MediaService,
    },
    domain::{
        entities::media::{LinkResult, MediaDetailResult, MediaType, MediumDetails},
        errors::media::MediaError,
    },
    infrastructure::web::server::MediaConfig,
};

#[derive(FromRow, Debug)]
struct MediaSearchRow {
    pub short_name: String,
    pub url: String,
    pub file_type: String,
}

pub struct MediaServiceImpl {
    pub pool: SqlitePool,
}

impl MediaServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

pub struct HashData {
    pub hash: String,
    pub size: i64,
    pub dir_path: PathBuf,
    pub file_path: PathBuf,
}

fn generate_dir_and_name(
    root: &PathBuf,
    hash: &String,
    extension: String,
    split: bool,
) -> (PathBuf, String) {
    let dir_path = match split {
        true => {
            let dir1 = &hash[0..2];
            let dir2 = &hash[2..4];
            &root.join(dir1).join(dir2)
        }
        false => root,
    };

    let filename = format!("{}{}", hash, extension);

    (dir_path.to_path_buf(), filename)
}

pub async fn hash_bytes(
    bytes: &Bytes,
    root: &PathBuf,
    extension: String,
    split: bool,
) -> Result<HashData, MediaError> {
    let hash = format!("{:x}", Sha256::digest(&bytes));
    if hash.len() < 4 {
        return Err(MediaError::InternalError("Hash too short.".to_string()));
    }

    let (dir_path, file_name) = generate_dir_and_name(root, &hash, extension, split);

    let file_path = dir_path.join(file_name);
    let size = bytes.len() as i64;

    Ok(HashData {
        hash,
        size,
        dir_path,
        file_path,
    })
}

pub async fn clean_up_files(file_paths: &Vec<PathBuf>) -> Result<(), MediaError> {
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

impl MediaServiceImpl {
    async fn is_supported(
        &self,
        file_type: &str,
        config: &MediaConfig,
    ) -> Result<bool, MediaError> {
        let media_type = MediaType::from_str(file_type)?;

        Ok(config.allowed_file_types.contains(&media_type))
    }
    async fn is_avatar_supported(
        &self,
        file_type: &str,
        config: &MediaConfig,
    ) -> Result<bool, MediaError> {
        let media_type = MediaType::from_str(file_type)?;

        Ok(config.allowed_avatar_types.contains(&media_type))
    }
}

#[async_trait::async_trait]
impl MediaService for MediaServiceImpl {
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
        } = hash_bytes(&cmd.bytes, &config.dir, extension.to_string(), true).await?;

        if fs::try_exists(&file_path).await? {
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
                return Err(MediaError::InternalError(format!(
                    "Failed to remove file after DB error {}",
                    remove_err
                )));
            }
            return Err(MediaError::InternalError(e.to_string()));
        }

        tx.commit().await?;

        Ok(())
    }
    async fn change_avatar(
        &self,
        cmd: ChangeAvatarCommand,
        config: &MediaConfig,
    ) -> Result<(), MediaError> {
        let MediumDetails {
            content_type,
            filename,
            bytes,
        } = cmd.medium_details;

        if !self.is_avatar_supported(&content_type, config).await? {
            return Err(MediaError::InvalidFileType);
        }

        let mut tx = self.pool.begin().await?;

        let hash_row = sqlx::query_as::<_, (Option<i64>, Option<String>, Option<String>)>(
            r#"
            SELECT media.id, media.hash, media.file_type
            FROM users
            JOIN user_meta ON users.id = user_meta.user_id
            LEFT JOIN media on media.id = user_meta.avatar_image_id
            WHERE users.id = ?
            "#,
        )
        .bind(&cmd.user_id)
        .fetch_one(&mut *tx)
        .await?;

        if let (Some(id), _, _) = hash_row {
            sqlx::query(
                r#"
                DELETE FROM media
                WHERE id = ?
                "#,
            )
            .bind(id)
            .execute(&mut *tx)
            .await?;
        }

        let extension = MediaType::from_str(&content_type)?.get_extension();

        let root = config.dir.join("avt").join(&cmd.user_id.to_string());

        let HashData {
            mut hash,
            size,
            dir_path,
            file_path,
        } = hash_bytes(&bytes, &root, extension.to_string(), false).await?;

        if fs::try_exists(&file_path).await? {
            return Ok(());
        }

        fs::create_dir_all(&dir_path).await?;
        fs::write(&file_path, &bytes).await?;

        let short_name = format!(".avt.{}", cmd.user_id);

        hash = format!(".avt.{}.{}", &cmd.user_id, hash);

        match sqlx::query_as::<_, (i64,)>(
            r#"
            INSERT INTO media
            (hash, short_name, file_name, file_type, url, size, uploader_id)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(&hash)
        .bind(short_name)
        .bind(&filename)
        .bind(&content_type)
        .bind(&file_path.to_str().ok_or(MediaError::ExposedInternalError(
            "Failed to get file path".to_string(),
        ))?)
        .bind(size)
        .bind(&cmd.user_id)
        .fetch_one(&mut *tx)
        .await
        {
            Ok((image_id,)) => {
                sqlx::query(
                    r#"
                    UPDATE user_meta
                    SET avatar_image_id = ?
                    WHERE user_id = ?
                    "#,
                )
                .bind(image_id)
                .bind(&cmd.user_id)
                .execute(&mut *tx)
                .await?;
            }
            Err(e) => {
                if let Err(remove_err) = fs::remove_file(&file_path).await {
                    return Err(MediaError::ExposedInternalError(format!(
                        "Failed to remove file after DB error {}",
                        remove_err
                    )));
                }
                return Err(MediaError::InternalError(e.to_string()));
            }
        }

        tx.commit().await?;

        if let (_, Some(hash), Some(file_type)) = hash_row {
            let hash = match hash.split('.').skip(3).next() {
                Some(hash) => hash.to_string(),
                None => {
                    return Err(MediaError::UploadFailed(
                        "Invalid stored hash found".to_string(),
                    ));
                }
            };

            let extension = match file_type.split('/').skip(1).next() {
                Some(extension) => format!(".{}", extension),
                None => {
                    return Err(MediaError::UploadFailed(
                        "Invalid stored file type found".to_string(),
                    ));
                }
            };

            let (dir_path, file_name) =
                generate_dir_and_name(&root, &hash, extension.to_string(), false);

            let file_path = dir_path.join(file_name);

            if let Err(remove_err) = fs::remove_file(&file_path).await {
                return Err(MediaError::ExposedInternalError(format!(
                    "Failed to clean up previous avatar after updated {}",
                    remove_err
                )));
            }
        }

        Ok(())
    }
    async fn change_post_cover(
        &self,
        cmd: ChangePostCoverCommand,
        config: &MediaConfig,
    ) -> Result<(), MediaError> {
        let MediumDetails {
            content_type,
            filename,
            bytes,
        } = cmd.medium_details;

        if !self.is_avatar_supported(&content_type, config).await? {
            return Err(MediaError::InvalidFileType);
        }

        let mut tx = self.pool.begin().await?;
        let exist: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM posts
                WHERE id = ? AND user_id = ?
            )
            "#,
        )
        .bind(&cmd.post_id)
        .bind(&cmd.user_id)
        .fetch_one(&mut *tx)
        .await?;
        if !exist {
            return Err(MediaError::PermissionDenied);
        }

        let hash_row = sqlx::query_as::<_, (Option<i64>, Option<String>, Option<String>)>(
            r#"
            SELECT media.id, media.hash, media.file_type
            FROM posts
            LEFT JOIN media on media.id = posts.cover_image_id
            WHERE posts.id = ?
            "#,
        )
        .bind(&cmd.post_id)
        .fetch_one(&mut *tx)
        .await?;

        if let (Some(id), _, _) = hash_row {
            sqlx::query(
                r#"
                DELETE FROM media
                WHERE id = ?
                "#,
            )
            .bind(id)
            .execute(&mut *tx)
            .await?;
        }

        let extension = MediaType::from_str(&content_type)?.get_extension();

        let root = config.dir.join("post").join(&cmd.user_id.to_string());

        let HashData {
            mut hash,
            size,
            dir_path,
            file_path,
        } = hash_bytes(&bytes, &root, extension.to_string(), false).await?;

        if fs::try_exists(&file_path).await? {
            return Ok(());
        }

        fs::create_dir_all(&dir_path).await?;
        fs::write(&file_path, &bytes).await?;

        let short_name = format!(".post.{}", cmd.post_id);

        hash = format!(".post.{}.{}", &cmd.post_id, hash);

        match sqlx::query_as::<_, (i64,)>(
            r#"
            INSERT INTO media
            (hash, short_name, file_name, file_type, url, size, uploader_id)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(&hash)
        .bind(short_name)
        .bind(&filename)
        .bind(&content_type)
        .bind(&file_path.to_str().ok_or(MediaError::ExposedInternalError(
            "Failed to get file path".to_string(),
        ))?)
        .bind(size)
        .bind(&cmd.user_id)
        .fetch_one(&mut *tx)
        .await
        {
            Ok((image_id,)) => {
                sqlx::query(
                    r#"
                    UPDATE posts
                    SET cover_image_id = ?
                    WHERE id = ?
                    "#,
                )
                .bind(image_id)
                .bind(&cmd.post_id)
                .execute(&mut *tx)
                .await?;
            }
            Err(e) => {
                if let Err(remove_err) = fs::remove_file(&file_path).await {
                    return Err(MediaError::ExposedInternalError(format!(
                        "Failed to remove file after DB error {}",
                        remove_err
                    )));
                }
                return Err(MediaError::InternalError(e.to_string()));
            }
        }

        tx.commit().await?;

        if let (_, Some(hash), Some(file_type)) = hash_row {
            let hash = match hash.split('.').skip(3).next() {
                Some(hash) => hash.to_string(),
                None => {
                    return Err(MediaError::UploadFailed(
                        "Invalid stored hash found".to_string(),
                    ));
                }
            };

            let extension = match file_type.split('/').skip(1).next() {
                Some(extension) => format!(".{}", extension),
                None => {
                    return Err(MediaError::UploadFailed(
                        "Invalid stored file type found".to_string(),
                    ));
                }
            };

            let (dir_path, file_name) =
                generate_dir_and_name(&root, &hash, extension.to_string(), false);

            let file_path = dir_path.join(file_name);

            if let Err(remove_err) = fs::remove_file(&file_path).await {
                return Err(MediaError::ExposedInternalError(format!(
                    "Failed to clean up previous avatar after updated {}",
                    remove_err
                )));
            }
        }

        Ok(())
    }
    async fn bulk_upload(
        &self,
        cmd: UploadMediaWithoutDescriptionCommand,
        config: &MediaConfig,
    ) -> Result<(), MediaError> {
        for content_type in &cmd.content_types {
            if !self.is_supported(&content_type, config).await? {
                println!("InvalidFileType?");
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
            } = match hash_bytes(&cmd.bytes_list[i], &config.dir, extension.to_string(), true).await
            {
                Ok(hash_data) => hash_data,
                Err(e) => {
                    got_error = Some(e);
                    break;
                }
            };

            if fs::try_exists(&file_path).await? {
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
        match sqlx::query_as::<_, (String, String)>(
            r#"
                SELECT url, file_type FROM media WHERE short_name = ?;
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
                m.file_type,
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
                file_type: r.file_type,
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
