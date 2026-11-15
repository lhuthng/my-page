//! Idempotent chapter writes plus metadata edits.
use std::{str::FromStr, sync::Arc};

use axum::{
    Extension, Json,
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
};

use crate::{
    application::{
        commands::audiobook::{
            AddTrackCommand, ChangeAudiobookStatusCommand, RemoveTrackCommand,
            ReplaceTrackMediumCommand, UpdateAudiobookCommand, UpdateTrackCommand,
        },
        services::audiobook::AudiobookService,
    },
    domain::{
        entities::media::{MediaType, MediumDetails},
        errors::author::AuthorError,
    },
    helper::string::validate_text,
    infrastructure::{
        author_keys::normalize_sha256,
        web::{
            api::{
                handlers::author::{
                    dto::{
                        BatchChaptersPayload, ChangeStatusPayload, ChapterUploadQuery,
                        UpdateBookPayload, UpdateChapterPayload,
                    },
                    response::{BatchChaptersResponse, ChapterPutResponse},
                    shared::{
                        assert_key_scope, load_chapter, map_audiobook_error,
                        normalize_content_type, pool, sha256_hex,
                    },
                },
                middlewares::auth::AuthorKeyAuth,
            },
            server::AppState,
        },
    },
};

const MAX_BATCH_CHAPTERS: usize = 2000;
const MAX_TITLE_CHARS: usize = 300;

/// `PUT …/chapters/{number}` — create, replace, or do nothing.
///
/// The same-hash check happens here, **before** any service call. Routing an
/// unchanged upload through `store_medium` would skip the disk write but still
/// re-run the zstd chunk encoder (`chunked::write` has no early exit), burning
/// full CPU on bytes that are already stored.
pub async fn put_chapter(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthorKeyAuth>,
    Path((audiobook_id, number)): Path<(i64, i64)>,
    Query(query): Query<ChapterUploadQuery>,
    headers: HeaderMap,
    bytes: Bytes,
) -> Result<(StatusCode, Json<ChapterPutResponse>), AuthorError> {
    assert_key_scope(&auth, audiobook_id)?;
    if number < 1 {
        return Err(AuthorError::Validation(
            "A chapter number must be 1 or greater.".to_string(),
        ));
    }
    if bytes.is_empty() {
        return Err(AuthorError::Validation(
            "The upload body is empty.".to_string(),
        ));
    }

    let size = bytes.len() as i64;
    let actual = sha256_hex(&bytes).await?;
    if let Some(declared) = headers
        .get("x-content-sha256")
        .and_then(|value| value.to_str().ok())
    {
        let expected = normalize_sha256(declared).ok_or_else(|| {
            AuthorError::Validation(
                "X-Content-SHA256 must be a 64-character hex SHA-256 digest.".to_string(),
            )
        })?;
        if expected != actual {
            return Err(AuthorError::HashMismatch { expected, actual });
        }
    }

    let existing = load_chapter(pool(&state), audiobook_id, number).await?;
    if let Some(ref chapter) = existing
        && chapter.hash == actual
    {
        return Ok((
            StatusCode::OK,
            Json(ChapterPutResponse {
                status: "unchanged",
                number,
                sha256: actual,
                size: chapter.size,
                track_id: chapter.track_id,
            }),
        ));
    }

    let content_type = normalize_content_type(
        headers
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
    );
    let extension = MediaType::from_str(&content_type)
        .map(|media_type| media_type.get_extension())
        .unwrap_or(".mp3");
    let filename = query
        .filename
        .clone()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| format!("chapter-{number}{extension}"));
    let opt_title = query.title.clone();
    let title = opt_title
        .clone()
        .unwrap_or_else(|| format!("Chapter {number}"));

    let medium = MediumDetails {
        filename,
        content_type,
        bytes,
    };

    let (status, http_status, track_id) = if let Some(chapter) = existing {
        state
            .audiobook_service
            .replace_track_medium(
                ReplaceTrackMediumCommand {
                    audiobook_id,
                    track_id: chapter.track_id,
                    user_id: auth.user_id,
                    is_admin: false,
                    // Omitted on a replace means "keep the existing title".
                    title: opt_title,
                    duration_seconds: query.duration_seconds,
                    medium,
                },
                &state.media_config,
            )
            .await
            .map_err(map_audiobook_error)?;
        ("replaced", StatusCode::OK, chapter.track_id)
    } else {
        let track_id = state
            .audiobook_service
            .add_track(
                AddTrackCommand {
                    audiobook_id,
                    user_id: auth.user_id,
                    is_admin: false,
                    title,
                    number: Some(number),
                    duration_seconds: query.duration_seconds,
                    medium,
                },
                &state.media_config,
            )
            .await
            .map_err(map_audiobook_error)?;
        ("created", StatusCode::CREATED, track_id)
    };

    Ok((
        http_status,
        Json(ChapterPutResponse {
            status,
            number,
            sha256: actual,
            size,
            track_id,
        }),
    ))
}

