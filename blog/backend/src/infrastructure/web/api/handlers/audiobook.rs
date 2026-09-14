use std::sync::Arc;

use axum::{
    Extension, Json,
    body::Bytes,
    extract::{Multipart, Path, Query, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::{
    application::{
        commands::audiobook::{
            AddTrackCommand, ChangeAudiobookStatusCommand, CheckAudiobookSlugCommand,
            DeleteAudiobookCommand, GetAudiobookCommand, GetAudiobooksCommand,
            GetPublicAudiobookCommand, GetPublicAudiobooksCommand, ListAudiobookTagsCommand,
            NewAudiobookCommand, RemoveTrackCommand, ReorderTracksCommand, SetAudiobookCoverCommand,
            UpdateAudiobookCommand, UpdateTrackCommand,
        },
        services::audiobook::AudiobookService,
    },
    domain::{
        entities::{
            audiobook::{AudiobookDetails, AudiobookSnapshot, AudiobookTag},
            media::MediumDetails,
            secret::Claims,
        },
        errors::{audiobook::AudiobookError, media::MediaError},
    },
    infrastructure::web::{
        api::handlers::common::extract_medium,
        server::AppState,
    },
};

fn caller_id(claims: &Claims) -> Result<i64, AudiobookError> {
    claims
        .user_id
        .parse::<i64>()
        .map_err(|_| AudiobookError::InternalError("Cannot parse id".to_string()))
}

fn is_admin(claims: &Claims) -> bool {
    claims.role == "admin"
}

/// Clamp a caller-supplied page window so a request cannot pull the whole table.
fn page_window(limit: Option<i64>, offset: Option<i64>, default: i64) -> (i64, i64) {
    (
        crate::helper::string::clamp_page_size(limit, default, 100),
        crate::helper::string::clamp_offset(offset),
    )
}

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

pub async fn get_audiobooks(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<ListQuery>,
) -> Result<impl IntoResponse, AudiobookError> {
    let (limit, offset) = page_window(query.limit, query.offset, 50);

    let audiobooks = state
        .audiobook_service
        .get_audiobooks(GetAudiobooksCommand {
            user_id: caller_id(&claims)?,
            is_admin: is_admin(&claims),
            term: query.term,
            limit,
            offset,
        })
        .await?;

    Ok(Json(AudiobookListResponse {
        audiobooks: audiobooks.into_iter().map(Into::into).collect(),
    }))
}

pub async fn get_audiobook_details(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(audiobook_id): Path<i64>,
) -> Result<impl IntoResponse, AudiobookError> {
    let audiobook = state
        .audiobook_service
        .get_audiobook(GetAudiobookCommand {
            audiobook_id,
            user_id: caller_id(&claims)?,
            is_admin: is_admin(&claims),
        })
        .await?;

    Ok(Json(AudiobookDetailsResponse { audiobook }))
}

pub async fn list_tags(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<ListQuery>,
) -> Result<impl IntoResponse, AudiobookError> {
    let (limit, offset) = page_window(query.limit, query.offset, 100);

    let tags = state
        .audiobook_service
        .list_audiobook_tags(ListAudiobookTagsCommand {
            user_id: caller_id(&claims)?,
            is_admin: is_admin(&claims),
            limit,
            offset,
        })
        .await?;

    Ok(Json(AudiobookTagsResponse { tags }))
}

/// Create an audiobook. Multipart so the optional cover ships in the same
/// request as the metadata.
pub async fn new_audiobook(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AudiobookError> {
    let user_id = caller_id(&claims)?;

    let mut opt_title: Option<String> = None;
    let mut opt_slug: Option<String> = None;
    let mut opt_description: Option<String> = None;
    let mut opt_translator: Option<String> = None;
    let mut tags: Vec<String> = Vec::new();
    let mut opt_filename: Option<String> = None;
    let mut opt_content_type: Option<String> = None;
    let mut opt_bytes: Option<Bytes> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AudiobookError::InternalError(e.to_string()))?
    {
        let field_name = field.name().ok_or(AudiobookError::Media(
            MediaError::UploadFailed("Empty field detected.".to_string()),
        ))?;

        match field_name {
            "file" => {
                if opt_filename.is_some() {
                    return Err(AudiobookError::Media(MediaError::UploadFailed(
                        "Only one cover is allowed at a time.".to_string(),
                    )));
                }
                let medium = extract_medium(field).await?;
                opt_filename = Some(medium.filename);
                opt_content_type = Some(medium.content_type);
                opt_bytes = Some(medium.bytes);
            }
            "title" => opt_title = Some(read_text(field).await?),
            "slug" => opt_slug = Some(read_text(field).await?),
            "description" => opt_description = Some(read_text(field).await?),
            "translator" => opt_translator = Some(read_text(field).await?),
            // Repeated field: one entry per tag.
            "tags" => tags.push(read_text(field).await?),
            _ => {}
        }
    }

    let cover_image: Option<MediumDetails> = match (opt_filename, opt_content_type, opt_bytes) {
        (Some(filename), Some(content_type), Some(bytes)) => Some(MediumDetails {
            filename,
            content_type,
            bytes,
        }),
        _ => None,
    };

    let audiobook_id = state
        .audiobook_service
        .new_audiobook(
            NewAudiobookCommand {
                user_id,
                title: opt_title
                    .ok_or_else(|| AudiobookError::Validation("Missing title.".to_string()))?,
                slug: opt_slug
                    .ok_or_else(|| AudiobookError::Validation("Missing slug.".to_string()))?,
                description: opt_description.unwrap_or_default(),
                translator: opt_translator,
                tags,
                cover_image,
            },
            &state.media_config,
        )
        .await?;

    Ok(Json(AudiobookCreatedResponse { audiobook_id }))
}

pub async fn update_audiobook(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(audiobook_id): Path<i64>,
    Json(payload): Json<UpdateAudiobookPayload>,
) -> Result<(), AudiobookError> {
    state
        .audiobook_service
        .update_audiobook(UpdateAudiobookCommand {
            audiobook_id,
            user_id: caller_id(&claims)?,
            is_admin: is_admin(&claims),
            title: payload.title,
            slug: payload.slug,
            description: payload.description,
            translator: payload.translator,
            tags: payload.tags,
        })
        .await
}

pub async fn change_cover(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(audiobook_id): Path<i64>,
    mut multipart: Multipart,
) -> Result<(), AudiobookError> {
    let mut opt_filename: Option<String> = None;
    let mut opt_content_type: Option<String> = None;
    let mut opt_bytes: Option<Bytes> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AudiobookError::InternalError(e.to_string()))?
    {
        if field.name() == Some("file") {
            if opt_filename.is_some() {
                return Err(AudiobookError::Media(MediaError::UploadFailed(
                    "Only one cover is allowed at a time.".to_string(),
                )));
            }
            let medium = extract_medium(field).await?;
            opt_filename = Some(medium.filename);
            opt_content_type = Some(medium.content_type);
            opt_bytes = Some(medium.bytes);
        }
    }

    let (filename, content_type, bytes) = match (opt_filename, opt_content_type, opt_bytes) {
        (Some(filename), Some(content_type), Some(bytes)) => (filename, content_type, bytes),
        _ => {
            return Err(AudiobookError::Media(MediaError::UploadFailed(
                "Missing cover file.".to_string(),
            )));
        }
    };

    state
        .audiobook_service
        .set_audiobook_cover(
            SetAudiobookCoverCommand {
                audiobook_id,
                user_id: caller_id(&claims)?,
                is_admin: is_admin(&claims),
                medium: MediumDetails {
                    filename,
                    content_type,
                    bytes,
                },
            },
            &state.media_config,
        )
        .await
}

pub async fn change_status(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(audiobook_id): Path<i64>,
    Json(payload): Json<ChangeStatusPayload>,
) -> Result<(), AudiobookError> {
    state
        .audiobook_service
        .change_audiobook_status(ChangeAudiobookStatusCommand {
            audiobook_id,
            user_id: caller_id(&claims)?,
            is_admin: is_admin(&claims),
            status: payload.status,
        })
        .await
}

pub async fn delete_audiobook(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(audiobook_id): Path<i64>,
) -> Result<(), AudiobookError> {
    state
        .audiobook_service
        .delete_audiobook(DeleteAudiobookCommand {
            audiobook_id,
            user_id: caller_id(&claims)?,
            is_admin: is_admin(&claims),
        })
        .await
}

// ---------------------------------------------------------------------------
// Track endpoints
// ---------------------------------------------------------------------------

/// Upload one audio track. Multipart carries the audio plus its title, the
/// optional playback position, and the browser-probed duration.
pub async fn add_track(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(audiobook_id): Path<i64>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AudiobookError> {
    let mut opt_title: Option<String> = None;
    let mut opt_number: Option<i64> = None;
    let mut opt_duration: Option<i64> = None;
    let mut opt_filename: Option<String> = None;
    let mut opt_content_type: Option<String> = None;
    let mut opt_bytes: Option<Bytes> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AudiobookError::InternalError(e.to_string()))?
    {
        let field_name = field.name().ok_or(AudiobookError::Media(
            MediaError::UploadFailed("Empty field detected.".to_string()),
        ))?;

        match field_name {
            "file" => {
                if opt_filename.is_some() {
                    return Err(AudiobookError::Media(MediaError::UploadFailed(
                        "Only one audio file is allowed at a time.".to_string(),
                    )));
                }
                let medium = extract_medium(field).await?;
                opt_filename = Some(medium.filename);
                opt_content_type = Some(medium.content_type);
                opt_bytes = Some(medium.bytes);
            }
            "title" => opt_title = Some(read_text(field).await?),
            "number" => opt_number = read_text(field).await?.trim().parse::<i64>().ok(),
            "duration_seconds" => {
                opt_duration = read_text(field).await?.trim().parse::<i64>().ok();
            }
            _ => {}
        }
    }

    let (filename, content_type, bytes) = match (opt_filename, opt_content_type, opt_bytes) {
        (Some(filename), Some(content_type), Some(bytes)) => (filename, content_type, bytes),
        _ => {
            return Err(AudiobookError::Media(MediaError::UploadFailed(
                "Missing audio file.".to_string(),
            )));
        }
    };

    // Fall back to the uploaded file name when the author leaves the title
    // blank, so a track is never nameless in the playlist.
    let title = opt_title
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| filename.clone());

    let track_id = state
        .audiobook_service
        .add_track(
            AddTrackCommand {
                audiobook_id,
                user_id: caller_id(&claims)?,
                is_admin: is_admin(&claims),
                title,
                number: opt_number,
                duration_seconds: opt_duration,
                medium: MediumDetails {
                    filename,
                    content_type,
                    bytes,
                },
            },
            &state.media_config,
        )
        .await?;

    Ok(Json(TrackCreatedResponse { track_id }))
}

