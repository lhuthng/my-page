// Media type allow-lists for uploads and avatars.
use std::str::FromStr;

use crate::domain::entities::media::MediaType;
use crate::domain::errors::media::MediaError;
use crate::infrastructure::web::server::MediaConfig;

use super::MediaServiceImpl;

impl MediaServiceImpl {
    pub(super) async fn is_supported(
        &self,
        file_type: &str,
        filename: &str,
        config: &MediaConfig,
    ) -> Result<bool, MediaError> {
        let media_type = MediaType::from_upload(file_type, filename)?;

        Ok(config.allowed_file_types.contains(&media_type))
    }
    pub(super) async fn is_avatar_supported(
        &self,
        file_type: &str,
        config: &MediaConfig,
    ) -> Result<bool, MediaError> {
        let media_type = MediaType::from_str(file_type)?;

        Ok(config.allowed_avatar_types.contains(&media_type))
    }
}
