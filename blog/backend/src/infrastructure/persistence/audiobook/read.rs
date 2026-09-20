// Read methods: admin catalogue, public feed, single loads.
use std::path::PathBuf;

use sqlx::Row;

use crate::application::{
    commands::audiobook::{
        GetAudiobookCommand, GetAudiobooksCommand, GetPublicAudiobookCommand,
        GetPublicAudiobooksCommand,
    },
    services::audiobook::AudiobookService,
};
use crate::domain::entities::audiobook::{AudiobookDetails, AudiobookSnapshot, AudiobookTag};
use crate::domain::errors::audiobook::AudiobookError;

use super::mapping;
use super::rows::{DetailsRow, SnapshotRow};
use super::validation;
use super::AudiobookServiceImpl;

impl AudiobookService for AudiobookServiceImpl {
    async fn get_audiobooks(
        &self,
        cmd: GetAudiobooksCommand,
    ) -> Result<Vec<AudiobookSnapshot>, AudiobookError> {
        let term = cmd
            .term
            .as_ref()
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .map(|t| format!("%{t}%"));

        // Two branches instead of a single predicate so the permission filter
        // stays in SQL rather than being re-applied in Rust.
        let base = r#"
            SELECT a.id, a.title, a.slug, COALESCE(a.description, ''), a.translator, a.status,
                   'media/i/' || m.short_name AS cover_url,
                   (SELECT COUNT(*) FROM audiobook_tracks t WHERE t.audiobook_id = a.id),
                   (SELECT COALESCE(SUM(t.duration_seconds), 0) FROM audiobook_tracks t
                     WHERE t.audiobook_id = a.id),
                   u.username, um.display_name,
                   a.created_at, a.published_at
            FROM audiobooks a
            LEFT JOIN media m ON m.id = a.cover_image_id
            LEFT JOIN users u ON u.id = a.user_id
            LEFT JOIN user_meta um ON um.user_id = a.user_id
        "#;

        let rows: Vec<SnapshotRow> = if cmd.is_admin {
            let sql = format!(
                r#"{base}
                   WHERE (?1 IS NULL OR a.title LIKE ?1 OR a.slug LIKE ?1)
                   ORDER BY a.created_at DESC, a.id DESC
                   LIMIT ?2 OFFSET ?3"#
            );
            sqlx::query_as::<_, SnapshotRow>(&sql)
                .bind(&term)
                .bind(cmd.limit)
                .bind(cmd.offset)
                .fetch_all(&self.pool)
                .await?
        } else {
            let sql = format!(
                r#"{base}
                   WHERE a.user_id = ?2
                     AND (?1 IS NULL OR a.title LIKE ?1 OR a.slug LIKE ?1)
                   ORDER BY a.created_at DESC, a.id DESC
                   LIMIT ?3 OFFSET ?4"#
            );
            sqlx::query_as::<_, SnapshotRow>(&sql)
                .bind(&term)
                .bind(cmd.user_id)
                .bind(cmd.limit)
                .bind(cmd.offset)
                .fetch_all(&self.pool)
                .await?
        };

        let mut snapshots: Vec<AudiobookSnapshot> = rows
            .into_iter()
            .map(
                |(
                    id,
                    title,
                    slug,
                    description,
                    translator,
                    status,
                    url,
                    track_count,
                    total_duration_seconds,
                    owner_username,
                    owner_display_name,
                    created_at,
                    published_at,
                )| AudiobookSnapshot {
                    id,
                    title,
                    slug,
                    description,
                    translator,
                    status,
                    url,
                    track_count,
                    total_duration_seconds,
                    owner_username,
                    owner_display_name,
                    tags: Vec::new(),
                    tag_slugs: Vec::new(),
                    created_at,
                    published_at,
                },
            )
            .collect();

        Self::attach_snapshot_tags(&self.pool, &mut snapshots).await?;

        Ok(snapshots)
    }

    async fn get_public_audiobooks(
        &self,
        cmd: GetPublicAudiobooksCommand,
    ) -> Result<Vec<AudiobookSnapshot>, AudiobookError> {
        let term = cmd
            .term
            .as_ref()
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .map(|t| format!("%{t}%"));
        let tag = cmd
            .tag
            .as_ref()
            .map(|t| crate::helper::string::slugify(t))
            .filter(|t| !t.is_empty());

        let rows: Vec<SnapshotRow> = sqlx::query_as(
            r#"
            SELECT a.id, a.title, a.slug, COALESCE(a.description, ''), a.translator, a.status,
                   'media/i/' || m.short_name AS cover_url,
                   (SELECT COUNT(*) FROM audiobook_tracks t WHERE t.audiobook_id = a.id),
                   (SELECT COALESCE(SUM(t.duration_seconds), 0) FROM audiobook_tracks t
                     WHERE t.audiobook_id = a.id),
                   u.username, um.display_name,
                   a.created_at, a.published_at
            FROM audiobooks a
            LEFT JOIN media m ON m.id = a.cover_image_id
            LEFT JOIN users u ON u.id = a.user_id
            LEFT JOIN user_meta um ON um.user_id = a.user_id
            WHERE a.status = 'published'
              AND (?1 IS NULL OR a.title LIKE ?1 OR a.slug LIKE ?1)
              AND (?2 IS NULL OR EXISTS (
                    SELECT 1 FROM audiobook_tag_links l
                    JOIN audiobook_tags t ON t.id = l.tag_id
                    WHERE l.audiobook_id = a.id AND t.slug = ?2))
            ORDER BY COALESCE(a.published_at, a.created_at) DESC, a.id DESC
            LIMIT ?3 OFFSET ?4
            "#,
        )
        .bind(&term)
        .bind(&tag)
        .bind(cmd.limit)
        .bind(cmd.offset)
        .fetch_all(&self.pool)
        .await?;

        let mut snapshots: Vec<AudiobookSnapshot> = rows
            .into_iter()
            .map(
                |(
                    id,
                    title,
                    slug,
                    description,
                    translator,
                    status,
                    url,
                    track_count,
                    total_duration_seconds,
                    owner_username,
                    owner_display_name,
                    created_at,
                    published_at,
                )| AudiobookSnapshot {
                    id,
                    title,
                    slug,
                    description,
                    translator,
                    status,
                    url,
                    track_count,
                    total_duration_seconds,
                    owner_username,
                    owner_display_name,
                    tags: Vec::new(),
                    tag_slugs: Vec::new(),
                    created_at,
                    published_at,
                },
            )
            .collect();

        Self::attach_snapshot_tags(&self.pool, &mut snapshots).await?;

        Ok(snapshots)
    }

