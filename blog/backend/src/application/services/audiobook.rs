use crate::{
    application::commands,
    domain::{entities, errors},
    infrastructure::web::server::MediaConfig,
};

#[async_trait::async_trait]
pub trait AudiobookService {
    /// Dashboard listing: all audiobooks for admins, only the caller's own
    /// otherwise.
    async fn get_audiobooks(
        &self,
        cmd: commands::audiobook::GetAudiobooksCommand,
    ) -> Result<Vec<entities::audiobook::AudiobookSnapshot>, errors::audiobook::AudiobookError>;

    /// Public catalogue: published audiobooks only.
    async fn get_public_audiobooks(
        &self,
        cmd: commands::audiobook::GetPublicAudiobooksCommand,
    ) -> Result<Vec<entities::audiobook::AudiobookSnapshot>, errors::audiobook::AudiobookError>;

    /// Full details including ordered tracks, for the dashboard editor.
    async fn get_audiobook(
        &self,
        cmd: commands::audiobook::GetAudiobookCommand,
    ) -> Result<entities::audiobook::AudiobookDetails, errors::audiobook::AudiobookError>;

    /// Full details including ordered tracks, for the public player.
    async fn get_public_audiobook(
        &self,
        cmd: commands::audiobook::GetPublicAudiobookCommand,
    ) -> Result<entities::audiobook::AudiobookDetails, errors::audiobook::AudiobookError>;

    async fn new_audiobook(
        &self,
        cmd: commands::audiobook::NewAudiobookCommand,
        config: &MediaConfig,
    ) -> Result<i64, errors::audiobook::AudiobookError>;

    async fn update_audiobook(
        &self,
        cmd: commands::audiobook::UpdateAudiobookCommand,
    ) -> Result<(), errors::audiobook::AudiobookError>;

    async fn set_audiobook_cover(
        &self,
        cmd: commands::audiobook::SetAudiobookCoverCommand,
        config: &MediaConfig,
    ) -> Result<(), errors::audiobook::AudiobookError>;

    async fn change_audiobook_status(
        &self,
        cmd: commands::audiobook::ChangeAudiobookStatusCommand,
    ) -> Result<(), errors::audiobook::AudiobookError>;

    async fn delete_audiobook(
        &self,
        cmd: commands::audiobook::DeleteAudiobookCommand,
    ) -> Result<(), errors::audiobook::AudiobookError>;

    async fn add_track(
        &self,
        cmd: commands::audiobook::AddTrackCommand,
        config: &MediaConfig,
    ) -> Result<i64, errors::audiobook::AudiobookError>;

    async fn update_track(
        &self,
        cmd: commands::audiobook::UpdateTrackCommand,
    ) -> Result<(), errors::audiobook::AudiobookError>;

    /// Replace a track's audio file in place, keeping its playlist position.
    async fn replace_track_medium(
        &self,
        cmd: commands::audiobook::ReplaceTrackMediumCommand,
        config: &MediaConfig,
    ) -> Result<(), errors::audiobook::AudiobookError>;

    async fn remove_track(
        &self,
        cmd: commands::audiobook::RemoveTrackCommand,
    ) -> Result<(), errors::audiobook::AudiobookError>;

    async fn reorder_tracks(
        &self,
        cmd: commands::audiobook::ReorderTracksCommand,
    ) -> Result<(), errors::audiobook::AudiobookError>;

    async fn list_audiobook_tags(
        &self,
        cmd: commands::audiobook::ListAudiobookTagsCommand,
    ) -> Result<Vec<entities::audiobook::AudiobookTag>, errors::audiobook::AudiobookError>;

    async fn check_audiobook_slug(
        &self,
        cmd: commands::audiobook::CheckAudiobookSlugCommand,
    ) -> Result<bool, errors::audiobook::AudiobookError>;
}
