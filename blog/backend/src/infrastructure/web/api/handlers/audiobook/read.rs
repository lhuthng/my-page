// Audiobook read endpoints: admin catalogue, tags, and the public feed.
use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};

use crate::{
    application::{
        commands::audiobook::{
            CheckAudiobookSlugCommand, GetAudiobookCommand, GetAudiobooksCommand,
            GetPublicAudiobookCommand, GetPublicAudiobooksCommand, ListAudiobookTagsCommand,
        },
        services::audiobook::AudiobookService,
    },
    domain::{entities::secret::Claims, errors::audiobook::AudiobookError},
    helper::string::{clamp_offset, clamp_page_size},
    infrastructure::web::{
        api::handlers::audiobook::dto::{ListQuery, SlugQuery, TrackWindowQuery},
        api::handlers::audiobook::response::{
            AudiobookDetailsResponse, AudiobookListResponse, AudiobookTagsResponse,
            SlugAvailabilityResponse,
        },
        api::handlers::audiobook::shared::{caller_id, is_admin, page_window},
        server::AppState,
    },
};

/// Chapters in a window when the caller asks for one but does not size it.
///
/// Sized for a screen or two of chapter list: big enough that a reader scrolling
/// through a normal book rarely waits, small enough that the request stays
/// cheap next to the cover and the book's own metadata.
const TRACK_WINDOW_DEFAULT: i64 = 20;
/// Ceiling on a requested window, so no single request can pull a whole long
/// book down at once.
const TRACK_WINDOW_MAX: i64 = 200;

pub async fn get_audiobooks(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<ListQuery>,
) -> Result<impl IntoResponse, AudiobookError> {
    let (limit, offset) = page_window(query.limit, query.offset, 50);

    let page = state
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
        audiobooks: page.audiobooks.into_iter().map(Into::into).collect(),
        has_more: page.has_more,
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

    let page = state
        .audiobook_service
        .get_public_audiobooks(GetPublicAudiobooksCommand {
            term: query.term,
            tag: query.tag,
            limit,
            offset,
        })
        .await?;

    Ok(Json(AudiobookListResponse {
        audiobooks: page.audiobooks.into_iter().map(Into::into).collect(),
        has_more: page.has_more,
    }))
}

/// The public detail feed, optionally windowed by chapter.
///
/// The players send `tracks_offset`/`tracks_limit` and render skeleton rows for
/// the chapters they have not asked for yet. `has_more_tracks` and
/// `track_count` come back on every answer, so a caller that asked for no
/// window can still tell a truncated list from a short one.
pub async fn get_public_audiobook(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    Query(query): Query<TrackWindowQuery>,
) -> Result<impl IntoResponse, AudiobookError> {
    // A limit is what makes this a windowed read. Without one the answer is the
    // whole chapter list — an offset alone still moves the window's start.
    let tracks_limit = query.tracks_limit.map(|limit| {
        clamp_page_size(Some(limit), TRACK_WINDOW_DEFAULT, TRACK_WINDOW_MAX)
    });

    let audiobook = state
        .audiobook_service
        .get_public_audiobook(GetPublicAudiobookCommand {
            slug,
            tracks_offset: clamp_offset(query.tracks_offset),
            tracks_limit,
        })
        .await?;

    Ok(Json(AudiobookDetailsResponse { audiobook }))
}