    async fn get_audiobook(
        &self,
        cmd: GetAudiobookCommand,
    ) -> Result<AudiobookDetails, AudiobookError> {
        let mut tx = self.pool.begin().await?;

        let sql = if cmd.is_admin {
            r#"
            SELECT a.id, a.title, a.slug, COALESCE(a.description, ''), a.translator, a.status,
                   'media/i/' || m.short_name AS cover_url,
                   (SELECT COALESCE(SUM(t.duration_seconds), 0) FROM audiobook_tracks t
                     WHERE t.audiobook_id = a.id),
                   COALESCE(u.username, ''), COALESCE(um.display_name, ''),
                   a.created_at, a.published_at
            FROM audiobooks a
            LEFT JOIN media m ON m.id = a.cover_image_id
            LEFT JOIN users u ON u.id = a.user_id
            LEFT JOIN user_meta um ON um.user_id = a.user_id
            WHERE a.id = ?
            "#
        } else {
            r#"
            SELECT a.id, a.title, a.slug, COALESCE(a.description, ''), a.translator, a.status,
                   'media/i/' || m.short_name AS cover_url,
                   (SELECT COALESCE(SUM(t.duration_seconds), 0) FROM audiobook_tracks t
                     WHERE t.audiobook_id = a.id),
                   COALESCE(u.username, ''), COALESCE(um.display_name, ''),
                   a.created_at, a.published_at
            FROM audiobooks a
            LEFT JOIN media m ON m.id = a.cover_image_id
            LEFT JOIN users u ON u.id = a.user_id
            LEFT JOIN user_meta um ON um.user_id = a.user_id
            WHERE a.id = ? AND a.user_id = ?
            "#
        };

        let row: Option<DetailsRow> = if cmd.is_admin {
            sqlx::query_as::<_, DetailsRow>(sql)
                .bind(cmd.audiobook_id)
                .fetch_optional(&mut *tx)
                .await?
        } else {
            sqlx::query_as::<_, DetailsRow>(sql)
                .bind(cmd.audiobook_id)
                .bind(cmd.user_id)
                .fetch_optional(&mut *tx)
                .await?
        };

        let row = row.ok_or(AudiobookError::NotFound)?;
        let tags = Self::load_tags(&mut tx, row.0).await?;
        let tracks = Self::load_tracks(&mut tx, row.0).await?;
        tx.commit().await?;

        Ok(AudiobookDetails {
            id: row.0,
            title: row.1,
            slug: row.2,
            description: row.3,
            translator: row.4,
            status: row.5,
            url: row.6,
            total_duration_seconds: row.7,
            owner_username: row.8,
            owner_display_name: row.9,
            tags,
            tracks,
            created_at: row.10,
            published_at: row.11,
        })
    }

    async fn get_public_audiobook(
        &self,
        cmd: GetPublicAudiobookCommand,
    ) -> Result<AudiobookDetails, AudiobookError> {
        let mut tx = self.pool.begin().await?;

        let row: Option<DetailsRow> = sqlx::query_as(
            r#"
            SELECT a.id, a.title, a.slug, COALESCE(a.description, ''), a.translator, a.status,
                   'media/i/' || m.short_name AS cover_url,
                   (SELECT COALESCE(SUM(t.duration_seconds), 0) FROM audiobook_tracks t
                     WHERE t.audiobook_id = a.id),
                   COALESCE(u.username, ''), COALESCE(um.display_name, ''),
                   a.created_at, a.published_at
            FROM audiobooks a
            LEFT JOIN media m ON m.id = a.cover_image_id
            LEFT JOIN users u ON u.id = a.user_id
            LEFT JOIN user_meta um ON um.user_id = a.user_id
            WHERE a.slug = ? AND a.status = 'published'
            "#,
        )
        .bind(&cmd.slug)
        .fetch_optional(&mut *tx)
        .await?;

        let row = row.ok_or(AudiobookError::NotFound)?;
        let tags = Self::load_tags(&mut tx, row.0).await?;
        let tracks = Self::load_tracks(&mut tx, row.0).await?;
        tx.commit().await?;

        Ok(AudiobookDetails {
            id: row.0,
            title: row.1,
            slug: row.2,
            description: row.3,
            translator: row.4,
            status: row.5,
            url: row.6,
            total_duration_seconds: row.7,
            owner_username: row.8,
            owner_display_name: row.9,
            tags,
            tracks,
            created_at: row.10,
            published_at: row.11,
        })
    }

}
