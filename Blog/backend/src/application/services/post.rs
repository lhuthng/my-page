use crate::{application::commands, domain::errors};

#[async_trait::async_trait]
pub trait PostService {
    async fn check_slug(
        &self,
        cmd: commands::post::CheckSlugCommand,
    ) -> Result<bool, errors::post::PostError>;
    async fn get_categories(
        &self,
        cmd: commands::post::GetCategoriesCommand,
    ) -> Result<Vec<(String, String)>, errors::post::PostError>;
    async fn post(&self, cmd: commands::post::PostCommand) -> Result<(), errors::post::PostError>;
    async fn publish(
        &self,
        cmd: commands::post::PublishCommand,
    ) -> Result<(), errors::post::PostError>;
}
