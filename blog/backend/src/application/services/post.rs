use crate::{
    application::commands,
    domain::{entities, errors},
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
    async fn new_post(
        &self,
        cmd: commands::post::NewPostCommand,
    ) -> Result<i64, errors::post::PostError>;
    async fn update_post(
        &self,
        cmd: commands::post::UpdatePostCommand,
    ) -> Result<(), errors::post::PostError>;
    async fn get_post(
        &self,
        cmd: commands::post::GetPostCommand,
    ) -> Result<entities::post::Post, errors::post::PostError>;
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
    async fn search(
        &self,
        cmd: commands::post::SearchPostCommand,
    ) -> Result<Vec<entities::post::PostSummary>, errors::post::PostError>;
    async fn get_comments(
        &self,
        cmd: commands::post::GetCommentsCommand,
    ) -> Result<Vec<entities::post::Comment>, errors::post::PostError>;
    async fn post_new_comment(
        &self,
        cmd: commands::post::PostNewCommentCommand,
    ) -> Result<i64, errors::post::PostError>;
    async fn post_new_anonymous_comment(
        &self,
        cmd: commands::post::PostNewAnynymouseCommentCommand,
    ) -> Result<i64, errors::post::PostError>;
    async fn push_new_view(
        &self,
        cmd: commands::post::PushNewViewCommand,
    ) -> Result<(), errors::post::PostError>;
    async fn push_new_like(
        &self,
        cmd: commands::post::PushNewLikeCommand,
    ) -> Result<(), errors::post::PostError>;
}
