// sqlx FromRow struct for media search.
use sqlx::prelude::FromRow;

#[derive(FromRow, Debug)]
pub(crate) struct MediaSearchRow {
    pub short_name: String,
    pub url: String,
    pub file_type: String,
    pub hash: String,
    pub uploader_id: i64,
    /// Upload time in UTC. Nullable because the column is `TEXT DEFAULT
    /// CURRENT_TIMESTAMP` rather than `NOT NULL`, so a row inserted by hand or
    /// predating the default can genuinely have no value.
    pub created_at: Option<String>,
}
