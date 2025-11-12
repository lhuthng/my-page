use crate::{application::commands, domain::errors, infrastructure::web::server::MediaConfig};

#[async_trait::async_trait]
pub trait MediaService {
    async fn upload(
        &self,
        cmd: commands::media::UploadMediaCommand,
        config: &MediaConfig,
    ) -> Result<(), errors::media::MediaError>;
}
