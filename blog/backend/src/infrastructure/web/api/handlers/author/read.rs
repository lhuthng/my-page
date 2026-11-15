//! The manifest: book metadata plus every chapter's hash and download path.
//!
//! This is the *plan* for a sync. A client reads it, compares the hashes with
//! its local files, and only PUTs chapters that differ — which is why an
//! omitted `limit` means the whole book, not a comfortable default window the
//! way the public player does.
use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use sqlx::Row;

use crate::{
    domain::errors::author::AuthorError,
    helper::string::{clamp_offset, clamp_page_size},
    infrastructure::web::{
        api::{
            handlers::author::{
                dto::ManifestQuery,
                response::{AuthorBookResponse, AuthorChapterResponse, AuthorManifestResponse},
                shared::{assert_key_scope, count_chapters, load_chapters, pool},
            },
            middlewares::auth::AuthorKeyAuth,
        },
        server::AppState,
    },
};

const MAX_WINDOW: i64 = 2000;

pub async fn get_manifest(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthorKeyAuth>,
    Path(audiobook_id): Path<i64>,
    Query(query): Query<ManifestQuery>,
) -> Result<Json<AuthorManifestResponse>, AuthorError> {
    assert_key_scope(&auth, audiobook_id)?;
    let db = pool(&state);

    let book = sqlx::query(
        r#"
        SELECT id, title, slug, COALESCE(description, '') AS description,
               translator, status, created_at,
               COALESCE(updated_at, created_at) AS updated_at, published_at
        FROM audiobooks WHERE id = ?
        "#,
    )
    .bind(audiobook_id)
    .fetch_optional(db)
    .await?
    .ok_or(AuthorError::NotFound)?;

    let total_duration_seconds: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(duration_seconds), 0) FROM audiobook_tracks WHERE audiobook_id = ?",
    )
    .bind(audiobook_id)
    .fetch_one(db)
    .await?;

    let tags: Vec<String> = sqlx::query_scalar(
        "SELECT t.name FROM audiobook_tags t
         JOIN audiobook_tag_links l ON l.tag_id = t.id
         WHERE l.audiobook_id = ? ORDER BY t.name",
    )
    .bind(audiobook_id)
    .fetch_all(db)
    .await?;

    let offset = clamp_offset(query.offset);
    let limit = query
        .limit
        .map(|value| clamp_page_size(Some(value), MAX_WINDOW, MAX_WINDOW));
    let rows = load_chapters(db, audiobook_id, offset, limit).await?;
    let chapter_count = count_chapters(db, audiobook_id).await?;

    let chapters: Vec<AuthorChapterResponse> = rows
        .into_iter()
        .map(|chapter| AuthorChapterResponse {
            download_url: format!(
                "author/audiobooks/id/{audiobook_id}/chapters/{}/audio",
                chapter.number
            ),
            number: chapter.number,
            track_id: chapter.track_id,
            title: chapter.title,
            duration_seconds: chapter.duration_seconds,
            sha256: chapter.hash,
            size: chapter.size,
            file_type: chapter.file_type,
            play_count: chapter.play_count,
            updated_at: chapter.updated_at,
        })
        .collect();
    let has_more_chapters = offset + (chapters.len() as i64) < chapter_count;

    Ok(Json(AuthorManifestResponse {
        audiobook: AuthorBookResponse {
            id: book.get("id"),
            title: book.get("title"),
            slug: book.get("slug"),
            description: book.get("description"),
            translator: book.get("translator"),
            status: book.get("status"),
            tags,
            total_duration_seconds,
            created_at: book.get("created_at"),
            updated_at: book.get("updated_at"),
            published_at: book.get("published_at"),
        },
        chapters,
        chapter_count,
        has_more_chapters,
    }))
}
