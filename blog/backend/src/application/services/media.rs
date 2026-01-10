use crate::{
    application::commands::{
        self,
        media::{
            AddAliasCommand, ChangeAliasCommand, ChangeMediaDetailsCommand, DeleteAliasCommand,
            GetAliasesCommand, GetMediaDetailsCommand,
        },
    },
    domain::{
        entities::{self, media::MediaDetailResult},
        errors::{self, media::MediaError},
    },
    infrastructure::web::server::MediaConfig,
};

#[async_trait::async_trait]
pub trait MediaService {
    async fn change_avatar(
        &self,
        cmd: commands::media::ChangeAvatarCommand,
        config: &MediaConfig,
    ) -> Result<(), errors::media::MediaError>;
    async fn change_post_cover(
        &self,
        cmd: commands::media::ChangePostCoverCommand,
        config: &MediaConfig,
    ) -> Result<(), errors::media::MediaError>;
    async fn upload(
        &self,
        cmd: commands::media::UploadMediumCommand,
        config: &MediaConfig,
    ) -> Result<(), errors::media::MediaError>;
    async fn bulk_upload(
        &self,
        cmd: commands::media::UploadMediaWithoutDescriptionCommand,
        config: &MediaConfig,
    ) -> Result<(), errors::media::MediaError>;
    async fn get_link(
        &self,
        cmd: commands::media::GetLinkCommand,
    ) -> Result<entities::media::LinkResult, MediaError>;
    async fn search(
        &self,
        cmd: commands::media::SearchMediaCommand,
    ) -> Result<Vec<entities::media::LinkResult>, MediaError>;
    async fn get_details(
        &self,
        cmd: GetMediaDetailsCommand,
    ) -> Result<MediaDetailResult, MediaError>;
    async fn change_details(&self, cmd: ChangeMediaDetailsCommand) -> Result<(), MediaError>;
    async fn get_aliases(&self, cmd: GetAliasesCommand) -> Result<Vec<String>, MediaError>;
    async fn add_alias(&self, cmd: AddAliasCommand) -> Result<(), MediaError>;
    async fn change_alias(&self, cmd: ChangeAliasCommand) -> Result<(), MediaError>;
    async fn delete_alias(&self, cmd: DeleteAliasCommand) -> Result<(), MediaError>;
}
