// Media serving: short-link resolution and streaming with HTTP Range
// support, including the thumbnail fallback and path re-rooting rules.
use std::{path::PathBuf, str::FromStr, sync::Arc};

use axum::{
    Json,
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, header},
    response::IntoResponse,
};
use http::Response;
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncSeekExt},
};
use tokio_util::io::ReaderStream;

use crate::{
    application::{
        commands::media::{GetLinkCommand, SearchMediaCommand},
        services::media::MediaService,
    },
    domain::{
        entities::media::{LinkResult, MediaType},
        errors::media::MediaError,
    },
    infrastructure::web::{
        api::handlers::media::dto::{GetLinkResponse, MediaQuery, SearchResponse},
        server::AppState,
    },
};

#[axum::debug_handler]
pub async fn search(
    State(state): State<Arc<AppState>>,
    Query(query): Query<MediaQuery>,
) -> Result<impl IntoResponse, MediaError> {
    let cmd = SearchMediaCommand {
        term: query.term,
        // A missing size used to become LIMIT 0, i.e. always-empty results;
        // clamp instead so the default page applies and callers cannot ask
        // for the whole table.
        size: crate::helper::string::clamp_page_size(query.size.map(i64::from), 24, 100) as u32,
        skip: crate::helper::string::clamp_offset(query.skip.map(i64::from)) as u32,
    };
    match state.media_service.search(cmd).await {
        Ok(link_results) => Ok(Json(SearchResponse {
            results: link_results
                .into_iter()
                .map(|r| GetLinkResponse {
                    short_name: r.short_name,
                    url: r.url,
                    file_type: r.file_type,
                })
                .collect(),
        })),
        Err(e) => Err(e),
    }
}

#[axum::debug_handler]
pub async fn get_link(
    State(state): State<Arc<AppState>>,
    Path(short_name): Path<String>,
) -> Result<impl IntoResponse, MediaError> {
    let link = state
        .media_service
        .get_link(GetLinkCommand { short_name })
        .await?;

    Ok(Json(GetLinkResponse {
        short_name: link.short_name,
        url: link.url,
        file_type: link.file_type,
    }))
}

#[axum::debug_handler]
pub async fn get_media(
    State(state): State<Arc<AppState>>,
    Path(short_name): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, MediaError> {
    let thumbnail_fallback = post_thumbnail_fallback_short_name(&short_name);
    let mut used_fallback = false;

    let link = match state
        .media_service
        .get_link(GetLinkCommand {
            short_name: short_name.clone(),
        })
        .await
    {
        Ok(link) => link,
        Err(e) => {
            if let Some(fallback_short_name) = thumbnail_fallback.as_ref() {
                used_fallback = true;
                state
                    .media_service
                    .get_link(GetLinkCommand {
                        short_name: fallback_short_name.clone(),
                    })
                    .await?
            } else {
                return Err(e);
            }
        }
    };

    let opened = open_media_link(&link, &state.media_config.dir).await;
    let (file, content_hash, file_type) = match opened {
        Ok(result) => result,
        Err(e) => {
            if !used_fallback && let Some(fallback_short_name) = thumbnail_fallback {
                let fallback_link = state
                    .media_service
                    .get_link(GetLinkCommand {
                        short_name: fallback_short_name,
                    })
                    .await?;
                open_media_link(&fallback_link, &state.media_config.dir).await?
            } else {
                return Err(e);
            }
        }
    };

    let size = file
        .metadata()
        .await
        .map_err(|e| MediaError::InternalError(e.to_string()))?
        .len();

    // Use the plain SHA-256 as the ETag - stable content identifier
    // regardless of the storage layout.
    let etag = format!("\"{}\"", content_hash);

    // Single byte-range support so <video>/<audio> can seek without
    // re-downloading the whole file. A partial answer is only safe when
    // If-Range (when present) still matches the ETag; media is
    // content-addressed and immutable, so that is the normal case.
    let range = headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok());
    let if_range_matches = headers
        .get(header::IF_RANGE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value == etag)
        .unwrap_or(true);

    if if_range_matches && let Some((start, end)) = range.and_then(|r| parse_byte_range(r, size)) {
        let length = end - start + 1;
        let mut file = file;
        file.seek(std::io::SeekFrom::Start(start))
            .await
            .map_err(|e| MediaError::InternalError(e.to_string()))?;
        let body = Body::from_stream(ReaderStream::new(file.take(length)));

        return Response::builder()
            .status(206)
            .header(header::CONTENT_TYPE, file_type)
            .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
            .header(header::ETAG, etag)
            .header(header::ACCEPT_RANGES, "bytes")
            .header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{size}"))
            .header(header::CONTENT_LENGTH, length)
            .body(body)
            .map_err(|e| MediaError::InternalError(e.to_string()));
    }

    Response::builder()
        .status(200)
        .header(header::CONTENT_TYPE, file_type)
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        .header(header::ETAG, etag)
        .header(header::ACCEPT_RANGES, "bytes")
        .body(Body::from_stream(ReaderStream::new(file)))
        .map_err(|e| MediaError::InternalError(e.to_string()))
}

