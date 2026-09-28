// Chapter play counting.
//
// The counter measures listening time: the player reports once per ten seconds
// of real playback, and every report that reaches here increments the chapter,
// with no per-listener or per-day cap. The handler sheds bursts in memory
// before they get this far, which is the whole of the anti-spam story now that
// there is nothing to dedup against — see the note there on what that costs.
//
// Reports for unknown tracks and unpublished books are a silent no-op: the
// beacon can neither error noisily nor probe which tracks exist.

use crate::application::commands::audiobook::RecordTrackPlayCommand;
use crate::domain::errors::audiobook::AudiobookError;

use super::AudiobookServiceImpl;

impl AudiobookServiceImpl {
    /// Returns the chapter's new total, or `None` when the report was not
    /// counted because the track is unknown or its book is not published.
    pub(super) async fn record_track_play(
        &self,
        cmd: RecordTrackPlayCommand,
    ) -> Result<Option<i64>, AudiobookError> {
        // The published-book guard and the increment are one statement, so a
        // report can never be counted against a draft. The guard is an EXISTS
        // rather than a join because the row being updated is the one the join
        // would read from.
        let counted: Option<(i64,)> = sqlx::query_as(
            r#"
            UPDATE audiobook_tracks
               SET play_count = play_count + 1
             WHERE id = ?
               AND EXISTS (
                   SELECT 1 FROM audiobooks a
                    WHERE a.id = audiobook_tracks.audiobook_id
                      AND a.status = 'published'
               )
            RETURNING play_count
            "#,
        )
        .bind(cmd.track_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(counted.map(|(play_count,)| play_count))
    }
}
