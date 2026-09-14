use serde::Serialize;

/// A summary row for the dashboard list and the public catalogue.
///
/// `url` is the backend-relative media path (`media/i/<short_name>`); the
/// frontend re-roots it through `fixClientRoute` before rendering.
#[derive(Debug, Clone)]
pub struct AudiobookSnapshot {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub description: String,
    pub translator: Option<String>,
    pub status: String,
    pub url: Option<String>,
    pub track_count: i64,
    pub total_duration_seconds: i64,
    pub owner_username: Option<String>,
    pub owner_display_name: Option<String>,
    pub tags: Vec<String>,
    pub tag_slugs: Vec<String>,
    pub created_at: String,
    pub published_at: Option<String>,
}

/// An audiobook-scoped tag. Deliberately separate from the global `tags`
/// vocabulary so audiobook taxonomy never leaks into post/project listings.
#[derive(Debug, Clone, Serialize)]
pub struct AudiobookTag {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub audiobook_count: i64,
}

/// One playable track.
///
/// `short_name` is the media handle used to build the streaming URL
/// (`media/i/<short_name>`), which the backend serves with HTTP Range support
/// so the browser can seek and buffer without downloading the whole file.
#[derive(Debug, Clone, Serialize)]
pub struct AudiobookTrack {
    pub id: i64,
    pub title: String,
    pub number: i64,
    pub duration_seconds: Option<i64>,
    pub short_name: String,
    pub url: String,
    pub file_type: String,
}

/// A full audiobook with its ordered tracks and tags, as served to the player.
#[derive(Debug, Clone, Serialize)]
pub struct AudiobookDetails {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub description: String,
    pub translator: Option<String>,
    pub status: String,
    pub url: Option<String>,
    pub total_duration_seconds: i64,
    pub owner_username: String,
    pub owner_display_name: String,
    pub tags: Vec<AudiobookTag>,
    pub tracks: Vec<AudiobookTrack>,
    pub created_at: String,
    pub published_at: Option<String>,
}

/// Decoded storage layout for a track's media row, used to rebuild the
/// on-disk path when the upload is written or re-read.
#[derive(Debug, Clone)]
pub struct TrackMediaInfo {
    pub short_name: String,
    pub file_type: String,
    pub url: String,
}
