// Track management: add/edit/remove plus loading.
//
// A chapter's `number` is a **slot** the author chooses, not a position in a
// list: gaps are normal (chapters 1 and 100 can coexist), and no write here
// ever renumbers a chapter to close one.
use std::path::PathBuf;

use sqlx::{Sqlite, Transaction};
use tokio::fs;

use crate::application::commands::audiobook::{
    AddTrackCommand, RemoveTrackCommand, ReplaceTrackMediumCommand, UpdateTrackCommand,
};
use crate::domain::entities::audiobook::AudiobookTrack;
use crate::domain::entities::media::MediaType;
use crate::domain::errors::{audiobook::AudiobookError, media::MediaError};
use crate::infrastructure::storage::{audio_object_key, chunked, media_key};
use crate::infrastructure::web::server::MediaConfig;

use super::AudiobookServiceImpl;
use super::rows::TrackRow;
use super::validation::{MAX_TRACK_TITLE_CHARS, MAX_TRACKS_PER_AUDIOBOOK};

/// Rejects a chapter number another chapter of the same book already holds,
/// and any number below 1.
///
/// A number names a slot, so two chapters sharing one would make "the chapter
/// at 5" ambiguous — and `ORDER BY number` would then answer in whatever order
/// SQLite happened to return the tie in.
async fn assert_slot_is_free(
    tx: &mut Transaction<'_, Sqlite>,
    audiobook_id: i64,
    number: i64,
    except_track_id: Option<i64>,
) -> Result<(), AudiobookError> {
    if number < 1 {
        return Err(AudiobookError::Validation(
            "A chapter number must be 1 or greater.".to_string(),
        ));
    }

    let taken: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audiobook_tracks
         WHERE audiobook_id = ? AND number = ? AND id != COALESCE(?, -1)",
    )
    .bind(audiobook_id)
    .bind(number)
    .bind(except_track_id)
    .fetch_one(&mut **tx)
    .await?;

    if taken > 0 {
        return Err(AudiobookError::Validation(format!(
            "Chapter {number} already exists in this audiobook."
        )));
    }

    Ok(())
}

/// Removes the bytes behind a media row: the disk copy, and the bucket object
/// when the row is audio.
///
/// Only ever called once nothing points at them any more. Failures are logged
/// rather than returned: the write that needed them has already committed, and
/// a leftover file is a smaller problem than an error the author cannot act on.
async fn discard_medium_files(config: &MediaConfig, hash: &str, file_type: &str, uploader_id: i64) {
    let Some(key) = media_key(hash, file_type, uploader_id) else {
        tracing::warn!(
            hash,
            file_type,
            "removed medium: no storage key derivable, nothing deleted"
        );
        return;
    };

    match fs::remove_file(config.dir.join(&key.key)).await {
        Ok(()) => tracing::info!(key = %key.key, "removed medium: disk copy deleted"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => tracing::error!(
            key = %key.key,
            %error,
            "removed medium: disk copy could not be deleted"
        ),
    }

    if let Some(bucket) = &config.audio_bucket
        && let Some(object) = audio_object_key(&key.key, file_type)
        && let Err(error) = bucket.client.delete_object(&object).await
    {
        tracing::error!(
            key = %object,
            %error,
            "removed medium: bucket object could not be deleted"
        );
    }

    // The chunked form of the same audio, when there is one. Read first: the
    // manifest is the only record of which chunks exist, so deleting it before
    // its chunks would leave objects behind that nothing could ever name again.
    if let Some(manifest) = chunked::read_manifest(config, &key.key, file_type).await {
        chunked::discard(config, &key.key, file_type, &manifest).await;
    }
}

impl AudiobookServiceImpl {
    /// Every chapter of a book, in play order.
    ///
    /// The dashboard editor edits against the whole playlist, so it reads it in
    /// one go; the public player asks for a window instead.
    pub(super) async fn load_tracks(
        tx: &mut Transaction<'_, Sqlite>,
        audiobook_id: i64,
    ) -> Result<Vec<AudiobookTrack>, AudiobookError> {
        Self::load_track_window(tx, audiobook_id, 0, None).await
    }

    /// One window of a book's chapters, in play order.
    ///
    /// `limit` is `None` for "everything from `offset` on", which SQLite spells
    /// `LIMIT -1`. The window is taken in chapter order rather than by id, so a
    /// book with gaps in its slots still pages in the order the player renders
    /// it in.
    pub(super) async fn load_track_window(
        tx: &mut Transaction<'_, Sqlite>,
        audiobook_id: i64,
        offset: i64,
        limit: Option<i64>,
    ) -> Result<Vec<AudiobookTrack>, AudiobookError> {
        let rows: Vec<TrackRow> = sqlx::query_as(
            r#"
            SELECT t.id, t.title, t.number, t.duration_seconds, t.play_count,
                   m.short_name, m.file_type,
                   COALESCE(t.updated_at, t.created_at)
            FROM audiobook_tracks t
            LEFT JOIN media m ON m.id = t.media_id
            WHERE t.audiobook_id = ?
            ORDER BY t.number ASC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(audiobook_id)
        .bind(limit.unwrap_or(-1))
        .bind(offset)
        .fetch_all(&mut **tx)
        .await?;

        Ok(playable_tracks(rows))
    }

    /// Playable chapters in a book.
    ///
    /// Counted through the same join the window filters on: the window drops
    /// chapters whose media row is gone, so a count that included them would
    /// promise the player a row no window could ever fill.
    pub(super) async fn count_playable_tracks(
        tx: &mut Transaction<'_, Sqlite>,
        audiobook_id: i64,
    ) -> Result<i64, AudiobookError> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM audiobook_tracks t
            INNER JOIN media m ON m.id = t.media_id
            WHERE t.audiobook_id = ?
            "#,
        )
        .bind(audiobook_id)
        .fetch_one(&mut **tx)
        .await?;

        Ok(count)
    }
}