/// Metadata-only batch update. Chapters that do not exist yet are reported in
/// `skipped` — a chapter cannot exist without audio bytes, so those need a PUT.
pub async fn put_chapters_batch(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthorKeyAuth>,
    Path(audiobook_id): Path<i64>,
    Json(payload): Json<BatchChaptersPayload>,
) -> Result<Json<BatchChaptersResponse>, AuthorError> {
    assert_key_scope(&auth, audiobook_id)?;
    if payload.chapters.len() > MAX_BATCH_CHAPTERS {
        return Err(AuthorError::Validation(format!(
            "At most {MAX_BATCH_CHAPTERS} chapters per batch."
        )));
    }

    let mut tx = pool(&state).begin().await?;
    let mut updated = Vec::new();
    let mut skipped = Vec::new();

    for entry in &payload.chapters {
        if entry.number < 1 || (entry.title.is_none() && entry.duration_seconds.is_none()) {
            skipped.push(entry.number);
            continue;
        }
        let title = match entry.title.as_deref() {
            Some(raw) => Some(
                validate_text(raw, "Chapter title", MAX_TITLE_CHARS)
                    .map_err(AuthorError::Validation)?,
            ),
            None => None,
        };
        let track_id: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM audiobook_tracks WHERE audiobook_id = ? AND number = ?",
        )
        .bind(audiobook_id)
        .bind(entry.number)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(track_id) = track_id else {
            skipped.push(entry.number);
            continue;
        };
        sqlx::query(
            "UPDATE audiobook_tracks
             SET title = COALESCE(?, title),
                 duration_seconds = COALESCE(?, duration_seconds),
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?",
        )
        .bind(title)
        .bind(entry.duration_seconds)
        .bind(track_id)
        .execute(&mut *tx)
        .await?;
        updated.push(entry.number);
    }

    tx.commit().await?;
    Ok(Json(BatchChaptersResponse { updated, skipped }))
}

pub async fn patch_chapter(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthorKeyAuth>,
    Path((audiobook_id, number)): Path<(i64, i64)>,
    Json(payload): Json<UpdateChapterPayload>,
) -> Result<StatusCode, AuthorError> {
    assert_key_scope(&auth, audiobook_id)?;
    let chapter = load_chapter(pool(&state), audiobook_id, number)
        .await?
        .ok_or(AuthorError::NotFound)?;
    state
        .audiobook_service
        .update_track(UpdateTrackCommand {
            audiobook_id,
            track_id: chapter.track_id,
            user_id: auth.user_id,
            is_admin: false,
            title: payload.title,
            number: None,
            duration_seconds: payload.duration_seconds,
        })
        .await
        .map_err(map_audiobook_error)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_chapter(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthorKeyAuth>,
    Path((audiobook_id, number)): Path<(i64, i64)>,
) -> Result<StatusCode, AuthorError> {
    assert_key_scope(&auth, audiobook_id)?;
    let chapter = load_chapter(pool(&state), audiobook_id, number)
        .await?
        .ok_or(AuthorError::NotFound)?;
    state
        .audiobook_service
        .remove_track(
            RemoveTrackCommand {
                audiobook_id,
                track_id: chapter.track_id,
                user_id: auth.user_id,
                is_admin: false,
            },
            &state.media_config,
        )
        .await
        .map_err(map_audiobook_error)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn patch_book(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthorKeyAuth>,
    Path(audiobook_id): Path<i64>,
    Json(payload): Json<UpdateBookPayload>,
) -> Result<StatusCode, AuthorError> {
    assert_key_scope(&auth, audiobook_id)?;
    state
        .audiobook_service
        .update_audiobook(UpdateAudiobookCommand {
            audiobook_id,
            user_id: auth.user_id,
            is_admin: false,
            title: payload.title,
            slug: payload.slug,
            description: payload.description,
            translator: payload.translator,
            tags: payload.tags,
        })
        .await
        .map_err(map_audiobook_error)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn change_status(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthorKeyAuth>,
    Path(audiobook_id): Path<i64>,
    Json(payload): Json<ChangeStatusPayload>,
) -> Result<StatusCode, AuthorError> {
    assert_key_scope(&auth, audiobook_id)?;
    state
        .audiobook_service
        .change_audiobook_status(ChangeAudiobookStatusCommand {
            audiobook_id,
            user_id: auth.user_id,
            is_admin: false,
            status: payload.status,
        })
        .await
        .map_err(map_audiobook_error)?;
    Ok(StatusCode::NO_CONTENT)
}
