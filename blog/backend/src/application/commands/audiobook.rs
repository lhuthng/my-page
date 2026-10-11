use crate::domain::entities::media::MediumDetails;

pub struct GetAudiobooksCommand {
    pub user_id: i64,
    pub is_admin: bool,
    pub term: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

pub struct GetPublicAudiobooksCommand {
    pub term: Option<String>,
    pub tag: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

pub struct GetAudiobookCommand {
    pub audiobook_id: i64,
    pub user_id: i64,
    pub is_admin: bool,
}

pub struct GetPublicAudiobookCommand {
    pub slug: String,
    /// Index of the first chapter to return, counting from the top of the
    /// chapter order.
    pub tracks_offset: i64,
    /// How many chapters to return. `None` means every remaining chapter, which
    /// is the whole list — the answer callers that predate windowed loading
    /// still get, and the one the dashboard editor relies on.
    pub tracks_limit: Option<i64>,
}

pub struct NewAudiobookCommand {
    pub user_id: i64,
    pub title: String,
    pub slug: String,
    pub description: String,
    pub translator: Option<String>,
    /// Dedicated audiobook tag names; normalized and de-duplicated on write.
    pub tags: Vec<String>,
    pub cover_image: Option<MediumDetails>,
}

pub struct UpdateAudiobookCommand {
    pub audiobook_id: i64,
    pub user_id: i64,
    pub is_admin: bool,
    pub title: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    /// `Some(None)` clears the translator, `Some(Some(_))` sets it, `None`
    /// leaves it untouched.
    pub translator: Option<Option<String>>,
    /// `None` leaves tags untouched; `Some(vec![])` clears them.
    pub tags: Option<Vec<String>>,
}

pub struct SetAudiobookCoverCommand {
    pub audiobook_id: i64,
    pub user_id: i64,
    pub is_admin: bool,
    pub medium: MediumDetails,
}

pub struct ChangeAudiobookStatusCommand {
    pub audiobook_id: i64,
    pub user_id: i64,
    pub is_admin: bool,
    pub status: String,
}

pub struct DeleteAudiobookCommand {
    pub audiobook_id: i64,
    pub user_id: i64,
    pub is_admin: bool,
}

pub struct AddTrackCommand {
    pub audiobook_id: i64,
    pub user_id: i64,
    pub is_admin: bool,
    pub title: String,
    /// The chapter's slot in the book. Gaps are normal — chapters 1 and 100
    /// can coexist — and with none given the chapter takes the next free slot
    /// past the highest one.
    pub number: Option<i64>,
    pub duration_seconds: Option<i64>,
    pub medium: MediumDetails,
}

pub struct UpdateTrackCommand {
    pub audiobook_id: i64,
    pub track_id: i64,
    pub user_id: i64,
    pub is_admin: bool,
    pub title: Option<String>,
    pub number: Option<i64>,
    pub duration_seconds: Option<i64>,
}

/// Swap a track's audio for a freshly uploaded file, keeping the chapter's
/// slot. Metadata that is not supplied is left untouched, and the audio being
/// replaced is released once nothing else is playing it.
pub struct ReplaceTrackMediumCommand {
    pub audiobook_id: i64,
    pub track_id: i64,
    pub user_id: i64,
    pub is_admin: bool,
    /// `Some(_)` renames the track; `None` keeps the current title.
    pub title: Option<String>,
    pub duration_seconds: Option<i64>,
    pub medium: MediumDetails,
}

pub struct RemoveTrackCommand {
    pub audiobook_id: i64,
    pub track_id: i64,
    pub user_id: i64,
    pub is_admin: bool,
}

pub struct ListAudiobookTagsCommand {
    pub user_id: i64,
    pub is_admin: bool,
    pub limit: i64,
    pub offset: i64,
}

pub struct CheckAudiobookSlugCommand {
    pub slug: String,
}

/// A chapter play report from the public player, sent once per ten seconds of
/// real playback. It carries nothing but the chapter: the caller's identity is
/// used only as an in-memory rate-limit key in the handler, never stored.
pub struct RecordTrackPlayCommand {
    pub track_id: i64,
}
