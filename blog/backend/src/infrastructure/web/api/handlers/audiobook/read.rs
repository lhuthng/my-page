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
    domain::{
        entities::{audiobook::AudiobookTag, secret::Claims},
        errors::audiobook::AudiobookError,
    },
    infrastructure::web::{
        api::handlers::audiobook::shared::{caller_id, is_admin, page_window},
        api::handlers::audiobook::dto::{ListQuery, SlugQuery},
        api::handlers::audiobook::response::{
            AudiobookListResponse, AudiobookSummaryResponse, AudiobookTagsResponse,
            SlugAvailabilityResponse,
        },
        server::AppState,
    },
};

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
