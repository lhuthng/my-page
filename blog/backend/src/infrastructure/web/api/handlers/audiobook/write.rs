// Audiobook authoring: create, update, cover, status, delete.
use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Multipart, Path, State},
    response::IntoResponse,
};

use crate::{
    application::{
        commands::audiobook::{
            ChangeAudiobookStatusCommand, DeleteAudiobookCommand, NewAudiobookCommand,
            SetAudiobookCoverCommand, UpdateAudiobookCommand,
        },
        services::audiobook::AudiobookService,
    },
    domain::{
        entities::{audiobook::AudiobookDetails, media::MediumDetails, secret::Claims},
        errors::{audiobook::AudiobookError, media::MediaError},
    },
    infrastructure::web::{
        api::handlers::audiobook::shared::{caller_id, is_admin, read_text},
        api::handlers::audiobook::dto::{ChangeStatusPayload, UpdateAudiobookPayload},
        api::handlers::audiobook::response::{
            AudiobookCreatedResponse, AudiobookDetailsResponse, AudiobookSummaryResponse,
        },
        api::handlers::support::cover::extract_medium,
        server::AppState,
    },
};

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