/// Drop the chapters a player could not load: a track whose media row vanished
/// has no URL to stream, so as far as the player is concerned it is not a
/// chapter at all.
fn playable_tracks(rows: Vec<TrackRow>) -> Vec<AudiobookTrack> {
    rows.into_iter()
        .filter_map(
            |(
                id,
                title,
                number,
                duration_seconds,
                play_count,
                short_name,
                file_type,
                updated_at,
            )| {
                let short_name = short_name?;
                Some(AudiobookTrack {
                    id,
                    title,
                    number,
                    duration_seconds,
                    play_count,
                    url: format!("media/i/{}", short_name),
                    short_name,
                    file_type: file_type.unwrap_or_else(|| "audio/mpeg".to_string()),
                    updated_at,
                })
            },
        )
        .collect()
}

impl AudiobookServiceImpl {
    pub(super) async fn add_track(
        &self,
        cmd: AddTrackCommand,
        config: &MediaConfig,
    ) -> Result<i64, AudiobookError> {
        let title =
            crate::helper::string::validate_text(&cmd.title, "Track title", MAX_TRACK_TITLE_CHARS)
                .map_err(AudiobookError::Validation)?;
        let duration_seconds = cmd.duration_seconds.filter(|d| *d >= 0);

        let media_type = MediaType::from_upload(&cmd.medium.content_type, &cmd.medium.filename)?;
        if !self
            .is_audio_supported(media_type.get_content_type(), config)
            .await?
        {
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

        // The chapter takes the slot it names — the editor names it, the upload
        // panel reads it off the file name — and nothing else moves for it.
        // With no slot named it lands past the highest, which is what "append"
        // means in a book whose numbers are slots rather than positions.
        let slot: i64 = match cmd.number {
            Some(number) => number,
            None => sqlx::query_scalar(
                "SELECT COALESCE(MAX(number), 0) + 1 FROM audiobook_tracks WHERE audiobook_id = ?",
            )
            .bind(cmd.audiobook_id)
            .fetch_one(&mut *tx)
            .await?,
        };
        // Settled before the upload is stored, so a rejected slot cannot leave
        // a file on disk that no row names.
        assert_slot_is_free(&mut tx, cmd.audiobook_id, slot, None).await?;

        // Opaque, collision-free media handle: the audiobook id, with the
        // content hash and a random tail appended by `store_medium`. The track
        // title is deliberately not slugged in — non-ASCII titles (Vietnamese
        // in particular) would degrade into a mangled letter soup — and the
        // handle is never user-visible anyway.
        let short_name_prefix = format!("abt-{}", cmd.audiobook_id);

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

        let track_id: i64 = match sqlx::query_scalar(
            r#"
            INSERT INTO audiobook_tracks
                (audiobook_id, media_id, title, number, duration_seconds, updated_at)
            VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            RETURNING id
            "#,
        )
        .bind(cmd.audiobook_id)
        .bind(media_id)
        .bind(&title)
        .bind(slot)
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

    pub(super) async fn update_track(&self, cmd: UpdateTrackCommand) -> Result<(), AudiobookError> {
        let mut tx = self.pool.begin().await?;
        Self::assert_owned(&mut tx, cmd.audiobook_id, cmd.user_id, cmd.is_admin).await?;

        let exists: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM audiobook_tracks WHERE id = ? AND audiobook_id = ?")
                .bind(cmd.track_id)
                .bind(cmd.audiobook_id)
                .fetch_optional(&mut *tx)
                .await?;
        if exists.is_none() {
            return Err(AudiobookError::NotFound);
        }

        if let Some(raw_title) = cmd.title.as_deref() {
            let title = crate::helper::string::validate_text(
                raw_title,
                "Track title",
                MAX_TRACK_TITLE_CHARS,
            )
            .map_err(AudiobookError::Validation)?;
            // A content edit, so it stamps the chapter's own modified time. The
            // slot branch below deliberately does not.
            sqlx::query(
                "UPDATE audiobook_tracks SET title = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            )
            .bind(title)
            .bind(cmd.track_id)
            .execute(&mut *tx)
            .await?;
        }

        if let Some(duration) = cmd.duration_seconds {
            sqlx::query(
                "UPDATE audiobook_tracks SET duration_seconds = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            )
            .bind(duration.max(0))
            .bind(cmd.track_id)
            .execute(&mut *tx)
            .await?;
        }

        if let Some(number) = cmd.number {
            // Where the chapter goes, not a position to be inserted at: the
            // slot has to be free, nothing else moves to make room, and the
            // slot it leaves behind simply stops being used. A move is also
            // deliberately not a modification — it is the same chapter — so
            // `updated_at` is left alone here, as the migration notes.
            assert_slot_is_free(&mut tx, cmd.audiobook_id, number, Some(cmd.track_id)).await?;

            sqlx::query("UPDATE audiobook_tracks SET number = ? WHERE id = ?")
                .bind(number)
                .bind(cmd.track_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Point an existing track at a newly uploaded audio file.
    ///
    /// The audio being replaced goes with it — its media row, its disk copy and
    /// its bucket object — but only when nothing else still points at them.
    /// Media is content-addressed, so a second chapter carrying the same audio
    /// shares that one file, and deleting it would pull the rug out from under
    /// a still-playable chapter. Anything still referring to the old row, or to
    /// its bytes, keeps them alive.
    pub(super) async fn replace_track_medium(
        &self,
        cmd: ReplaceTrackMediumCommand,
        config: &MediaConfig,
    ) -> Result<(), AudiobookError> {
        let media_type = MediaType::from_upload(&cmd.medium.content_type, &cmd.medium.filename)?;
        if !self
            .is_audio_supported(media_type.get_content_type(), config)
            .await?
        {
            return Err(AudiobookError::Media(MediaError::InvalidFileType));
        }

        let title = match cmd.title.as_deref() {
            Some(raw) => Some(
                crate::helper::string::validate_text(raw, "Track title", MAX_TRACK_TITLE_CHARS)
                    .map_err(AudiobookError::Validation)?,
            ),
            None => None,
        };

        let mut tx = self.pool.begin().await?;
        Self::assert_owned(&mut tx, cmd.audiobook_id, cmd.user_id, cmd.is_admin).await?;

        let exists: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM audiobook_tracks WHERE id = ? AND audiobook_id = ?")
                .bind(cmd.track_id)
                .bind(cmd.audiobook_id)
                .fetch_optional(&mut *tx)
                .await?;
        if exists.is_none() {
            return Err(AudiobookError::NotFound);
        }

        // The audio this chapter plays today, so its row and the files behind
        // it can be released once nothing refers to them any more.
        let previous: Option<(i64, String, String, i64)> = sqlx::query_as(
            "SELECT id, hash, file_type, uploader_id FROM media
             WHERE id = (SELECT media_id FROM audiobook_tracks WHERE id = ? AND audiobook_id = ?)",
        )
        .bind(cmd.track_id)
        .bind(cmd.audiobook_id)
        .fetch_optional(&mut *tx)
        .await?;

        let mut created_path: Option<PathBuf> = None;
        let media_id = match Self::store_medium(
            &mut tx,
            cmd.user_id,
            config,
            &cmd.medium,
            media_type,
            format!("abt-{}", cmd.audiobook_id),
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

        // A replaced audio file is the clearest content edit there is, so the
        // chapter's own modified time moves with it.
        sqlx::query(
            "UPDATE audiobook_tracks SET media_id = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND audiobook_id = ?",
        )
        .bind(media_id)
        .bind(cmd.track_id)
        .bind(cmd.audiobook_id)
        .execute(&mut *tx)
        .await?;

        if let Some(duration) = cmd.duration_seconds {
            sqlx::query(
                "UPDATE audiobook_tracks SET duration_seconds = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            )
            .bind(duration.max(0))
            .bind(cmd.track_id)
            .execute(&mut *tx)
            .await?;
        }

        if let Some(title) = title {
            sqlx::query(
                "UPDATE audiobook_tracks SET title = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            )
                .bind(title)
                .bind(cmd.track_id)
                .execute(&mut *tx)
                .await?;
        }

        sqlx::query("UPDATE audiobooks SET updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(cmd.audiobook_id)
            .execute(&mut *tx)
            .await?;

        // What the swap left nameless, and whether it is free to go. Decided
        // inside the transaction, because the answer depends on rows the swap
        // itself has just written.
        let released: Option<(String, String, i64)> = match previous {
            Some((previous_id, hash, file_type, uploader_id)) => {
                let still_played_by_a_chapter: i64 =
                    sqlx::query_scalar("SELECT COUNT(*) FROM audiobook_tracks WHERE media_id = ?")
                        .bind(previous_id)
                        .fetch_one(&mut *tx)
                        .await?;

                if still_played_by_a_chapter > 0 {
                    None
                } else {
                    // Identical audio uploaded twice shares one file, so another
                    // row holding this hash has to keep the bytes — which is
                    // also what makes a swap to the very same recording leave
                    // them alone.
                    let same_bytes_elsewhere: i64 =
                        sqlx::query_scalar("SELECT COUNT(*) FROM media WHERE hash = ? AND id != ?")
                            .bind(&hash)
                            .bind(previous_id)
                            .fetch_one(&mut *tx)
                            .await?;

                    sqlx::query("DELETE FROM media WHERE id = ?")
                        .bind(previous_id)
                        .execute(&mut *tx)
                        .await?;

                    (same_bytes_elsewhere == 0).then_some((hash, file_type, uploader_id))
                }
            }
            None => None,
        };

        tx.commit().await?;

        // Only after the commit: a swap that rolled back must not have deleted
        // the audio it still points at.
        if let Some((hash, file_type, uploader_id)) = released {
            discard_medium_files(config, &hash, &file_type, uploader_id).await;
        }

        Ok(())
    }

    pub(super) async fn remove_track(&self, cmd: RemoveTrackCommand) -> Result<(), AudiobookError> {
        let mut tx = self.pool.begin().await?;
        Self::assert_owned(&mut tx, cmd.audiobook_id, cmd.user_id, cmd.is_admin).await?;

        // The slot the chapter held is left empty rather than closed up:
        // removing chapter 5 from a book of 1, 5 and 100 leaves the other two
        // exactly where they were.
        let affected =
            sqlx::query("DELETE FROM audiobook_tracks WHERE id = ? AND audiobook_id = ?")
                .bind(cmd.track_id)
                .bind(cmd.audiobook_id)
                .execute(&mut *tx)
                .await?
                .rows_affected();

        if affected == 0 {
            return Err(AudiobookError::NotFound);
        }

        sqlx::query("UPDATE audiobooks SET updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(cmd.audiobook_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }
}
