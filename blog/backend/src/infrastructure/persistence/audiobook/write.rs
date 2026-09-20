// Write methods: create, update, cover, status, delete.
use std::path::PathBuf;

use sqlx::Row;

use crate::application::{
    commands::audiobook::{
        ChangeAudiobookStatusCommand, DeleteAudiobookCommand, NewAudiobookCommand,
        SetAudiobookCoverCommand, UpdateAudiobookCommand,
    },
    services::audiobook::AudiobookService,
};
use crate::domain::entities::audiobook::{AudiobookDetails, AudiobookSnapshot, AudiobookTrack};
use crate::domain::errors::{audiobook::AudiobookError, media::MediaError};

use super::medium::PreparedCover;
use super::rows::{DetailsRow, SnapshotRow, TrackRow};
use super::{tags, tracks, validation};
use super::AudiobookServiceImpl;

#[async_trait::async_trait]
impl AudiobookService for AudiobookServiceImpl {
    async fn new_audiobook(
        &self,
        cmd: NewAudiobookCommand,
        config: &MediaConfig,
    ) -> Result<i64, AudiobookError> {
        let title = crate::helper::string::validate_text(&cmd.title, "Title", MAX_TITLE_CHARS)
            .map_err(AudiobookError::Validation)?;
        let slug = crate::helper::string::validate_slug(&cmd.slug)
            .map_err(AudiobookError::Validation)?;
        let description = {
            let trimmed = cmd.description.trim();
            if trimmed.chars().count() > MAX_DESCRIPTION_CHARS {
                return Err(AudiobookError::Validation(format!(
                    "Description must be at most {MAX_DESCRIPTION_CHARS} characters."
                )));
            }
            trimmed.to_string()
        };
        let translator = crate::helper::string::validate_optional_long_text(
            cmd.translator.as_deref(),
            "Translator",
            MAX_TRANSLATOR_CHARS,
        )
        .map_err(AudiobookError::Validation)?;

        // Prepare the cover file before opening the transaction so a large
        // upload is never written while a database write lock is held.
        let mut cover_media_id: Option<i64> = None;
        let mut prepared_cover: Option<PreparedCover> = None;
        if let Some(cover) = cmd.cover_image {
            let (bytes, content_type, filename) =
                convert_to_webp(cover.bytes, &cover.content_type, &cover.filename).await?;

            if !self.is_cover_supported(&content_type, config).await? {
                return Err(AudiobookError::Media(MediaError::InvalidFileType));
            }

            let media_type = MediaType::from_str(&content_type)?;
            let extension = media_type.get_extension();
            let root = config.dir.join("abc").join(cmd.user_id.to_string());

            let HashData {
                hash,
                dir_path,
                file_path,
                ..
            } = hash_bytes(&bytes, &root, extension.to_string(), false).await?;

            if !fs::try_exists(&file_path).await? {
                fs::create_dir_all(&dir_path).await?;
                fs::write(&file_path, &bytes).await?;
            }

            // `.abc.<user_id>.<sha>` is the layout the media handler decodes
            // into `<media_dir>/abc/<uploader_id>/<sha><ext>`.
            let short_name = format!(".abc.{}", hash);
            let stored_hash = format!(".abc.{}.{}", cmd.user_id, hash);
            prepared_cover = Some(PreparedCover {
                media_type,
                file_path,
                short_name,
                stored_hash,
                filename,
                size: bytes.len() as i64,
            });
        }

        let mut tx = self.pool.begin().await?;

        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM audiobooks WHERE slug = ?)")
                .bind(&slug)
                .fetch_one(&mut *tx)
                .await?;
        if exists {
            return Err(AudiobookError::Duplication);
        }

        if let Some(cover) = prepared_cover {
            cover_media_id = Some(
                sqlx::query_scalar(
                    r#"
                    INSERT INTO media
                    (hash, short_name, file_name, file_type, url, size, description, uploader_id)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                    RETURNING id
                    "#,
                )
                .bind(&cover.stored_hash)
                .bind(&cover.short_name)
                .bind(&cover.filename)
                .bind(cover.media_type.get_content_type())
                .bind(cover.file_path.to_str().ok_or_else(|| {
                    AudiobookError::ExposedInternalError("Failed to get file path".to_string())
                })?)
                .bind(cover.size)
                .bind("Audiobook cover")
                .bind(cmd.user_id)
                .fetch_one(&mut *tx)
                .await?,
            );
        }

        let audiobook_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO audiobooks
            (user_id, title, slug, description, translator, cover_image_id)
            VALUES (?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(cmd.user_id)
        .bind(&title)
        .bind(&slug)
        .bind(&description)
        .bind(&translator)
        .bind(cover_media_id)
        .fetch_one(&mut *tx)
        .await?;

        Self::replace_tags(&mut tx, audiobook_id, &cmd.tags).await?;

        tx.commit().await?;

        Ok(audiobook_id)
    }

