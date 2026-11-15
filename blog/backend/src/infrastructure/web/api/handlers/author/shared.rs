//! Shared plumbing for the author API: scope checks, chapter loading, hashing.
use std::sync::Arc;

use axum::body::Bytes;
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool, sqlite::SqliteRow};

use crate::domain::{
    entities::secret::Claims,
    errors::{audiobook::AudiobookError, author::AuthorError},
};
use crate::infrastructure::web::{api::middlewares::auth::AuthorKeyAuth, server::AppState};

/// Parse the JWT subject for the session-guarded key-management routes. The
/// audiobook module's `shared` is private to its own feature, so this lives
/// here rather than reaching across module boundaries.
pub(crate) fn caller_id(claims: &Claims) -> Result<i64, AuthorError> {
    claims
        .user_id
        .parse::<i64>()
        .map_err(|_| AuthorError::InternalError("Cannot parse user id.".to_string()))
}

/// Every key-tier handler calls this first. A key unlocks exactly one book, and
/// a mismatch is a 404 — a 403 would confirm that the other book exists.
pub(crate) fn assert_key_scope(auth: &AuthorKeyAuth, path_id: i64) -> Result<(), AuthorError> {
    if auth.audiobook_id == path_id {
        Ok(())
    } else {
        Err(AuthorError::NotFound)
    }
}

/// Key issuance is session-guarded; this is the ownership check the guard
/// performs for the key tier instead.
pub(crate) async fn assert_book_owner(
    pool: &SqlitePool,
    audiobook_id: i64,
    user_id: i64,
) -> Result<(), AuthorError> {
    let owner: Option<i64> = sqlx::query_scalar("SELECT user_id FROM audiobooks WHERE id = ?")
        .bind(audiobook_id)
        .fetch_optional(pool)
        .await?;
    match owner {
        None => Err(AuthorError::NotFound),
        Some(owner) if owner == user_id => Ok(()),
        Some(_) => Err(AuthorError::Forbidden(
            "Only the creator of this audiobook can manage its keys.".to_string(),
        )),
    }
}

/// A chapter joined to the media row that owns its bytes.
#[derive(Clone)]
pub(crate) struct ChapterRow {
    pub track_id: i64,
    pub number: i64,
    pub title: String,
    pub duration_seconds: Option<i64>,
    pub play_count: i64,
    pub updated_at: Option<String>,
    pub hash: String,
    pub file_type: String,
    pub size: i64,
    pub uploader_id: i64,
}

pub(crate) const CHAPTER_SELECT: &str = r#"
    SELECT t.id AS track_id, t.number, t.title, t.duration_seconds, t.play_count,
           COALESCE(t.updated_at, t.created_at) AS updated_at,
           m.hash, m.file_type, m.size,
           COALESCE(m.uploader_id, 0) AS uploader_id
    FROM audiobook_tracks t
    JOIN media m ON m.id = t.media_id
    WHERE t.audiobook_id = ?
"#;

pub(crate) fn chapter_from_row(row: &SqliteRow) -> ChapterRow {
    ChapterRow {
        track_id: row.get("track_id"),
        number: row.get("number"),
        title: row.get("title"),
        duration_seconds: row.get("duration_seconds"),
        play_count: row.get("play_count"),
        updated_at: row.get("updated_at"),
        hash: row.get("hash"),
        file_type: row.get("file_type"),
        size: row.get("size"),
        uploader_id: row.get("uploader_id"),
    }
}

/// Loads chapters in play order. `limit = None` means the whole book, which is
/// the default for the manifest: a hash diff against a partial list would
/// silently skip chapters.
pub(crate) async fn load_chapters(
    pool: &SqlitePool,
    audiobook_id: i64,
    offset: i64,
    limit: Option<i64>,
) -> Result<Vec<ChapterRow>, AuthorError> {
    let sql = format!("{CHAPTER_SELECT} ORDER BY t.number ASC LIMIT ? OFFSET ?");
    let rows = sqlx::query(&sql)
        .bind(audiobook_id)
        .bind(limit.unwrap_or(-1))
        .bind(offset)
        .fetch_all(pool)
        .await?;
    Ok(rows.iter().map(chapter_from_row).collect())
}

/// One chapter by its slot. Used by PUT, PATCH, DELETE and the single download.
pub(crate) async fn load_chapter(
    pool: &SqlitePool,
    audiobook_id: i64,
    number: i64,
) -> Result<Option<ChapterRow>, AuthorError> {
    let sql = format!("{CHAPTER_SELECT} AND t.number = ?");
    let row = sqlx::query(&sql)
        .bind(audiobook_id)
        .bind(number)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(chapter_from_row))
}

pub(crate) async fn count_chapters(
    pool: &SqlitePool,
    audiobook_id: i64,
) -> Result<i64, AuthorError> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audiobook_tracks t
         JOIN media m ON m.id = t.media_id WHERE t.audiobook_id = ?",
    )
    .bind(audiobook_id)
    .fetch_one(pool)
    .await?;
    Ok(count)
}

/// SHA-256 on the blocking pool — chapters reach 100 MB and hashing one on an
/// async worker would pin it, exactly as `persistence::media::hash_bytes`
/// documents.
pub(crate) async fn sha256_hex(bytes: &Bytes) -> Result<String, AuthorError> {
    let owned = bytes.clone();
    tokio::task::spawn_blocking(move || hex::encode(Sha256::digest(&owned)))
        .await
        .map_err(|error| AuthorError::InternalError(error.to_string()))
}

/// The write paths already validate titles, slots and audio types; surface
/// their answers without leaking DB details into a scripting API.
pub(crate) fn map_audiobook_error(error: AudiobookError) -> AuthorError {
    match error {
        AudiobookError::NotFound => AuthorError::NotFound,
        AudiobookError::PermissionDenied => {
            AuthorError::Forbidden("This key's owner no longer owns this audiobook.".to_string())
        }
        AudiobookError::Validation(message) => AuthorError::Validation(message),
        AudiobookError::Duplication => {
            AuthorError::Validation("A chapter already occupies that number.".to_string())
        }
        AudiobookError::Media(_) => {
            AuthorError::Validation("Unsupported or invalid audio file.".to_string())
        }
        other => AuthorError::InternalError(format!("{other:?}")),
    }
}

pub(crate) fn pool(state: &Arc<AppState>) -> &SqlitePool {
    &state.audiobook_service.pool
}

/// `Content-Type` without parameters, lower-cased. `MediaType::from_str` is
/// exact-match, so `audio/mpeg; charset=utf-8` has to be trimmed first.
pub(crate) fn normalize_content_type(raw: Option<&str>) -> String {
    raw.unwrap_or("audio/mpeg")
        .split(';')
        .next()
        .unwrap_or("audio/mpeg")
        .trim()
        .to_ascii_lowercase()
}

/// Header-safe, filesystem-safe ASCII name. Chapter titles here are often
/// Vietnamese; keeping only ASCII avoids quoting bugs in `Content-Disposition`
/// and in zip entries.
pub(crate) fn safe_stem(title: &str) -> String {
    let stem: String = title
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, ' ' | '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = stem.trim_matches(|character: char| character == '-' || character == ' ');
    if trimmed.is_empty() {
        "chapter".to_string()
    } else {
        trimmed.chars().take(80).collect()
    }
}
