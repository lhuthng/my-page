// Audiobook request payloads and query params.
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ListQuery {
    pub term: Option<String>,
    pub tag: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Deserialize)]
pub struct SlugQuery {
    pub slug: String,
}

#[derive(Deserialize)]
pub struct UpdateAudiobookPayload {
    pub title: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    /// Sent as `null` to clear the translator; omit to leave it unchanged.
    #[serde(default, deserialize_with = "double_option")]
    pub translator: Option<Option<String>>,
    pub tags: Option<Vec<String>>,
}

/// Distinguish "field absent" from "field explicitly null".
///
/// `Option<Option<T>>` alone cannot do this: serde maps both to `None`. The
/// extra layer makes `{"translator": null}` clear the value while omitting the
/// key leaves it untouched.
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

#[derive(Deserialize)]
pub struct UpdateTrackPayload {
    pub title: Option<String>,
    pub number: Option<i64>,
    pub duration_seconds: Option<i64>,
}

#[derive(Deserialize)]
pub struct ReorderTracksPayload {
    pub order: Vec<i64>,
}

// ---------------------------------------------------------------------------
// Dashboard (moderator/admin) endpoints
// ---------------------------------------------------------------------------

