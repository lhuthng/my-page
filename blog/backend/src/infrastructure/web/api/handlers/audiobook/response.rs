// Audiobook wire responses and the conversions that build them from domain
// entities.
use serde::{Deserialize, Serialize};

use crate::domain::entities::audiobook::AudiobookSnapshot;

#[derive(Serialize, Deserialize)]
pub struct AudiobookSummaryResponse {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub description: String,
    pub translator: Option<String>,
    pub status: String,
    pub url: Option<String>,
    pub track_count: i64,
    pub total_duration_seconds: i64,
    pub tags: Vec<String>,
    pub tag_slugs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_display_name: Option<String>,
    pub created_at: String,
    pub published_at: Option<String>,
}

impl From<AudiobookSnapshot> for AudiobookSummaryResponse {
    fn from(s: AudiobookSnapshot) -> Self {
        Self {
            id: s.id,
            title: s.title,
            slug: s.slug,
            description: s.description,
            translator: s.translator,
            status: s.status,
            url: s.url,
            track_count: s.track_count,
            total_duration_seconds: s.total_duration_seconds,
            tags: s.tags,
            tag_slugs: s.tag_slugs,
            owner_username: s.owner_username,
            owner_display_name: s.owner_display_name,
            created_at: s.created_at,
            published_at: s.published_at,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct AudiobookListResponse {
    pub audiobooks: Vec<AudiobookSummaryResponse>,
}

#[derive(Serialize)]
pub struct AudiobookDetailsResponse {
    pub audiobook: AudiobookDetails,
}

#[derive(Serialize)]
pub struct AudiobookTagsResponse {
    pub tags: Vec<AudiobookTag>,
}

#[derive(Serialize, Deserialize)]
pub struct TrackCreatedResponse {
    pub track_id: i64,
}

#[derive(Serialize, Deserialize)]
pub struct AudiobookCreatedResponse {
    pub audiobook_id: i64,
}

#[derive(Serialize, Deserialize)]
pub struct SlugAvailabilityResponse {
    /// `true` when the slug is free to use.
    pub available: bool,
}

