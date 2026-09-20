// Series persistence adapter: implements
// `application::services::series::SeriesService` against SQLite. The plan's
// rows.rs is skipped: this adapter reads via `query_as` tuples, not FromRow
// structs.
use sqlx::SqlitePool;

mod read;
mod write;

pub struct SeriesServiceImpl {
    pub pool: SqlitePool,
}

impl SeriesServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl SeriesServiceImpl {
    async fn is_cover_supported(
        &self,
        file_type: &str,
        config: &MediaConfig,
    ) -> Result<bool, MediaError> {
        let media_type = MediaType::from_str(file_type)?;

        Ok(config.allowed_cover_types.contains(&media_type))
    }
}

