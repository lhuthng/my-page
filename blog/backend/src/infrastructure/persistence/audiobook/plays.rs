// Chapter play counting.
//
// The measurement bar: one play per listener per track per UTC day, counted
// only while the book is published. The listener key arrives pre-hashed from
// the handler, so no raw client identity ever reaches the database, and the
// handler sheds bursty repeat reports in memory before they get this far.

use crate::application::commands::audiobook::RecordTrackPlayCommand;
use crate::domain::errors::audiobook::AudiobookError;

use super::AudiobookServiceImpl;

impl AudiobookServiceImpl {
    pub(super) async fn record_track_play(
        &self,
        cmd: RecordTrackPlayCommand,
    ) -> Result<(), AudiobookError> {
        // Unknown track ids and unpublished books are a silent no-op: the
        // beacon can neither error noisily nor probe which tracks exist.
        let playable: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT t.id FROM audiobook_tracks t
            JOIN audiobooks a ON a.id = t.audiobook_id
            WHERE t.id = ? AND a.status = 'published'
            "#,
        )
        .bind(cmd.track_id)
        .fetch_optional(&self.pool)
        .await?;

        if playable.is_none() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        // The dedup row IS the measurement: the counter only moves when this
        // listener had not already played the chapter today, so replaying the
        // insert (refreshes, a second tab, a scripted client) cannot inflate
        // the count beyond one play per listener per day.
        let counted: Option<(i64,)> = sqlx::query_as(
            r#"
            INSERT OR IGNORE INTO audiobook_track_plays (track_id, listener, day)
            VALUES (?, ?, ?)
            RETURNING track_id
            "#,
        )
        .bind(cmd.track_id)
        .bind(&cmd.listener)
        .bind(&cmd.day)
        .fetch_optional(&mut *tx)
        .await?;

        if counted.is_some() {
            sqlx::query("UPDATE audiobook_tracks SET play_count = play_count + 1 WHERE id = ?")
                .bind(cmd.track_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}
