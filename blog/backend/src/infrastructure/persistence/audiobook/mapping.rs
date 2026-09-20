// Snapshot assembly: attaching tags to loaded audiobook snapshots.
use sqlx::Row;

use crate::domain::errors::audiobook::AudiobookError;

use super::AudiobookServiceImpl;

impl AudiobookServiceImpl {
    pub(super) async fn attach_snapshot_tags(
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
}
