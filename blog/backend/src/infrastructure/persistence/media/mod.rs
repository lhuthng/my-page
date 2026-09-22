// Media persistence adapter: implements
// `application::services::media::MediaService` against SQLite + the object
// store. Split by concern; each trait-impl file carries its own
// `#[async_trait] impl MediaService` block.
use sqlx::SqlitePool;

mod aliases;
mod avatar;
mod covers;
mod crud;
mod files;
mod hashing;
mod rows;
mod search;
mod upload;
mod validation;

pub use files::clean_up_files;
pub use hashing::{HashData, hash_bytes};

pub struct MediaServiceImpl {
    pub pool: SqlitePool,
}

impl MediaServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

// `MediaService` port implementation: thin delegation to the inherent
// methods in the sibling files (inherent candidates win over trait
// candidates in method resolution, so `self.<method>` calls the local one).
use crate::application::commands::{
    self,
    media::{
        AddAliasCommand, ChangeAliasCommand, ChangeMediaDetailsCommand, DeleteAliasCommand,
        GetAliasesCommand, GetMediaDetailsCommand,
    },
};
use crate::application::services::media::MediaService;
use crate::domain::{
    entities::{self, media::MediaDetailResult},
    errors::{self, media::MediaError},
};
use crate::infrastructure::web::server::MediaConfig;

#[async_trait::async_trait]
impl MediaService for MediaServiceImpl {
    async fn change_avatar(
        &self,
        cmd: commands::media::ChangeAvatarCommand,
        config: &MediaConfig,
    ) -> Result<(), errors::media::MediaError> {
        self.change_avatar(cmd, config).await
    }
    async fn change_post_cover(
        &self,
        cmd: commands::media::ChangePostCoverCommand,
        config: &MediaConfig,
    ) -> Result<(), errors::media::MediaError> {
        self.change_post_cover(cmd, config).await
    }
    async fn upload(
        &self,
        cmd: commands::media::UploadMediumCommand,
        config: &MediaConfig,
    ) -> Result<(), errors::media::MediaError> {
        self.upload(cmd, config).await
    }
    async fn bulk_upload(
        &self,
        cmd: commands::media::UploadMediaWithoutDescriptionCommand,
        config: &MediaConfig,
    ) -> Result<(), errors::media::MediaError> {
        self.bulk_upload(cmd, config).await
    }
    async fn get_link(
        &self,
        cmd: commands::media::GetLinkCommand,
    ) -> Result<entities::media::LinkResult, MediaError> {
        self.get_link(cmd).await
    }
    async fn search(
        &self,
        cmd: commands::media::SearchMediaCommand,
    ) -> Result<Vec<entities::media::LinkResult>, MediaError> {
        self.search(cmd).await
    }
    async fn get_details(
        &self,
        cmd: GetMediaDetailsCommand,
    ) -> Result<MediaDetailResult, MediaError> {
        self.get_details(cmd).await
    }
    async fn change_details(&self, cmd: ChangeMediaDetailsCommand) -> Result<(), MediaError> {
        self.change_details(cmd).await
    }
    async fn get_aliases(&self, cmd: GetAliasesCommand) -> Result<Vec<String>, MediaError> {
        self.get_aliases(cmd).await
    }
    async fn add_alias(&self, cmd: AddAliasCommand) -> Result<(), MediaError> {
        self.add_alias(cmd).await
    }
    async fn change_alias(&self, cmd: ChangeAliasCommand) -> Result<(), MediaError> {
        self.change_alias(cmd).await
    }
    async fn delete_alias(&self, cmd: DeleteAliasCommand) -> Result<(), MediaError> {
        self.delete_alias(cmd).await
    }
}
