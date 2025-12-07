use crate::{
    application::commands,
    domain::{
        entities::{self, post::Post},
        errors,
    },
};

#[async_trait::async_trait]
pub trait PostService {
    async fn check_slug(
        &self,
        cmd: commands::post::CheckSlugCommand,
    ) -> Result<bool, errors::post::PostError>;
    async fn get_categories(
        &self,
        cmd: commands::post::GetCategoriesCommand,
    ) -> Result<Vec<entities::post::CategoryResult>, errors::post::PostError>;
    async fn post(&self, cmd: commands::post::PostCommand) -> Result<(), errors::post::PostError>;
    async fn get_post(
        &self,
        cmd: commands::post::GetPostCommand,
    ) -> Result<Post, errors::post::PostError>;
    async fn publish(
        &self,
        cmd: commands::post::PublishCommand,
    ) -> Result<(), errors::post::PostError>;
    async fn get_featured_post_snapshots(
        &self,
        cmd: commands::post::GetFeaturedPostsCommand,
    ) -> Result<Vec<entities::post::PostSnapshot>, errors::post::PostError>;
    async fn get_latest_post_snapshots(
        &self,
        cmd: commands::post::GetLatestPostsCommand,
    ) -> Result<Vec<entities::post::PostSnapshot>, errors::post::PostError>;
    async fn get_post_details(
        &self,
        cmd: commands::post::GetDetailedPostsCommand,
    ) -> Result<entities::post::PostDetails, errors::post::PostError>;
    async fn post_new_comment(
        &self,
        cmd: commands::post::PostNewCommentCommand,
    ) -> Result<(), errors::post::PostError>;
    async fn post_new_anonymous_comment(
        &self,
        cmd: commands::post::PostNewAnynymouseCommentCommand,
    ) -> Result<(), errors::post::PostError>;
}
