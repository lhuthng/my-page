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
    ) -> Result<Vec<entities::series::SeriesSnapshot>, errors::series::SeriesError>;
    async fn get_all_series(
        &self,
        cmd: commands::series::GetAllSeriesCommand,
    ) -> Result<Vec<entities::series::SeriesWithPosts>, errors::series::SeriesError>;
    async fn new_series(
        &self,
        cmd: commands::series::NewSeriesCommand,
        config: &MediaConfig,
    ) -> Result<bool, errors::series::SeriesError>;

    async fn add_post_to_series(
        &self,
        cmd: commands::series::AddPostToSeriesCommand,
    ) -> Result<bool, errors::series::SeriesError>;
    async fn remove_post_from_series(
        &self,
        cmd: commands::series::RemovePostFromSeriesCommand,
    ) -> Result<bool, errors::series::SeriesError>;
}
