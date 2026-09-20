// Audiobook validation rules and ownership guard.
use std::path::PathBuf;

use sqlx::Row;

use crate::domain::errors::audiobook::AudiobookError;

use super::AudiobookServiceImpl;
use super::rows::TrackRow;

const MAX_TITLE_CHARS: usize = 300;
const MAX_DESCRIPTION_CHARS: usize = 4000;
const MAX_TRANSLATOR_CHARS: usize = 200;
const MAX_TRACK_TITLE_CHARS: usize = 300;
const MAX_TAG_NAME_CHARS: usize = 60;
/// Upper bound on tracks per audiobook: the editor reorders in memory and the
/// player renders a flat playlist, so an unbounded count would let one request
/// pull an enormous payload.
const MAX_TRACKS_PER_AUDIOBOOK: i64 = 2000;
const MAX_TAGS_PER_AUDIOBOOK: usize = 25;


impl AudiobookServiceImpl {
    pub(super) async fn is_cover_supported(
        &self,
        file_type: &str,
        config: &MediaConfig,
    ) -> Result<bool, MediaError> {
        let media_type = MediaType::from_str(file_type)?;

        Ok(config.allowed_cover_types.contains(&media_type))
    }

    pub(super) async fn is_audio_supported(
        &self,
        file_type: &str,
        config: &MediaConfig,
    ) -> Result<bool, MediaError> {
        let media_type = MediaType::from_str(file_type)?;

        Ok(config.allowed_audio_types.contains(&media_type))
    }

    /// Confirm the caller may mutate `audiobook_id`, returning the owning user.
    ///
    /// Admins pass for any row; everyone else only for their own. A missing row
    /// is reported as `NotFound` so a probe cannot distinguish "not yours" from
    /// "does not exist".
    pub(super) async fn assert_owned(
        tx: &mut Transaction<'_, Sqlite>,
        audiobook_id: i64,
        user_id: i64,
        is_admin: bool,
    ) -> Result<(), AudiobookError> {
        let owner: Option<i64> =
            sqlx::query_scalar("SELECT user_id FROM audiobooks WHERE id = ?")
                .bind(audiobook_id)
                .fetch_optional(&mut **tx)
                .await?;

        match owner {
            None => Err(AudiobookError::NotFound),
            Some(owner) if is_admin || owner == user_id => Ok(()),
            Some(_) => Err(AudiobookError::PermissionDenied),
        }
    }

    /// Renumber an audiobook's tracks contiguously from 1, preserving the
    /// current (number, id) ordering. Called after every insert/remove so the
    /// player's "next track" is always simply `number + 1`.
}
