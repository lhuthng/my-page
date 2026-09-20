// Track management: add/edit/remove/reorder plus sequencing and loading.
use std::path::PathBuf;

use sqlx::{Row, Sqlite, Transaction};

use crate::application::{
    commands::audiobook::{
        AddTrackCommand, RemoveTrackCommand, ReorderTracksCommand, UpdateTrackCommand,
    },
    services::audiobook::AudiobookService,
};
use crate::domain::entities::audiobook::AudiobookTrack;
use crate::domain::errors::audiobook::AudiobookError;

use super::rows::TrackRow;
use super::validation::MAX_TRACKS_PER_AUDIOBOOK;
use super::AudiobookServiceImpl;

impl AudiobookServiceImpl {
    pub(super) async fn resequence_tracks(
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
    pub(super) async fn load_tracks(
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
}

impl AudiobookService for AudiobookServiceImpl {
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

}