pub async fn update_track(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path((audiobook_id, track_id)): Path<(i64, i64)>,
    Json(payload): Json<UpdateTrackPayload>,
) -> Result<(), AudiobookError> {
    state
        .audiobook_service
        .update_track(UpdateTrackCommand {
            audiobook_id,
            track_id,
            user_id: caller_id(&claims)?,
            is_admin: is_admin(&claims),
            title: payload.title,
            number: payload.number,
            duration_seconds: payload.duration_seconds,
        })
        .await
}

pub async fn remove_track(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path((audiobook_id, track_id)): Path<(i64, i64)>,
) -> Result<(), AudiobookError> {
    state
        .audiobook_service
        .remove_track(RemoveTrackCommand {
            audiobook_id,
            track_id,
            user_id: caller_id(&claims)?,
            is_admin: is_admin(&claims),
        })
        .await
}

pub async fn reorder_tracks(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(audiobook_id): Path<i64>,
    Json(payload): Json<ReorderTracksPayload>,
) -> Result<(), AudiobookError> {
    state
        .audiobook_service
        .reorder_tracks(ReorderTracksCommand {
            audiobook_id,
            user_id: caller_id(&claims)?,
            is_admin: is_admin(&claims),
            order: payload.order,
        })
        .await
}

// ---------------------------------------------------------------------------
// Public endpoints
// ---------------------------------------------------------------------------

