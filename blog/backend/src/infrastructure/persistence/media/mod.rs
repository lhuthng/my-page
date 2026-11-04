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
mod store;
mod upload;
mod validation;

#[cfg(test)]
mod tests;

pub use files::clean_up_files;
pub use hashing::{HashData, hash_bytes};
pub use store::{mirror_audio_to_bucket, write_chunked_audio};

pub const MEDIA_REFERENCES: &str = "
    SELECT media_id FROM audiobook_tracks
    UNION ALL SELECT media_id FROM media_aliases
    UNION ALL SELECT medium_id FROM post_media_usages
    UNION ALL SELECT cover_media_id FROM posts
    UNION ALL SELECT cover_image_id FROM series
    UNION ALL SELECT cover_image_id FROM audiobooks
    UNION ALL SELECT avatar_image_id FROM user_meta
";

/// Fills one slot per stored media code, leaving a hole where a code has no
/// surviving row.
///
/// Codes are positional and per-content (`@[img:2]`), but the usage query joins
/// `media`, so a usage whose media row is gone simply does not come back.
/// Sizing the result by row count would shift every later code down a slot or
/// reject the page outright; sizing by `max(code) + 1` keeps each surviving code
/// at the index the body refers to, and the client renders the hole as a missing
/// image instead of an empty source.
pub(crate) fn media_slots_by_code<T: Default>(rows: impl IntoIterator<Item = (i64, T)>) -> Vec<T> {
    let rows: Vec<(i64, T)> = rows.into_iter().filter(|(code, _)| *code >= 0).collect();
    let len = rows
        .iter()
        .map(|(code, _)| *code as usize + 1)
        .max()
        .unwrap_or(0);
    let mut slots: Vec<T> = (0..len).map(|_| T::default()).collect();
    for (code, value) in rows {
        slots[code as usize] = value;
    }
    slots
}

pub struct MediaServiceImpl {
    pub pool: SqlitePool,
}

/// What happened to a row handed to [`release_media`].
pub(super) enum ReleaseOutcome {
    /// Nothing else pointed at it, so the row and its file can go.
    Deleted,
    /// Something else still points at it, so only its slot name was freed.
    Archived,
}

/// Frees one fixed slot name (`.post.<id>`, `.avt.<user>`) in `tx`.
///
/// Replacement flows reuse a single `short_name`, so the previous row has to
/// leave that name before the new row is inserted. Deleting it outright also
/// cascades away `post_media_usages` rows and blanks the covers and avatars that
/// share it. Instead: delete when nothing else references the row, otherwise
/// rename it to `<old name>.replaced.<id>` so its id, url, and file survive.
/// Callers must clear their own owning pointer before calling this, or the row
/// will look referenced by the very slot being replaced.
pub(super) async fn release_media(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    media_id: i64,
) -> Result<ReleaseOutcome, sqlx::Error> {
    let referenced: bool = sqlx::query_scalar(&format!(
        "SELECT EXISTS(SELECT 1 FROM ({MEDIA_REFERENCES}) WHERE media_id = ?)"
    ))
    .bind(media_id)
    .fetch_one(&mut **tx)
    .await?;

    if referenced {
        sqlx::query(
            r#"
            UPDATE media
            SET short_name = short_name || '.replaced.' || id
            WHERE id = ?
            "#,
        )
        .bind(media_id)
        .execute(&mut **tx)
        .await?;
        Ok(ReleaseOutcome::Archived)
    } else {
        sqlx::query("DELETE FROM media WHERE id = ?")
            .bind(media_id)
            .execute(&mut **tx)
            .await?;
        Ok(ReleaseOutcome::Deleted)
    }
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

#[cfg(test)]
mod slot_tests {
    use super::media_slots_by_code;

    #[test]
    fn a_missing_code_leaves_a_hole_instead_of_shifting_later_media() {
        let slots = media_slots_by_code([(0, "a".to_string()), (2, "c".to_string())]);
        assert_eq!(slots, vec!["a".to_string(), String::new(), "c".to_string()]);
    }

    #[test]
    fn no_rows_make_no_slots() {
        assert!(media_slots_by_code(Vec::<(i64, String)>::new()).is_empty());
    }

    #[test]
    fn negative_codes_are_ignored_rather_than_indexing_out_of_bounds() {
        let slots = media_slots_by_code([(-1, "x".to_string()), (0, "a".to_string())]);
        assert_eq!(slots, vec!["a".to_string()]);
    }

    #[test]
    fn trailing_codes_keep_the_array_long_enough_to_index() {
        let slots = media_slots_by_code([(4, "e".to_string())]);
        assert_eq!(slots.len(), 5);
        assert_eq!(slots[4], "e");
    }
}
