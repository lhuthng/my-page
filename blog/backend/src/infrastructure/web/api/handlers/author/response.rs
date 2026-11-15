//! Responses for the author API.
use serde::Serialize;

/// The per-book switch. `enabled` is what the dashboard toggles; the counts
/// let the panel explain what switching it off would pause.
#[derive(Serialize)]
pub struct SettingsResponse {
    pub enabled: bool,
    pub key_count: i64,
    pub active_key_count: i64,
}

#[derive(Serialize)]
pub struct AuthorBookResponse {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub description: String,
    pub translator: Option<String>,
    pub status: String,
    pub tags: Vec<String>,
    pub total_duration_seconds: i64,
    pub created_at: String,
    pub updated_at: String,
    pub published_at: Option<String>,
}

#[derive(Serialize)]
pub struct AuthorChapterResponse {
    pub number: i64,
    pub track_id: i64,
    pub title: String,
    pub duration_seconds: Option<i64>,
    /// SHA-256 of the *uploaded* bytes. Audio is never transcoded, so this is
    /// the same digest the author's tooling computes locally.
    pub sha256: String,
    pub size: i64,
    pub file_type: String,
    pub play_count: i64,
    pub updated_at: Option<String>,
    /// Backend-relative download path, as with `media/i/...` in the player.
    pub download_url: String,
}

#[derive(Serialize)]
pub struct AuthorManifestResponse {
    pub audiobook: AuthorBookResponse,
    pub chapters: Vec<AuthorChapterResponse>,
    /// Total playable chapters, independent of the window above.
    pub chapter_count: i64,
    pub has_more_chapters: bool,
}

#[derive(Serialize)]
pub struct ChapterPutResponse {
    /// `created`, `replaced` or `unchanged`.
    pub status: &'static str,
    pub number: i64,
    pub sha256: String,
    pub size: i64,
    pub track_id: i64,
}

#[derive(Serialize)]
pub struct BatchChaptersResponse {
    pub updated: Vec<i64>,
    pub skipped: Vec<i64>,
}

#[derive(Serialize)]
pub struct CreatedKeyResponse {
    pub id: i64,
    pub label: String,
    pub hint: String,
    pub scopes: String,
    pub expires_at: Option<String>,
    /// The full `abk_…` secret. Returned exactly once and never stored.
    pub key: String,
}

#[derive(Serialize)]
pub struct KeyInfoResponse {
    pub id: i64,
    pub label: String,
    pub hint: String,
    pub scopes: String,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
    pub last_used_at: Option<String>,
}

#[derive(Serialize)]
pub struct KeyListResponse {
    pub keys: Vec<KeyInfoResponse>,
}
