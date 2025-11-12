use sqlx::SqlitePool;

use crate::{
    application::{commands::media::UploadMediaCommand, services::media::MediaService},
    domain::errors::media::MediaError,
    infrastructure::web::server::MediaConfig,
};

pub struct MediaServiceImpl {
    pub pool: SqlitePool,
}

impl MediaServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl MediaService for MediaServiceImpl {
    async fn upload(
        &self,
        cmd: UploadMediaCommand,
        config: &MediaConfig,
    ) -> Result<(), MediaError> {
        Err(MediaError::PermissionDenied)
    }
}
