//! Wire types for the author API.
use serde::Deserialize;

/// Optional window on the chapter manifest. Omitted = the whole book, which is
/// what a hash diff needs. A partial window is explicit and always flagged.
#[derive(Deserialize)]
pub struct ManifestQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct ChapterUploadQuery {
    pub title: Option<String>,
    pub duration_seconds: Option<i64>,
    pub filename: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateChapterPayload {
    pub title: Option<String>,
    pub duration_seconds: Option<i64>,
}

#[derive(Deserialize)]
pub struct UpdateBookPayload {
    pub title: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub translator: Option<Option<String>>,
    pub tags: Option<Vec<String>>,
}

/// Distinguish "absent" from "explicitly null" so `translator: null` clears.
fn double_option<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Some(Option::<String>::deserialize(deserializer)?))
}

#[derive(Deserialize)]
pub struct ChangeStatusPayload {
    pub status: String,
}

/// Metadata-only batch update. Slots are not moved here: two chapters claiming
/// one slot in the same request would be ambiguous, so moves stay on the
/// single-chapter PATCH.
#[derive(Deserialize)]
pub struct BatchChaptersPayload {
    pub chapters: Vec<BatchChapterPayload>,
}

#[derive(Deserialize)]
pub struct BatchChapterPayload {
    pub number: i64,
    pub title: Option<String>,
    pub duration_seconds: Option<i64>,
}

#[derive(Deserialize)]
pub struct ArchiveQuery {
    /// `1-50,53,60-62`. Omitted = every chapter.
    pub numbers: Option<String>,
}

#[derive(Deserialize)]
pub struct TogglePayload {
    pub enabled: bool,
}

#[derive(Deserialize)]
pub struct CreateKeyPayload {
    pub label: Option<String>,
    /// Omitted or `null` = no expiry. When set, hours from now.
    pub ttl_hours: Option<u64>,
    /// `read` or `read,write`. Omitted = `read,write`.
    pub scopes: Option<String>,
}
