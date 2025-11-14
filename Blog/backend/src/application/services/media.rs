use crate::{
    application::commands,
    domain::errors::{self, media::MediaError},
    infrastructure::web::server::MediaConfig,
};

#[async_trait::async_trait]
pub trait MediaService {
    async fn is_supported(&self, file_type: &str, config: &MediaConfig)
    -> Result<bool, MediaError>;
    async fn upload(
        &self,
        cmd: commands::media::UploadMediaCommand,
        config: &MediaConfig,
    ) -> Result<(), errors::media::MediaError>;
}
