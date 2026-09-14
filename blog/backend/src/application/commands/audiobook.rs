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
    /// 1-based playback position; appended to the end when omitted.
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

pub struct RemoveTrackCommand {
    pub audiobook_id: i64,
    pub track_id: i64,
    pub user_id: i64,
    pub is_admin: bool,
}

pub struct ReorderTracksCommand {
    pub audiobook_id: i64,
    pub user_id: i64,
    pub is_admin: bool,
    /// Track ids in their desired playback order. Must be a permutation of the
    /// audiobook's current tracks.
    pub order: Vec<i64>,
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
