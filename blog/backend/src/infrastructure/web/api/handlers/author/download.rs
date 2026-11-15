//! Single-chapter streaming and capped batch archives.
use std::{collections::BTreeSet, path::PathBuf, str::FromStr, sync::Arc};

use axum::{
    Extension,
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::{HeaderValue, header},
    response::Response,
};
use futures::StreamExt;
use tokio_util::io::ReaderStream;

use crate::{
    domain::errors::author::AuthorError,
    infrastructure::{
        storage::{audio_object_key, media_key},
        web::{
            api::{
                handlers::author::{
                    dto::ArchiveQuery,
                    shared::{
                        ChapterRow, assert_key_scope, load_chapter, load_chapters, pool, safe_stem,
                    },
                },
                middlewares::auth::AuthorKeyAuth,
            },
            server::{AppState, config::author_api::max_archive_bytes},
        },
    },
};

const MAX_ARCHIVE_CHAPTERS: usize = 500;

/// Streams the original uploaded bytes (never the zstd chunk form). The same
/// bucket-then-disk resolution `handlers/sync.rs::get_media_by_hash` uses, so
/// a chapter served to a player and one downloaded by its author cannot drift.
pub async fn get_chapter_audio(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthorKeyAuth>,
    Path((audiobook_id, number)): Path<(i64, i64)>,
) -> Result<Response, AuthorError> {
    assert_key_scope(&auth, audiobook_id)?;
    let chapter = load_chapter(pool(&state), audiobook_id, number)
        .await?
        .ok_or(AuthorError::NotFound)?;
    let (body, size) = open_audio(&state, &chapter).await?;

    let extension = crate::domain::entities::media::MediaType::from_str(&chapter.file_type)
        .map(|media_type| media_type.get_extension())
        .unwrap_or(".mp3");
    let filename = format!(
        "{:03} - {}{extension}",
        chapter.number,
        safe_stem(&chapter.title)
    );

    let mut response = Response::new(body);
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&chapter.file_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    if let Ok(value) = HeaderValue::from_str(&size.to_string()) {
        headers.insert(header::CONTENT_LENGTH, value);
    }
    if let Ok(value) = HeaderValue::from_str(&chapter.hash) {
        headers.insert("x-content-sha256", value);
    }
    if let Ok(value) = HeaderValue::from_str(&format!("attachment; filename=\"{filename}\"")) {
        headers.insert(header::CONTENT_DISPOSITION, value);
    }
    Ok(response)
}

/// Streams the bytes from the bucket when present, otherwise from the disk.
/// The plain disk copy is authoritative and is kept even for chunked rows, so
/// the fallback is always able to answer.
async fn open_audio(
    state: &Arc<AppState>,
    chapter: &ChapterRow,
) -> Result<(Body, u64), AuthorError> {
    let key =
        media_key(&chapter.hash, &chapter.file_type, chapter.uploader_id).ok_or_else(|| {
            AuthorError::InternalError("No storage key derivable for this chapter.".to_string())
        })?;

    if let Some(bucket) = &state.media_config.audio_bucket
        && let Some(object) = audio_object_key(&key.key, &chapter.file_type)
        && let Some(size) = bucket
            .client
            .object_size(&object)
            .await
            .map_err(|error| AuthorError::InternalError(error.to_string()))?
    {
        let reader = bucket
            .client
            .get_object_reader(&object)
            .await
            .map_err(|error| AuthorError::InternalError(error.to_string()))?;
        return Ok((Body::from_stream(ReaderStream::new(reader)), size));
    }

    let file = tokio::fs::File::open(state.media_config.dir.join(&key.key))
        .await
        .map_err(|_| AuthorError::NotFound)?;
    let size = file.metadata().await?.len();
    Ok((Body::from_stream(ReaderStream::new(file)), size))
}

/// `1-50,53,60-62`. `None` means every chapter.
fn parse_numbers(raw: Option<&str>) -> Result<Option<BTreeSet<i64>>, AuthorError> {
    let Some(raw) = raw else { return Ok(None) };
    if raw.trim().is_empty() {
        return Ok(None);
    }
    let mut numbers = BTreeSet::new();
    for part in raw.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((start, end)) = part.split_once('-') {
            let start: i64 = start
                .trim()
                .parse()
                .map_err(|_| AuthorError::Validation(format!("Bad chapter range '{part}'.")))?;
            let end: i64 = end
                .trim()
                .parse()
                .map_err(|_| AuthorError::Validation(format!("Bad chapter range '{part}'.")))?;
            if start < 1 || end < start || (end - start) as usize > MAX_ARCHIVE_CHAPTERS {
                return Err(AuthorError::Validation(format!(
                    "Bad chapter range '{part}'."
                )));
            }
            for number in start..=end {
                numbers.insert(number);
            }
        } else {
            let number: i64 = part
                .parse()
                .map_err(|_| AuthorError::Validation(format!("Bad chapter number '{part}'.")))?;
            if number < 1 {
                return Err(AuthorError::Validation(format!(
                    "Bad chapter number '{part}'."
                )));
            }
            numbers.insert(number);
        }
        if numbers.len() > MAX_ARCHIVE_CHAPTERS {
            return Err(AuthorError::Validation(format!(
                "An archive may hold at most {MAX_ARCHIVE_CHAPTERS} chapters."
            )));
        }
    }
    Ok(Some(numbers))
}

