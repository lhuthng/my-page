use std::{path::PathBuf, str::FromStr};

use sqlx::{Sqlite, SqlitePool, Transaction};
use tokio::fs;

use crate::{
    application::{
        commands::audiobook::{
            AddTrackCommand, ChangeAudiobookStatusCommand, CheckAudiobookSlugCommand,
            DeleteAudiobookCommand, GetAudiobookCommand, GetAudiobooksCommand,
            GetPublicAudiobookCommand, GetPublicAudiobooksCommand, ListAudiobookTagsCommand,
            NewAudiobookCommand, RemoveTrackCommand, ReorderTracksCommand, SetAudiobookCoverCommand,
            UpdateAudiobookCommand, UpdateTrackCommand,
        },
        services::audiobook::AudiobookService,
    },
    domain::{
        entities::{
            audiobook::{AudiobookDetails, AudiobookSnapshot, AudiobookTag, AudiobookTrack},
            media::MediaType,
        },
        errors::{audiobook::AudiobookError, media::MediaError},
    },
    infrastructure::{
        persistence::{
            image_convert::convert_to_webp,
            media::{HashData, hash_bytes},
        },
        web::server::MediaConfig,
    },
};

const MAX_TITLE_CHARS: usize = 300;
const MAX_DESCRIPTION_CHARS: usize = 4000;
const MAX_TRANSLATOR_CHARS: usize = 200;
const MAX_TRACK_TITLE_CHARS: usize = 300;
const MAX_TAG_NAME_CHARS: usize = 60;
/// Upper bound on tracks per audiobook: the editor reorders in memory and the
/// player renders a flat playlist, so an unbounded count would let one request
/// pull an enormous payload.
const MAX_TRACKS_PER_AUDIOBOOK: i64 = 2000;
const MAX_TAGS_PER_AUDIOBOOK: usize = 25;

pub struct AudiobookServiceImpl {
    pub pool: SqlitePool,
}

impl AudiobookServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

/// Columns shared by every audiobook summary query, in a fixed order.
type SnapshotRow = (
    i64,
    String,
    String,
    String,
    Option<String>,
    String,
    Option<String>,
    i64,
    i64,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
);

type DetailsRow = (
    i64,
    String,
    String,
    String,
    Option<String>,
    String,
    Option<String>,
    i64,
    String,
    String,
    String,
    Option<String>,
);

/// A track joined with the media row it plays from.
type TrackRow = (
    i64,
    String,
    i64,
    Option<i64>,
    Option<String>,
    Option<String>,
);

/// A cover upload already written to disk, ready to be registered as a media
/// row. Built before the transaction opens so no large write happens while the
/// database write lock is held.
struct PreparedCover {
    media_type: MediaType,
    file_path: PathBuf,
    short_name: String,
    stored_hash: String,
    filename: String,
    size: i64,
}

impl AudiobookServiceImpl {
    async fn is_cover_supported(
        &self,
        file_type: &str,
        config: &MediaConfig,
    ) -> Result<bool, MediaError> {
        let media_type = MediaType::from_str(file_type)?;

        Ok(config.allowed_cover_types.contains(&media_type))
    }

    async fn is_audio_supported(
        &self,
        file_type: &str,
        config: &MediaConfig,
    ) -> Result<bool, MediaError> {
        let media_type = MediaType::from_str(file_type)?;

        Ok(config.allowed_audio_types.contains(&media_type))
    }

    /// Confirm the caller may mutate `audiobook_id`, returning the owning user.
    ///
    /// Admins pass for any row; everyone else only for their own. A missing row
    /// is reported as `NotFound` so a probe cannot distinguish "not yours" from
    /// "does not exist".
    async fn assert_owned(
        tx: &mut Transaction<'_, Sqlite>,
        audiobook_id: i64,
        user_id: i64,
        is_admin: bool,
    ) -> Result<(), AudiobookError> {
        let owner: Option<i64> =
            sqlx::query_scalar("SELECT user_id FROM audiobooks WHERE id = ?")
                .bind(audiobook_id)
                .fetch_optional(&mut **tx)
                .await?;

        match owner {
            None => Err(AudiobookError::NotFound),
            Some(owner) if is_admin || owner == user_id => Ok(()),
            Some(_) => Err(AudiobookError::PermissionDenied),
        }
    }

