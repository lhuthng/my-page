// Series persistence adapter: implements
// `application::services::series::SeriesService` against SQLite. The plan's
// rows.rs is skipped: this adapter reads via `query_as` tuples, not FromRow
// structs.
use std::str::FromStr;

use sqlx::SqlitePool;

use crate::domain::entities::media::MediaType;
use crate::domain::errors::media::MediaError;
use crate::infrastructure::web::server::MediaConfig;

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

// `SeriesService` port implementation: thin delegation to the inherent
// methods in the sibling files (inherent candidates win over trait
// candidates in method resolution, so `self.<method>` calls the local one).
use crate::application::commands;
use crate::application::services::series::SeriesService;
use crate::domain::{entities, errors};

#[async_trait::async_trait]
impl SeriesService for SeriesServiceImpl {
    async fn get_series(
        &self,
        cmd: commands::series::GetSeriesCommand,
    ) -> Result<Vec<entities::series::SeriesSnapshot>, errors::series::SeriesError> {
        self.get_series(cmd).await
    }
    async fn get_all_series(
        &self,
        cmd: commands::series::GetAllSeriesCommand,
    ) -> Result<Vec<entities::series::SeriesWithPosts>, errors::series::SeriesError> {
        self.get_all_series(cmd).await
    }
    async fn new_series(
        &self,
        cmd: commands::series::NewSeriesCommand,
        config: &MediaConfig,
    ) -> Result<bool, errors::series::SeriesError> {
        self.new_series(cmd, config).await
    }
    async fn add_post_to_series(
        &self,
        cmd: commands::series::AddPostToSeriesCommand,
    ) -> Result<bool, errors::series::SeriesError> {
        self.add_post_to_series(cmd).await
    }
    async fn remove_post_from_series(
        &self,
        cmd: commands::series::RemovePostFromSeriesCommand,
    ) -> Result<bool, errors::series::SeriesError> {
        self.remove_post_from_series(cmd).await
    }
}