struct ArchiveEntry {
    name: String,
    path: PathBuf,
}

/// Builds a zip in a temp file, then streams it and deletes it whether the
/// client finished or disconnected — `stream_temp_file` in `handlers/sync.rs`.
pub async fn get_archive(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthorKeyAuth>,
    Path(audiobook_id): Path<i64>,
    Query(query): Query<ArchiveQuery>,
) -> Result<Response, AuthorError> {
    assert_key_scope(&auth, audiobook_id)?;
    let selected = parse_numbers(query.numbers.as_deref())?;
    let all = load_chapters(pool(&state), audiobook_id, 0, None).await?;
    let chapters: Vec<ChapterRow> = all
        .into_iter()
        .filter(|chapter| {
            selected
                .as_ref()
                .is_none_or(|numbers| numbers.contains(&chapter.number))
        })
        .collect();
    if chapters.is_empty() {
        return Err(AuthorError::Validation(
            "No chapters matched that selection.".to_string(),
        ));
    }

    let total: i64 = chapters.iter().map(|chapter| chapter.size).sum();
    let cap = max_archive_bytes();
    if total as u64 > cap {
        return Err(AuthorError::ArchiveTooLarge(format!(
            "Those chapters total {total} bytes, above the {cap} byte cap. Narrow the request with ?numbers= or download chapters individually."
        )));
    }

    let mut entries = Vec::with_capacity(chapters.len());
    let mut manifest = Vec::new();
    for chapter in &chapters {
        let key =
            media_key(&chapter.hash, &chapter.file_type, chapter.uploader_id).ok_or_else(|| {
                AuthorError::InternalError("No storage key derivable for this chapter.".to_string())
            })?;
        let path = state.media_config.dir.join(&key.key);
        if !tokio::fs::try_exists(&path).await? {
            return Err(AuthorError::NotFound);
        }
        let extension = crate::domain::entities::media::MediaType::from_str(&chapter.file_type)
            .map(|media_type| media_type.get_extension())
            .unwrap_or(".mp3");
        let name = format!(
            "{:03} - {}{extension}",
            chapter.number,
            safe_stem(&chapter.title)
        );
        manifest.push(serde_json::json!({
            "number": chapter.number,
            "title": chapter.title,
            "sha256": chapter.hash,
            "size": chapter.size,
            "file_type": chapter.file_type,
            "file": name,
        }));
        entries.push(ArchiveEntry {
            name: name.clone(),
            path,
        });
    }

    let manifest_bytes = serde_json::to_vec(&serde_json::json!({ "chapters": manifest }))
        .map_err(|error| AuthorError::InternalError(error.to_string()))?;
    let temp_path = std::env::temp_dir().join(format!(
        "author-archive-{}-{}.zip",
        chrono::Utc::now().timestamp(),
        uuid::Uuid::new_v4()
    ));
    let write_path = temp_path.clone();
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let file = std::fs::File::create(&write_path).map_err(|error| error.to_string())?;
        let mut writer = zip::ZipWriter::new(file);
        // Audio containers are already compressed; re-deflating them only
        // burns CPU for a bigger temp file.
        let options = || {
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored)
        };
        for entry in &entries {
            writer
                .start_file(entry.name.clone(), options())
                .map_err(|error| error.to_string())?;
            let mut source = std::fs::File::open(&entry.path).map_err(|error| error.to_string())?;
            std::io::copy(&mut source, &mut writer).map_err(|error| error.to_string())?;
        }
        writer
            .start_file("manifest.json", options())
            .map_err(|error| error.to_string())?;
        std::io::Write::write_all(&mut writer, &manifest_bytes)
            .map_err(|error| error.to_string())?;
        writer.finish().map_err(|error| error.to_string())?;
        Ok(())
    })
    .await
    .map_err(|error| AuthorError::InternalError(error.to_string()))?
    .map_err(AuthorError::InternalError)?;

    let file = tokio::fs::File::open(&temp_path).await?;
    let size = file.metadata().await?.len();
    let cleanup_path = temp_path.clone();
    let cleanup = futures::stream::once(async move {
        let _ = tokio::fs::remove_file(&cleanup_path).await;
        Ok::<Bytes, std::io::Error>(Bytes::new())
    });
    let stream = ReaderStream::with_capacity(file, 64 * 1024).chain(cleanup);

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "application/zip")
        .header(header::CONTENT_LENGTH, size)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"audiobook-{audiobook_id}-chapters.zip\""),
        )
        .body(Body::from_stream(stream))
        .unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_specs_parse_ranges_and_singles() {
        let parsed = parse_numbers(Some("1-3,7, 10-11")).unwrap().unwrap();
        assert_eq!(
            parsed.into_iter().collect::<Vec<_>>(),
            vec![1, 2, 3, 7, 10, 11]
        );
    }

    #[test]
    fn an_empty_spec_means_the_whole_book() {
        assert!(parse_numbers(None).unwrap().is_none());
        assert!(parse_numbers(Some("  ")).unwrap().is_none());
    }

    #[test]
    fn junk_specs_are_rejected() {
        for spec in ["0", "5-2", "abc", "1-99999"] {
            assert!(parse_numbers(Some(spec)).is_err(), "{spec}");
        }
    }
}
