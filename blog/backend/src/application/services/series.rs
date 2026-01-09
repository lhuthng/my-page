use crate::{
    application::commands,
    domain::{entities, errors},
    infrastructure::web::server::MediaConfig,
};

#[async_trait::async_trait]
pub trait SeriesService {
    async fn get_series(
        &self,
        cmd: commands::series::GetSeriesCommand,
    ) -> Result<Vec<entities::series::Series>, errors::series::SeriesError>;
    async fn new_series(
        &self,
        cmd: commands::series::NewSeriesCommand,
        config: &MediaConfig,
    ) -> Result<bool, errors::series::SeriesError>;

    async fn add_post_to_series(
        &self,
        cmd: commands::series::AddPostToSeriesCommand,
    ) -> Result<bool, errors::series::SeriesError>;
}