    /// Renumber an audiobook's tracks contiguously from 1, preserving the
    /// current (number, id) ordering. Called after every insert/remove so the
    /// player's "next track" is always simply `number + 1`.
    async fn resequence_tracks(
        tx: &mut Transaction<'_, Sqlite>,
        audiobook_id: i64,
    ) -> Result<(), AudiobookError> {
        let ids: Vec<(i64,)> = sqlx::query_as(
            "SELECT id FROM audiobook_tracks WHERE audiobook_id = ? ORDER BY number ASC, id ASC",
        )
        .bind(audiobook_id)
        .fetch_all(&mut **tx)
        .await?;

        for (index, (track_id,)) in ids.iter().enumerate() {
            sqlx::query("UPDATE audiobook_tracks SET number = ? WHERE id = ?")
                .bind(index as i64 + 1)
                .bind(track_id)
                .execute(&mut **tx)
                .await?;
        }

        Ok(())
    }

    /// Replace an audiobook's tag links with exactly `names`, creating any tag
    /// rows that do not exist yet. Tag identity is the slug, so "Sci-Fi" and
    /// "sci fi" collapse onto one row instead of duplicating.
    async fn replace_tags(
        tx: &mut Transaction<'_, Sqlite>,
        audiobook_id: i64,
        names: &[String],
    ) -> Result<(), AudiobookError> {
        sqlx::query("DELETE FROM audiobook_tag_links WHERE audiobook_id = ?")
            .bind(audiobook_id)
            .execute(&mut **tx)
            .await?;

        let mut seen: Vec<String> = Vec::new();
        for raw in names {
            let name = crate::helper::string::validate_text(raw, "Tag", MAX_TAG_NAME_CHARS)
                .map_err(AudiobookError::Validation)?;
            let slug = crate::helper::string::slugify(&name);
            if slug.is_empty() {
                return Err(AudiobookError::Validation(format!(
                    "Tag '{}' must contain at least one letter or number.",
                    name
                )));
            }
            // Duplicate tags in one payload would violate the link primary key.
            if seen.contains(&slug) {
                continue;
            }
            if seen.len() >= MAX_TAGS_PER_AUDIOBOOK {
                return Err(AudiobookError::Validation(format!(
                    "An audiobook may have at most {MAX_TAGS_PER_AUDIOBOOK} tags."
                )));
            }
            seen.push(slug.clone());

            // Reuse an existing tag row when the slug matches, otherwise create
            // it; `ON CONFLICT DO NOTHING` keeps a concurrent insert harmless.
            sqlx::query(
                "INSERT INTO audiobook_tags (name, slug) VALUES (?, ?)
                 ON CONFLICT(slug) DO NOTHING",
            )
            .bind(&name)
            .bind(&slug)
            .execute(&mut **tx)
            .await?;

            let tag_id: i64 = sqlx::query_scalar("SELECT id FROM audiobook_tags WHERE slug = ?")
                .bind(&slug)
                .fetch_one(&mut **tx)
                .await?;

            sqlx::query(
                "INSERT INTO audiobook_tag_links (audiobook_id, tag_id) VALUES (?, ?)
                 ON CONFLICT(audiobook_id, tag_id) DO NOTHING",
            )
            .bind(audiobook_id)
            .bind(tag_id)
            .execute(&mut **tx)
            .await?;
        }

        Ok(())
    }

