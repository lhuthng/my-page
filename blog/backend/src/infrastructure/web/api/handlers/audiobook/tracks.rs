// Track management: add, edit, remove, reorder.
use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Multipart, Path, State},
    response::IntoResponse,
};

use crate::{
    application::{
        commands::audiobook::{
            AddTrackCommand, RemoveTrackCommand, ReorderTracksCommand, UpdateTrackCommand,
        },
        services::audiobook::AudiobookService,
    },
    domain::{
        entities::audiobook::AudiobookTag,
        secret::Claims,
        errors::audiobook::AudiobookError,
    },
    infrastructure::web::{
        api::handlers::audiobook::shared::{caller_id, is_admin, read_text},
        api::handlers::audiobook::dto::{ReorderTracksPayload, UpdateTrackPayload},
        api::handlers::audiobook::response::TrackCreatedResponse,
        server::AppState,
    },
};

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