/// Parses a single-range `bytes=start-end` header into an inclusive
/// (start, end) pair clamped to the file size. Multi-range, malformed, and
/// unsatisfiable headers return None so the caller answers with the full
/// 200 body, which the RFC permits in place of 416.
fn parse_byte_range(range: &str, size: u64) -> Option<(u64, u64)> {
    let rest = range.strip_prefix("bytes=")?;
    if rest.contains(',') {
        return None;
    }
    let (start_str, end_str) = rest.split_once('-')?;
    if start_str.is_empty() {
        // Suffix form: last N bytes.
        let last = end_str.parse::<u64>().ok()?;
        if last == 0 || size == 0 {
            return None;
        }
        let start = size.saturating_sub(last);
        return Some((start, size - 1));
    }
    let start = start_str.parse::<u64>().ok()?;
    if start >= size {
        return None;
    }
    let end = if end_str.is_empty() {
        size - 1
    } else {
        end_str.parse::<u64>().ok()?.min(size - 1)
    };
    if end < start {
        return None;
    }
    Some((start, end))
}

fn post_thumbnail_fallback_short_name(short_name: &str) -> Option<String> {
    let post_id = short_name
        .strip_prefix(".post.")?
        .strip_suffix(".thumbnail")?;

    if post_id.is_empty() || !post_id.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    Some(format!(".post.{}", post_id))
}

fn media_path_from_link(
    link: &LinkResult,
    media_dir: &std::path::Path,
) -> Result<(PathBuf, String), MediaError> {
    // Decode the storage layout from the hash field. Three layouts exist:
    //
    //  Regular media  hash = "<sha256>"
    //                 file = <media_dir>/<sha256[0..2]>/<sha256[2..4]>/<sha256><ext>
    //
    //  Post cover     hash = ".post.<post_id>.<sha256>"
    //                 file = <media_dir>/post/<uploader_id>/<sha256><ext>
    //                 NOTE: hash encodes post_id but the dir uses uploader_id!
    //
    //  User avatar    hash = ".avt.<user_id>.<sha256>"
    //                 file = <media_dir>/avt/<uploader_id>/<sha256><ext>
    //
    //  Series cover   hash = ".srs.<user_id>.<sha256>"  (fixed)
    //                 file = <media_dir>/srs/<uploader_id>/<sha256><ext>
    //
    // Hash-based reconstruction is the primary path. If the file is missing,
    // open_media_link() falls back to reroot_path() using the stored `url`.
    let extension = MediaType::from_str(&link.file_type)?.get_extension();
    let (file_path, content_hash) = if link.hash.starts_with('.') {
        // Special layout: ".<type>.<id>.<sha256>"
        // splitn(4, '.') keeps the sha256 tail (which has no dots) in one piece:
        // ["", "<type>", "<id>", "<sha256>"]
        //
        // IMPORTANT: parts[2] is the post_id for ".post.*" entries, NOT the
        // user_id used as the on-disk subdirectory.  Always use uploader_id
        // (fetched from the DB) as the directory name - it is the user_id for
        // post covers, avatars, and series covers.
        let parts: Vec<&str> = link.hash.splitn(4, '.').collect();
        if parts.len() < 4 {
            return Err(MediaError::FileNotFound);
        }
        let type_dir = parts[1]; // "post", "avt", or "srs"
        let sha256 = parts[3]; // plain SHA-256 hex
        let path = media_dir
            .join(type_dir)
            .join(link.uploader_id.to_string()) // user_id used at upload time
            .join(format!("{}{}", sha256, extension));
        (path, sha256.to_string())
    } else {
        // Regular layout: hash is a plain SHA-256 hex string.
        let path = media_dir
            .join(&link.hash[0..2])
            .join(&link.hash[2..4])
            .join(format!("{}{}", link.hash, extension));
        (path, link.hash.clone())
    };

    Ok((file_path, content_hash))
}

async fn open_media_link(
    link: &LinkResult,
    media_dir: &std::path::Path,
) -> Result<(fs::File, String, String), MediaError> {
    let (file_path, content_hash) = media_path_from_link(link, media_dir)?;

    // Open the file for streaming - avoids loading the entire file into RAM,
    // which previously caused OOM kills on the 256 MB machine when many
    // images were requested concurrently.
    //
    // If the hash-derived path doesn't exist (e.g. a series cover whose hash
    // was written with the wrong ".avt." prefix), fall back to reconstructing
    // the path from the stored `url` column via reroot_path().
    let file = match fs::File::open(&file_path).await {
        Ok(f) => f,
        Err(_) => {
            let fallback = reroot_path(&link.url, media_dir).ok_or(MediaError::FileNotFound)?;
            fs::File::open(&fallback)
                .await
                .map_err(|_| MediaError::FileNotFound)?
        }
    };

    Ok((file, content_hash, link.file_type.clone()))
}

/// Re-root a stored file path under a new media directory.
///
/// The `stored_url` was written at upload time and may contain an old or
/// relative media directory prefix (e.g. `"./media/srs/2/abc.webp"` or
/// `"/old/path/to/media/post/11/abc.png"`).  This function finds the first
/// path component whose name matches `media_dir`'s own directory name and
/// returns `media_dir` joined with everything that came after it.
///
/// Returns `None` if the media directory name is not found in `stored_url`.
fn reroot_path(stored_url: &str, media_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    let dir_name = media_dir.file_name()?;
    let p = std::path::Path::new(stored_url);
    let mut after = std::path::PathBuf::new();
    let mut found = false;
    for component in p.components() {
        if found {
            after.push(component);
        } else if component.as_os_str() == dir_name {
            found = true;
        }
    }
    if found {
        Some(media_dir.join(after))
    } else {
        None
    }
}
