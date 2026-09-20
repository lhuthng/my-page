// sqlx FromRow struct for media search.
use sqlx::prelude::FromRow;

#[derive(FromRow, Debug)]
struct MediaSearchRow {
    pub short_name: String,
    pub url: String,
    pub file_type: String,
    pub hash: String,
    pub uploader_id: i64,
}

