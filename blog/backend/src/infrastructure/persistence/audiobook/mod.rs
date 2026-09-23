// Audiobook persistence adapter: implements
// `application::services::audiobook::AudiobookService` against SQLite.
use sqlx::SqlitePool;

mod mapping;
mod medium;
mod read;
mod rows;
mod tags;
mod tracks;
mod validation;
mod write;

pub struct AudiobookServiceImpl {
    pub pool: SqlitePool,
}

impl AudiobookServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

// `AudiobookService` port implementation: thin delegation to the inherent
// methods in the sibling files (inherent candidates win over trait
// candidates in method resolution, so `self.<method>` calls the local one).
use crate::application::commands;
use crate::application::services::audiobook::AudiobookService;
use crate::domain::{entities, errors};
use crate::infrastructure::web::server::MediaConfig;

#[async_trait::async_trait]
impl AudiobookService for AudiobookServiceImpl {
    async fn get_audiobooks(
        &self,
        cmd: commands::audiobook::GetAudiobooksCommand,
    ) -> Result<Vec<entities::audiobook::AudiobookSnapshot>, errors::audiobook::AudiobookError>
    {
        self.get_audiobooks(cmd).await
    }
    async fn get_public_audiobooks(
        &self,
        cmd: commands::audiobook::GetPublicAudiobooksCommand,
    ) -> Result<Vec<entities::audiobook::AudiobookSnapshot>, errors::audiobook::AudiobookError>
    {
        self.get_public_audiobooks(cmd).await
    }
    async fn get_audiobook(
        &self,
        cmd: commands::audiobook::GetAudiobookCommand,
    ) -> Result<entities::audiobook::AudiobookDetails, errors::audiobook::AudiobookError> {
        self.get_audiobook(cmd).await
    }
    async fn get_public_audiobook(
        &self,
        cmd: commands::audiobook::GetPublicAudiobookCommand,
    ) -> Result<entities::audiobook::AudiobookDetails, errors::audiobook::AudiobookError> {
        self.get_public_audiobook(cmd).await
    }
    async fn new_audiobook(
        &self,
        cmd: commands::audiobook::NewAudiobookCommand,
        config: &MediaConfig,
    ) -> Result<i64, errors::audiobook::AudiobookError> {
        self.new_audiobook(cmd, config).await
    }
    async fn update_audiobook(
        &self,
        cmd: commands::audiobook::UpdateAudiobookCommand,
    ) -> Result<(), errors::audiobook::AudiobookError> {
        self.update_audiobook(cmd).await
    }
    async fn set_audiobook_cover(
        &self,
        cmd: commands::audiobook::SetAudiobookCoverCommand,
        config: &MediaConfig,
    ) -> Result<(), errors::audiobook::AudiobookError> {
        self.set_audiobook_cover(cmd, config).await
    }
    async fn change_audiobook_status(
        &self,
        cmd: commands::audiobook::ChangeAudiobookStatusCommand,
    ) -> Result<(), errors::audiobook::AudiobookError> {
        self.change_audiobook_status(cmd).await
    }
    async fn delete_audiobook(
        &self,
        cmd: commands::audiobook::DeleteAudiobookCommand,
    ) -> Result<(), errors::audiobook::AudiobookError> {
        self.delete_audiobook(cmd).await
    }
    async fn add_track(
        &self,
        cmd: commands::audiobook::AddTrackCommand,
        config: &MediaConfig,
    ) -> Result<i64, errors::audiobook::AudiobookError> {
        self.add_track(cmd, config).await
    }
    async fn update_track(
        &self,
        cmd: commands::audiobook::UpdateTrackCommand,
    ) -> Result<(), errors::audiobook::AudiobookError> {
        self.update_track(cmd).await
    }
    async fn replace_track_medium(
        &self,
        cmd: commands::audiobook::ReplaceTrackMediumCommand,
        config: &MediaConfig,
    ) -> Result<(), errors::audiobook::AudiobookError> {
        self.replace_track_medium(cmd, config).await
    }
    async fn remove_track(
        &self,
        cmd: commands::audiobook::RemoveTrackCommand,
    ) -> Result<(), errors::audiobook::AudiobookError> {
        self.remove_track(cmd).await
    }
    async fn reorder_tracks(
        &self,
        cmd: commands::audiobook::ReorderTracksCommand,
    ) -> Result<(), errors::audiobook::AudiobookError> {
        self.reorder_tracks(cmd).await
    }
    async fn list_audiobook_tags(
        &self,
        cmd: commands::audiobook::ListAudiobookTagsCommand,
    ) -> Result<Vec<entities::audiobook::AudiobookTag>, errors::audiobook::AudiobookError> {
        self.list_audiobook_tags(cmd).await
    }
    async fn check_audiobook_slug(
        &self,
        cmd: commands::audiobook::CheckAudiobookSlugCommand,
    ) -> Result<bool, errors::audiobook::AudiobookError> {
        self.check_audiobook_slug(cmd).await
    }
}