    async fn update_audiobook(
        &self,
        cmd: UpdateAudiobookCommand,
    ) -> Result<(), AudiobookError> {
        let mut tx = self.pool.begin().await?;
        Self::assert_owned(&mut tx, cmd.audiobook_id, cmd.user_id, cmd.is_admin).await?;

        if let Some(raw_title) = cmd.title.as_deref() {
            let title = crate::helper::string::validate_text(raw_title, "Title", MAX_TITLE_CHARS)
                .map_err(AudiobookError::Validation)?;
            sqlx::query("UPDATE audiobooks SET title = ? WHERE id = ?")
                .bind(title)
                .bind(cmd.audiobook_id)
                .execute(&mut *tx)
                .await?;
        }

        if let Some(raw_slug) = cmd.slug.as_deref() {
            let slug = crate::helper::string::validate_slug(raw_slug)
                .map_err(AudiobookError::Validation)?;
            let taken: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM audiobooks WHERE slug = ? AND id != ?)",
            )
            .bind(&slug)
            .bind(cmd.audiobook_id)
            .fetch_one(&mut *tx)
            .await?;
            if taken {
                return Err(AudiobookError::Duplication);
            }
            sqlx::query("UPDATE audiobooks SET slug = ? WHERE id = ?")
                .bind(slug)
                .bind(cmd.audiobook_id)
                .execute(&mut *tx)
                .await?;
        }

        if let Some(raw_description) = cmd.description.as_deref() {
            let trimmed = raw_description.trim();
            if trimmed.chars().count() > MAX_DESCRIPTION_CHARS {
                return Err(AudiobookError::Validation(format!(
                    "Description must be at most {MAX_DESCRIPTION_CHARS} characters."
                )));
            }
            sqlx::query("UPDATE audiobooks SET description = ? WHERE id = ?")
                .bind(trimmed)
                .bind(cmd.audiobook_id)
                .execute(&mut *tx)
                .await?;
        }

        if let Some(translator) = cmd.translator.as_ref() {
            let translator = crate::helper::string::validate_optional_long_text(
                translator.as_deref(),
                "Translator",
                MAX_TRANSLATOR_CHARS,
            )
            .map_err(AudiobookError::Validation)?;
            sqlx::query("UPDATE audiobooks SET translator = ? WHERE id = ?")
                .bind(translator)
                .bind(cmd.audiobook_id)
                .execute(&mut *tx)
                .await?;
        }

        if let Some(tags) = cmd.tags.as_ref() {
            Self::replace_tags(&mut tx, cmd.audiobook_id, tags).await?;
        }

        sqlx::query("UPDATE audiobooks SET updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(cmd.audiobook_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn set_audiobook_cover(
        &self,
        cmd: SetAudiobookCoverCommand,
        config: &MediaConfig,
    ) -> Result<(), AudiobookError> {
        let (bytes, content_type, filename) = convert_to_webp(
            cmd.medium.bytes,
            &cmd.medium.content_type,
            &cmd.medium.filename,
        )
        .await?;

        if !self.is_cover_supported(&content_type, config).await? {
            return Err(AudiobookError::Media(MediaError::InvalidFileType));
        }

        let media_type = MediaType::from_str(&content_type)?;
        let extension = media_type.get_extension();
        let root = config.dir.join("abc").join(cmd.user_id.to_string());

        let HashData {
            hash,
            dir_path,
            file_path,
            ..
        } = hash_bytes(&bytes, &root, extension.to_string(), false).await?;

        if !fs::try_exists(&file_path).await? {
            fs::create_dir_all(&dir_path).await?;
            fs::write(&file_path, &bytes).await?;
        }

        let short_name = format!(".abc.{}", hash);
        let stored_hash = format!(".abc.{}.{}", cmd.user_id, hash);
        let size = bytes.len() as i64;

        let mut tx = self.pool.begin().await?;
        Self::assert_owned(&mut tx, cmd.audiobook_id, cmd.user_id, cmd.is_admin).await?;

        let media_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO media
            (hash, short_name, file_name, file_type, url, size, description, uploader_id)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(&stored_hash)
        .bind(&short_name)
        .bind(&filename)
        .bind(media_type.get_content_type())
        .bind(file_path.to_str().ok_or_else(|| {
            AudiobookError::ExposedInternalError("Failed to get file path".to_string())
        })?)
        .bind(size)
        .bind("Audiobook cover")
        .bind(cmd.user_id)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            "UPDATE audiobooks SET cover_image_id = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(media_id)
        .bind(cmd.audiobook_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn change_audiobook_status(
        &self,
        cmd: ChangeAudiobookStatusCommand,
    ) -> Result<(), AudiobookError> {
        if !matches!(cmd.status.as_str(), "draft" | "published" | "archived") {
            return Err(AudiobookError::Validation(
                "Status must be one of draft, published, archived.".to_string(),
            ));
        }

        let mut tx = self.pool.begin().await?;
        Self::assert_owned(&mut tx, cmd.audiobook_id, cmd.user_id, cmd.is_admin).await?;

        if cmd.status == "published" {
            // Publishing an audiobook with no playable track would produce a
            // public page the player cannot start.
            let track_count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM audiobook_tracks WHERE audiobook_id = ?")
                    .bind(cmd.audiobook_id)
                    .fetch_one(&mut *tx)
                    .await?;
            if track_count == 0 {
                return Err(AudiobookError::Validation(
                    "Add at least one track before publishing.".to_string(),
                ));
            }

            // Keep the original publication date on re-publish.
            sqlx::query(
                "UPDATE audiobooks
                 SET status = ?, published_at = COALESCE(published_at, CURRENT_TIMESTAMP),
                     updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?",
            )
            .bind(&cmd.status)
            .bind(cmd.audiobook_id)
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query(
                "UPDATE audiobooks SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            )
            .bind(&cmd.status)
            .bind(cmd.audiobook_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn delete_audiobook(
        &self,
        cmd: DeleteAudiobookCommand,
    ) -> Result<(), AudiobookError> {
        let mut tx = self.pool.begin().await?;
        Self::assert_owned(&mut tx, cmd.audiobook_id, cmd.user_id, cmd.is_admin).await?;

        // Tracks and tag links cascade. Media rows and their files are left
        // alone: media is content-addressed and may be referenced elsewhere,
        // and the media manager owns its own lifecycle.
        sqlx::query("DELETE FROM audiobooks WHERE id = ?")
            .bind(cmd.audiobook_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

}