    async fn load_tags(
        tx: &mut Transaction<'_, Sqlite>,
        audiobook_id: i64,
    ) -> Result<Vec<AudiobookTag>, AudiobookError> {
        let rows: Vec<(i64, String, String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT t.id, t.name, t.slug, t.description
            FROM audiobook_tag_links l
            JOIN audiobook_tags t ON t.id = l.tag_id
            WHERE l.audiobook_id = ?
            ORDER BY t.name COLLATE NOCASE ASC
            "#,
        )
        .bind(audiobook_id)
        .fetch_all(&mut **tx)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, name, slug, description)| AudiobookTag {
                id,
                name,
                slug,
                description,
                // Per-audiobook reads do not need the global usage count; the
                // tag manager fills it in.
                audiobook_count: 0,
            })
            .collect())
    }

    async fn load_tracks(
        tx: &mut Transaction<'_, Sqlite>,
        audiobook_id: i64,
    ) -> Result<Vec<AudiobookTrack>, AudiobookError> {
        let rows: Vec<TrackRow> = sqlx::query_as(
            r#"
            SELECT t.id, t.title, t.number, t.duration_seconds,
                   m.short_name, m.file_type
            FROM audiobook_tracks t
            LEFT JOIN media m ON m.id = t.media_id
            WHERE t.audiobook_id = ?
            ORDER BY t.number ASC
            "#,
        )
            .bind(audiobook_id)
            .fetch_all(&mut **tx)
            .await?;

        Ok(rows
            .into_iter()
            .filter_map(
                |(id, title, number, duration_seconds, short_name, file_type)| {
                    // A track whose media row vanished is not playable; skip it
                    // rather than emitting an entry the player cannot load.
                    let short_name = short_name?;
                    Some(AudiobookTrack {
                        id,
                        title,
                        number,
                        duration_seconds,
                        url: format!("media/i/{}", short_name),
                        short_name,
                        file_type: file_type.unwrap_or_else(|| "audio/mpeg".to_string()),
                    })
                },
            )
            .collect())
    }

    /// Attach tags to a batch of snapshots in one extra query.
    async fn attach_snapshot_tags(
        pool: &SqlitePool,
        snapshots: &mut [AudiobookSnapshot],
    ) -> Result<(), AudiobookError> {
        if snapshots.is_empty() {
            return Ok(());
        }

        // ids are i64 values read from the database, so interpolating them into
        // the IN list cannot inject SQL.
        let ids = snapshots
            .iter()
            .map(|s| s.id.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!(
            r#"
            SELECT l.audiobook_id, t.name, t.slug
            FROM audiobook_tag_links l
            JOIN audiobook_tags t ON t.id = l.tag_id
            WHERE l.audiobook_id IN ({ids})
            ORDER BY t.name COLLATE NOCASE ASC
            "#
        );

        let rows: Vec<(i64, String, String)> = sqlx::query_as(&sql).fetch_all(pool).await?;

        for (audiobook_id, name, slug) in rows {
            if let Some(snapshot) = snapshots.iter_mut().find(|s| s.id == audiobook_id) {
                snapshot.tags.push(name);
                snapshot.tag_slugs.push(slug);
            }
        }

        Ok(())
    }

    /// Write an uploaded file into content-addressed storage and register a
    /// media row, returning the new media id.
    ///
    /// Regular media layout (`<media_dir>/<sha[0..2]>/<sha[2..4]>/<sha><ext>`)
    /// is used on purpose: the `/media/i/{short_name}` handler reconstructs
    /// exactly that path from a plain-SHA `hash` column, which is what makes
    /// range-request streaming work for these files.
    async fn store_medium(
        tx: &mut Transaction<'_, Sqlite>,
        uploader_id: i64,
        config: &MediaConfig,
        medium: &crate::domain::entities::media::MediumDetails,
        media_type: MediaType,
        short_name_prefix: String,
        created_path: &mut Option<PathBuf>,
    ) -> Result<i64, AudiobookError> {
        let content_type = media_type.get_content_type().to_string();
        let extension = media_type.get_extension();

        let HashData {
            hash,
            size,
            dir_path,
            file_path,
        } = hash_bytes(&medium.bytes, &config.dir, extension.to_string(), true).await?;

        // Content-addressed storage dedupes identical uploads: two tracks with
        // the same audio share one file on disk.
        if !fs::try_exists(&file_path).await? {
            fs::create_dir_all(&dir_path).await?;
            fs::write(&file_path, &medium.bytes).await?;
            *created_path = Some(file_path.clone());
        }

        // `short_name` is UNIQUE in `media`, so a random tail guarantees
        // uniqueness even when the same bytes are registered more than once.
        let short_name = format!(
            "{}-{}-{}",
            short_name_prefix,
            &hash[..8],
            crate::helper::string::random_suffix()
        );

        let media_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO media
            (hash, short_name, file_name, file_type, url, size, description, uploader_id)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(&hash)
        .bind(&short_name)
        .bind(&medium.filename)
        .bind(&content_type)
        .bind(file_path.to_str().ok_or_else(|| {
            AudiobookError::ExposedInternalError("Failed to get file path".to_string())
        })?)
        .bind(size)
        .bind("Audiobook track")
        .bind(uploader_id)
        .fetch_one(&mut **tx)
        .await?;

        Ok(media_id)
    }
}