pub async fn check_slug(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SlugQuery>,
) -> Result<impl IntoResponse, AudiobookError> {
    let available = state
        .audiobook_service
        .check_audiobook_slug(CheckAudiobookSlugCommand { slug: query.slug })
        .await?;

    Ok(Json(SlugAvailabilityResponse { available }))
}

pub async fn get_public_audiobooks(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListQuery>,
) -> Result<impl IntoResponse, AudiobookError> {
    let (limit, offset) = page_window(query.limit, query.offset, 24);

    let audiobooks = state
        .audiobook_service
        .get_public_audiobooks(GetPublicAudiobooksCommand {
            term: query.term,
            tag: query.tag,
            limit,
            offset,
        })
        .await?;

    Ok(Json(AudiobookListResponse {
        audiobooks: audiobooks.into_iter().map(Into::into).collect(),
    }))
}

pub async fn get_public_audiobook(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, AudiobookError> {
    let audiobook = state
        .audiobook_service
        .get_public_audiobook(GetPublicAudiobookCommand { slug })
        .await?;

    Ok(Json(AudiobookDetailsResponse { audiobook }))
}

/// Read a multipart text field, mapping transport failures onto our error type.
async fn read_text(field: axum::extract::multipart::Field<'_>) -> Result<String, AudiobookError> {
    field
        .text()
        .await
        .map_err(|e| AudiobookError::Validation(format!("Cannot read field: {e}")))
}
