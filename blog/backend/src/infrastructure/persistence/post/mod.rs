// Post persistence adapter: implements `application::services::post::PostService`
// against SQLite. The service methods are split across `read`, `detail`,
// `write`, `comments`, and `threads`; each contributes its own
// `impl PostService for PostServiceImpl` block.
use sqlx::SqlitePool;

mod comments;
mod detail;
mod links;
mod mapping;
mod read;
mod rows;
mod threads;
mod write;

pub use rows::{
    MediumUsageRow, MediumUsageWithNameRow, PostContentRow, PostDetailsRow, PostRow, PostSearchRow,
    TagRow, TagSummaryRow,
};

pub struct PostServiceImpl {
    pub pool: SqlitePool,
}

impl PostServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

// `PostService` port implementation: thin delegation to the inherent
// methods in the sibling files (inherent candidates win over trait
// candidates in method resolution, so `self.<method>` calls the local one).
use crate::application::commands;
use crate::application::services::post::PostService;
use crate::domain::{entities, errors};

#[async_trait::async_trait]
impl PostService for PostServiceImpl {
    async fn check_slug(
        &self,
        cmd: commands::post::CheckSlugCommand,
    ) -> Result<bool, errors::post::PostError> {
        self.check_slug(cmd).await
    }
    async fn get_categories(
        &self,
        cmd: commands::post::GetCategoriesCommand,
    ) -> Result<Vec<entities::post::CategoryResult>, errors::post::PostError> {
        self.get_categories(cmd).await
    }
    async fn new_post(
        &self,
        cmd: commands::post::NewPostCommand,
    ) -> Result<i64, errors::post::PostError> {
        self.new_post(cmd).await
    }
    async fn update_post(
        &self,
        cmd: commands::post::UpdatePostCommand,
    ) -> Result<String, errors::post::PostError> {
        self.update_post(cmd).await
    }
    async fn get_post(
        &self,
        cmd: commands::post::GetPostCommand,
    ) -> Result<entities::post::Post, errors::post::PostError> {
        self.get_post(cmd).await
    }
    async fn publish(
        &self,
        cmd: commands::post::PublishCommand,
    ) -> Result<(), errors::post::PostError> {
        self.publish(cmd).await
    }
    async fn get_featured_post_snapshots(
        &self,
        cmd: commands::post::GetFeaturedPostsCommand,
    ) -> Result<Vec<entities::post::PostSnapshot>, errors::post::PostError> {
        self.get_featured_post_snapshots(cmd).await
    }
    async fn get_latest_post_snapshots(
        &self,
        cmd: commands::post::GetLatestPostsCommand,
    ) -> Result<entities::post::PostSnapshotPage, errors::post::PostError> {
        self.get_latest_post_snapshots(cmd).await
    }
    async fn get_post_details(
        &self,
        cmd: commands::post::GetDetailedPostsCommand,
    ) -> Result<entities::post::PostDetails, errors::post::PostError> {
        self.get_post_details(cmd).await
    }
    async fn search(
        &self,
        cmd: commands::post::SearchPostCommand,
    ) -> Result<Vec<entities::post::PostSummary>, errors::post::PostError> {
        self.search(cmd).await
    }
    async fn search_tags(
        &self,
        cmd: commands::post::SearchTagsCommand,
    ) -> Result<Vec<entities::post::TagSummary>, errors::post::PostError> {
        self.search_tags(cmd).await
    }
    async fn get_posts_by_tag(
        &self,
        cmd: commands::post::GetPostsByTagCommand,
    ) -> Result<
        (
            entities::post::TagSummary,
            Vec<entities::post::PostSnapshot>,
        ),
        errors::post::PostError,
    > {
        self.get_posts_by_tag(cmd).await
    }
    async fn get_comments(
        &self,
        cmd: commands::post::GetCommentsCommand,
    ) -> Result<entities::post::CommentPage, errors::post::PostError> {
        self.get_comments(cmd).await
    }
    async fn post_new_comment(
        &self,
        cmd: commands::post::PostNewCommentCommand,
    ) -> Result<i64, errors::post::PostError> {
        self.post_new_comment(cmd).await
    }
    async fn post_new_anonymous_comment(
        &self,
        cmd: commands::post::PostNewAnynymouseCommentCommand,
    ) -> Result<i64, errors::post::PostError> {
        self.post_new_anonymous_comment(cmd).await
    }
    async fn push_new_view(
        &self,
        cmd: commands::post::PushNewViewCommand,
    ) -> Result<(), errors::post::PostError> {
        self.push_new_view(cmd).await
    }
    async fn push_new_like(
        &self,
        cmd: commands::post::PushNewLikeCommand,
    ) -> Result<(), errors::post::PostError> {
        self.push_new_like(cmd).await
    }
    async fn get_related_posts(
        &self,
        cmd: commands::post::GetRelatedPostsCommand,
    ) -> Result<Vec<entities::post::PostSummary>, errors::post::PostError> {
        self.get_related_posts(cmd).await
    }
    async fn set_related_posts(
        &self,
        cmd: commands::post::SetRelatedPostsCommand,
    ) -> Result<(), errors::post::PostError> {
        self.set_related_posts(cmd).await
    }
    async fn set_post_featured(
        &self,
        cmd: commands::post::SetFeaturedPostCommand,
    ) -> Result<(), errors::post::PostError> {
        self.set_post_featured(cmd).await
    }
    async fn update_post_cover(
        &self,
        cmd: commands::post::UpdatePostCoverCommand,
    ) -> Result<(), errors::post::PostError> {
        self.update_post_cover(cmd).await
    }
}