#[async_trait::async_trait]
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

    async fn add_track(
        &self,
        cmd: AddTrackCommand,
        config: &MediaConfig,
    ) -> Result<i64, AudiobookError> {
        let title =
            crate::helper::string::validate_text(&cmd.title, "Track title", MAX_TRACK_TITLE_CHARS)
                .map_err(AudiobookError::Validation)?;
        let duration_seconds = cmd.duration_seconds.filter(|d| *d >= 0);

        let media_type = MediaType::from_upload(&cmd.medium.content_type, &cmd.medium.filename)?;
        if !self.is_audio_supported(media_type.get_content_type(), config).await? {
            return Err(AudiobookError::Media(MediaError::InvalidFileType));
        }

        let mut tx = self.pool.begin().await?;
        Self::assert_owned(&mut tx, cmd.audiobook_id, cmd.user_id, cmd.is_admin).await?;

        let track_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM audiobook_tracks WHERE audiobook_id = ?")
                .bind(cmd.audiobook_id)
                .fetch_one(&mut *tx)
                .await?;
        if track_count >= MAX_TRACKS_PER_AUDIOBOOK {
            return Err(AudiobookError::Validation(format!(
                "An audiobook may have at most {MAX_TRACKS_PER_AUDIOBOOK} tracks."
            )));
        }

        // Append by default; an explicit position inserts and shifts the rest
        // down so numbering stays contiguous.
        let target = match cmd.number {
            Some(n) => n.clamp(1, track_count + 1),
            None => track_count + 1,
        };

        // Readable, collision-free media handle: audiobook + a slugged slice of
        // the title, with the content hash and a random tail appended by
        // `store_medium`.
        let short_name_prefix = format!(
            "abt-{}-{}",
            cmd.audiobook_id,
            crate::helper::string::slugify(&title)
                .chars()
                .take(24)
                .collect::<String>()
        );

        let mut created_path: Option<PathBuf> = None;
        let media_id = match Self::store_medium(
            &mut tx,
            cmd.user_id,
            config,
            &cmd.medium,
            media_type,
            short_name_prefix,
            &mut created_path,
        )
        .await
        {
            Ok(id) => id,
            Err(e) => {
                if let Some(path) = created_path {
                    let _ = fs::remove_file(path).await;
                }
                return Err(e);
            }
        };

        sqlx::query(
            "UPDATE audiobook_tracks SET number = number + 1 WHERE audiobook_id = ? AND number >= ?",
        )
        .bind(cmd.audiobook_id)
        .bind(target)
        .execute(&mut *tx)
        .await?;

        let track_id: i64 = match sqlx::query_scalar(
            r#"
            INSERT INTO audiobook_tracks (audiobook_id, media_id, title, number, duration_seconds)
            VALUES (?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(cmd.audiobook_id)
        .bind(media_id)
        .bind(&title)
        .bind(target)
        .bind(duration_seconds)
        .fetch_one(&mut *tx)
        .await
        {
            Ok(id) => id,
            Err(e) => {
                // Roll back the file too, so a failed insert cannot leave an
                // orphan on disk that no row points at.
                if let Some(path) = created_path {
                    let _ = fs::remove_file(path).await;
                }
                return Err(AudiobookError::InternalError(e.to_string()));
            }
        };

        sqlx::query("UPDATE audiobooks SET updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(cmd.audiobook_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(track_id)
    }

    async fn update_track(&self, cmd: UpdateTrackCommand) -> Result<(), AudiobookError> {
        let mut tx = self.pool.begin().await?;
        Self::assert_owned(&mut tx, cmd.audiobook_id, cmd.user_id, cmd.is_admin).await?;

        let current: Option<(i64, i64)> = sqlx::query_as(
            "SELECT number, media_id FROM audiobook_tracks WHERE id = ? AND audiobook_id = ?",
        )
        .bind(cmd.track_id)
        .bind(cmd.audiobook_id)
        .fetch_optional(&mut *tx)
        .await?;
        let (current_number, _) = current.ok_or(AudiobookError::NotFound)?;

        if let Some(raw_title) = cmd.title.as_deref() {
            let title = crate::helper::string::validate_text(
                raw_title,
                "Track title",
                MAX_TRACK_TITLE_CHARS,
            )
            .map_err(AudiobookError::Validation)?;
            sqlx::query("UPDATE audiobook_tracks SET title = ? WHERE id = ?")
                .bind(title)
                .bind(cmd.track_id)
                .execute(&mut *tx)
                .await?;
        }

        if let Some(duration) = cmd.duration_seconds {
            sqlx::query("UPDATE audiobook_tracks SET duration_seconds = ? WHERE id = ?")
                .bind(duration.max(0))
                .bind(cmd.track_id)
                .execute(&mut *tx)
                .await?;
        }

        if let Some(number) = cmd.number {
            let count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM audiobook_tracks WHERE audiobook_id = ?")
                    .bind(cmd.audiobook_id)
                    .fetch_one(&mut *tx)
                    .await?;
            let target = number.clamp(1, count.max(1));

            if target < current_number {
                // Moving up: everything in [target, current) shifts down.
                sqlx::query(
                    "UPDATE audiobook_tracks SET number = number + 1
                     WHERE audiobook_id = ? AND number >= ? AND number < ?",
                )
                .bind(cmd.audiobook_id)
                .bind(target)
                .bind(current_number)
                .execute(&mut *tx)
                .await?;
            } else if target > current_number {
                // Moving down: everything in (current, target] shifts up.
                sqlx::query(
                    "UPDATE audiobook_tracks SET number = number - 1
                     WHERE audiobook_id = ? AND number > ? AND number <= ?",
                )
                .bind(cmd.audiobook_id)
                .bind(current_number)
                .bind(target)
                .execute(&mut *tx)
                .await?;
            }

            sqlx::query("UPDATE audiobook_tracks SET number = ? WHERE id = ?")
                .bind(target)
                .bind(cmd.track_id)
                .execute(&mut *tx)
                .await?;

            Self::resequence_tracks(&mut tx, cmd.audiobook_id).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn remove_track(&self, cmd: RemoveTrackCommand) -> Result<(), AudiobookError> {
        let mut tx = self.pool.begin().await?;
        Self::assert_owned(&mut tx, cmd.audiobook_id, cmd.user_id, cmd.is_admin).await?;

        let affected = sqlx::query("DELETE FROM audiobook_tracks WHERE id = ? AND audiobook_id = ?")
            .bind(cmd.track_id)
            .bind(cmd.audiobook_id)
            .execute(&mut *tx)
            .await?
            .rows_affected();

        if affected == 0 {
            return Err(AudiobookError::NotFound);
        }

        Self::resequence_tracks(&mut tx, cmd.audiobook_id).await?;

        sqlx::query("UPDATE audiobooks SET updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(cmd.audiobook_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn reorder_tracks(&self, cmd: ReorderTracksCommand) -> Result<(), AudiobookError> {
        let mut tx = self.pool.begin().await?;
        Self::assert_owned(&mut tx, cmd.audiobook_id, cmd.user_id, cmd.is_admin).await?;

        let existing: Vec<(i64,)> =
            sqlx::query_as("SELECT id FROM audiobook_tracks WHERE audiobook_id = ?")
                .bind(cmd.audiobook_id)
                .fetch_all(&mut *tx)
                .await?;

        let mut existing_ids: Vec<i64> = existing.into_iter().map(|(id,)| id).collect();
        let mut incoming = cmd.order.clone();
        existing_ids.sort_unstable();
        incoming.sort_unstable();

        // Require an exact permutation: a partial or duplicated list would
        // otherwise silently drop or double-assign a track.
        if existing_ids != incoming {
            return Err(AudiobookError::Validation(
                "Track order must list every track of the audiobook exactly once.".to_string(),
            ));
        }

        for (index, track_id) in cmd.order.iter().enumerate() {
            sqlx::query("UPDATE audiobook_tracks SET number = ? WHERE id = ? AND audiobook_id = ?")
                .bind(index as i64 + 1)
                .bind(track_id)
                .bind(cmd.audiobook_id)
                .execute(&mut *tx)
                .await?;
        }

        sqlx::query("UPDATE audiobooks SET updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(cmd.audiobook_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn list_audiobook_tags(
        &self,
        cmd: ListAudiobookTagsCommand,
    ) -> Result<Vec<AudiobookTag>, AudiobookError> {
        let rows: Vec<(i64, String, String, Option<String>, i64)> = if cmd.is_admin {
            sqlx::query_as(
                r#"
                SELECT t.id, t.name, t.slug, t.description,
                       (SELECT COUNT(*) FROM audiobook_tag_links l
                        WHERE l.tag_id = t.id)
                FROM audiobook_tags t
                ORDER BY t.name COLLATE NOCASE ASC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(cmd.limit)
            .bind(cmd.offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as(
                r#"
                SELECT t.id, t.name, t.slug, t.description,
                       (SELECT COUNT(*) FROM audiobook_tag_links l
                        JOIN audiobooks a ON a.id = l.audiobook_id
                        WHERE l.tag_id = t.id AND a.user_id = ?)
                FROM audiobook_tags t
                WHERE EXISTS (
                    SELECT 1 FROM audiobook_tag_links l
                    JOIN audiobooks a ON a.id = l.audiobook_id
                    WHERE l.tag_id = t.id AND a.user_id = ?)
                ORDER BY t.name COLLATE NOCASE ASC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(cmd.user_id)
            .bind(cmd.user_id)
            .bind(cmd.limit)
            .bind(cmd.offset)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows
            .into_iter()
            .map(|(id, name, slug, description, audiobook_count)| AudiobookTag {
                id,
                name,
                slug,
                description,
                audiobook_count,
            })
            .collect())
    }

    async fn check_audiobook_slug(
        &self,
        cmd: CheckAudiobookSlugCommand,
    ) -> Result<bool, AudiobookError> {
        let slug = crate::helper::string::validate_slug(&cmd.slug)
            .map_err(AudiobookError::Validation)?;

        let taken: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM audiobooks WHERE slug = ?)")
                .bind(&slug)
                .fetch_one(&self.pool)
                .await?;

        // `true` means the slug is free.
        Ok(!taken)
    }
}
