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
    ) -> Result<entities::post::PostSnapshotPage, errors::post::PostError>;
    async fn get_post_details(
        &self,
        cmd: commands::post::GetDetailedPostsCommand,
    ) -> Result<entities::post::PostDetails, errors::post::PostError>;
    async fn search(
        &self,
        cmd: commands::post::SearchPostCommand,
    ) -> Result<Vec<entities::post::PostSummary>, errors::post::PostError>;
    async fn search_tags(
        &self,
        cmd: commands::post::SearchTagsCommand,
    ) -> Result<Vec<entities::post::TagSummary>, errors::post::PostError>;
    async fn get_posts_by_tag(
        &self,
        cmd: commands::post::GetPostsByTagCommand,
    ) -> Result<
        (
            entities::post::TagSummary,
            Vec<entities::post::PostSnapshot>,
        ),
        errors::post::PostError,
    >;
    async fn get_comments(
        &self,
        cmd: commands::post::GetCommentsCommand,
    ) -> Result<entities::post::CommentPage, errors::post::PostError>;
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
    async fn get_related_posts(
        &self,
        cmd: commands::post::GetRelatedPostsCommand,
    ) -> Result<Vec<entities::post::PostSummary>, errors::post::PostError>;
    async fn set_related_posts(
        &self,
        cmd: commands::post::SetRelatedPostsCommand,
    ) -> Result<(), errors::post::PostError>;
    async fn set_post_featured(
        &self,
        cmd: commands::post::SetFeaturedPostCommand,
    ) -> Result<(), errors::post::PostError>;
    async fn update_post_cover(
        &self,
        cmd: commands::post::UpdatePostCoverCommand,
    ) -> Result<(), errors::post::PostError>;
}
