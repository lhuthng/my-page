// Media serving: short-link resolution and streaming with HTTP Range
// support, including the thumbnail fallback, the audio-bucket branch, and the
// path re-rooting rules.
use std::sync::Arc;

use axum::{
    Json,
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
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
    domain::{entities::media::LinkResult, errors::media::MediaError},
    infrastructure::{
        storage::{MEDIA_CACHE_CONTROL, MediaKey, StorageError, audio_object_key, media_key},
        web::{
            api::handlers::media::dto::{GetLinkResponse, MediaQuery, SearchResponse},
            server::{AppState, AudioReadMode, AudioStore},
        },
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
                    created_at: r.created_at,
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
        .get_link(GetLinkCommand {
            short_name: short_name.clone(),
        })
        .await?;

    Ok(Json(GetLinkResponse {
        short_name: link.short_name,
        // The stored `url` column is a disk path, which stops being the
        // address of the bytes as soon as they live in the bucket. The
        // short-name route resolves through whichever store actually holds
        // them, so it is the only URL worth handing out.
        url: format!("media/i/{short_name}"),
        file_type: link.file_type,
        created_at: link.created_at,
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

    // Bytes first, then the fallback link — the same order as before, except
    // that the audio bucket now gets a chance before the disk does.
    match serve_media(&state, &link, &headers).await {
        Ok(response) => Ok(response),
        Err(error) => {
            if !used_fallback && let Some(fallback_short_name) = thumbnail_fallback {
                let fallback_link = state
                    .media_service
                    .get_link(GetLinkCommand {
                        short_name: fallback_short_name,
                    })
                    .await?;
                serve_media(&state, &fallback_link, &headers).await
            } else {
                Err(error)
            }
        }
    }
}

/// Serves one media row.
///
/// The audio bucket answers first when it is configured and actually holds the
/// object; otherwise the disk copy does, which is what every deployment did
/// before the bucket existed and is still the only store for a row that has not
/// been backfilled yet. That ordering is what makes the migration per-file
/// rather than all-or-nothing.
///
/// Nothing but audio ever reaches the bucket. `audio_object_key` returns `None`
/// for every other type, so an image, a cover, an avatar, a video or a model
/// falls straight through to the disk — the scope is structural rather than a
/// check this function has to remember to make.
async fn serve_media(
    state: &AppState,
    link: &LinkResult,
    headers: &HeaderMap,
) -> Result<Response<Body>, MediaError> {
    let Some(key) = media_key(&link.hash, &link.file_type, link.uploader_id) else {
        return Err(MediaError::FileNotFound);
    };

    if let Some(bucket) = &state.media_config.audio_bucket
        && let Some(object) = audio_object_key(&key.key, &link.file_type)
    {
        // The disk path is `MEDIA_PATH.join(key)` and the object is that same
        // key with a prefix, so the two stores hold the same bytes under the
        // same name and the backfill is a plain copy.
        if let Some(size) = bucket
            .client
            .object_size(&object)
            .await
            .map_err(internal_error)?
        {
            return bucket_media_response(
                bucket,
                &object,
                &key.sha256,
                size,
                &link.file_type,
                headers,
            )
            .await;
        }
    }

    let file = open_media_file(&key, link, &state.media_config.dir).await?;
    let size = file
        .metadata()
        .await
        .map_err(|e| MediaError::InternalError(e.to_string()))?
        .len();

    // Use the plain SHA-256 as the ETag - stable content identifier
    // regardless of which store answered the request.
    let etag = format!("\"{}\"", key.sha256);

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
            .header(header::CONTENT_TYPE, link.file_type.clone())
            .header(header::CACHE_CONTROL, MEDIA_CACHE_CONTROL)
            .header(header::ETAG, etag)
            .header(header::ACCEPT_RANGES, "bytes")
            .header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{size}"))
            .header(header::CONTENT_LENGTH, length)
            .body(body)
            .map_err(|e| MediaError::InternalError(e.to_string()));
    }

    Response::builder()
        .status(200)
        .header(header::CONTENT_TYPE, link.file_type.clone())
        .header(header::CACHE_CONTROL, MEDIA_CACHE_CONTROL)
        .header(header::ETAG, etag)
        .header(header::ACCEPT_RANGES, "bytes")
        .body(Body::from_stream(ReaderStream::new(file)))
        .map_err(|e| MediaError::InternalError(e.to_string()))
}

/// Answers a request whose bytes are in the audio bucket, either by pointing
/// the client at the object's public URL or by streaming it through here.
///
/// Nothing is signed: the bucket has a public domain, so the URL is the same
/// for every client and every request.
async fn bucket_media_response(
    bucket: &AudioStore,
    object: &str,
    sha256: &str,
    size: u64,
    file_type: &str,
    headers: &HeaderMap,
) -> Result<Response<Body>, MediaError> {
    match bucket.read_mode {
        AudioReadMode::Redirect => {
            let location = bucket.public_url(object);

            Response::builder()
                .status(StatusCode::FOUND)
                .header(header::LOCATION, location)
                // Deliberately not cacheable, though the target is immutable.
                // The object's identity is fixed, but the URL it is served from
                // is configuration (`R2_PUBLIC_URL`), so a cached redirect
                // would pin a client to a domain that a later deploy may have
                // moved off — for as long as the cache lives. The object
                // carries the immutable Cache-Control instead, and this costs
                // one round trip per cache miss: a player follows the redirect
                // once and then ranges against the public URL directly.
                .header(header::CACHE_CONTROL, "no-store")
                .header(header::CONTENT_TYPE, file_type)
                .body(Body::empty())
                .map_err(|e| MediaError::InternalError(e.to_string()))
        }
        AudioReadMode::Proxy => {
            let etag = format!("\"{sha256}\"");
            let range = headers
                .get(header::RANGE)
                .and_then(|value| value.to_str().ok());
            let if_range_matches = headers
                .get(header::IF_RANGE)
                .and_then(|value| value.to_str().ok())
                .map(|value| value == etag)
                .unwrap_or(true);

            if if_range_matches
                && let Some((start, end)) = range.and_then(|r| parse_byte_range(r, size))
            {
                let bytes = bucket
                    .client
                    .get_object_range(object, start, end)
                    .await
                    .map_err(internal_error)?;
                let length = bytes.len() as u64;

                return Response::builder()
                    .status(206)
                    .header(header::CONTENT_TYPE, file_type)
                    .header(header::CACHE_CONTROL, MEDIA_CACHE_CONTROL)
                    .header(header::ETAG, etag)
                    .header(header::ACCEPT_RANGES, "bytes")
                    .header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{size}"))
                    .header(header::CONTENT_LENGTH, length)
                    .body(Body::from(bytes))
                    .map_err(|e| MediaError::InternalError(e.to_string()));
            }

            let reader = bucket
                .client
                .get_object_reader(object)
                .await
                .map_err(internal_error)?;

            Response::builder()
                .status(200)
                .header(header::CONTENT_TYPE, file_type)
                .header(header::CACHE_CONTROL, MEDIA_CACHE_CONTROL)
                .header(header::ETAG, etag)
                .header(header::ACCEPT_RANGES, "bytes")
                .body(Body::from_stream(ReaderStream::new(reader)))
                .map_err(|e| MediaError::InternalError(e.to_string()))
        }
    }
}

fn internal_error(error: StorageError) -> MediaError {
    MediaError::InternalError(error.to_string())
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

/// Opens the disk copy of a row. The hash-derived path is the primary one; a
/// row whose file is not there (a cover written under a mismatched type
/// prefix, for instance) falls back to re-rooting the stored `url` under the
/// current media directory.
async fn open_media_file(
    key: &MediaKey,
    link: &LinkResult,
    media_dir: &std::path::Path,
) -> Result<fs::File, MediaError> {
    // Open the file for streaming - avoids loading the entire file into RAM,
    // which previously caused OOM kills on the 256 MB machine when many
    // images were requested concurrently.
    let file_path = media_dir.join(&key.key);
    match fs::File::open(&file_path).await {
        Ok(file) => Ok(file),
        Err(_) => {
            let fallback = reroot_path(&link.url, media_dir).ok_or(MediaError::FileNotFound)?;
            fs::File::open(&fallback)
                .await
                .map_err(|_| MediaError::FileNotFound)
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::storage::{S3Settings, r2::R2Client};

    const SHA: &str = "b4c0ffee1234567890abcdef1234567890abcdef1234567890abcdef12345678";
    const PUBLIC_BASE: &str = "https://disk.huuthangle.site";

    fn audio_store(read_mode: AudioReadMode) -> AudioStore {
        AudioStore {
            client: R2Client::from_settings(S3Settings {
                endpoint: "https://account123.r2.cloudflarestorage.com".to_string(),
                region: "auto".to_string(),
                bucket: "retro-games".to_string(),
                access_key: "access".to_string(),
                secret_key: "secret".to_string(),
            }),
            public_base_url: PUBLIC_BASE.to_string(),
            read_mode,
        }
    }

    /// Runs one redirect for a content type and hands back its status, target
    /// and cache policy. `None` when the type has no object key at all, which
    /// is the answer for everything that is not audio.
    async fn redirect_for(content_type: &str) -> Option<(StatusCode, String, String)> {
        let store = audio_store(AudioReadMode::Redirect);
        let key = media_key(SHA, content_type, 1).expect("key");
        let object = audio_object_key(&key.key, content_type)?;

        let response = bucket_media_response(
            &store,
            &object,
            &key.sha256,
            1234,
            content_type,
            &HeaderMap::new(),
        )
        .await
        .expect("response");

        let header_value = |name: &header::HeaderName| {
            response
                .headers()
                .get(name)
                .unwrap_or_else(|| panic!("missing {name}"))
                .to_str()
                .expect("ascii")
                .to_string()
        };

        Some((
            response.status(),
            header_value(&header::LOCATION),
            header_value(&header::CACHE_CONTROL),
        ))
    }

    #[tokio::test]
    async fn audio_redirects_into_its_own_prefix() {
        let (status, location, cache) = redirect_for("audio/mpeg").await.expect("audio redirects");

        assert_eq!(status, StatusCode::FOUND);
        assert_eq!(
            location,
            format!("{PUBLIC_BASE}/audio/b4/c0/{SHA}.mp3"),
            "audio must be under audio/ so it can carry its own lifecycle rule"
        );
        // Not cacheable: the target is immutable but the domain is
        // configuration, and a cached redirect would outlive a move of it.
        assert_eq!(cache, "no-store");
    }

    #[tokio::test]
    async fn nothing_but_audio_is_ever_asked_of_the_bucket() {
        // The scope of the migration, asserted at the read path: these types
        // have no object key, so `serve_media` cannot reach the bucket for
        // them and they are served from the disk exactly as before.
        for content_type in [
            "image/png",
            "image/webp",
            "image/gif",
            "image/jpeg",
            "video/mp4",
            "video/webm",
            "model/gltf-binary",
            "application/vnd.lottie+zip",
        ] {
            assert_eq!(
                redirect_for(content_type).await,
                None,
                "{content_type} must never resolve to a bucket object"
            );
        }
    }

    #[tokio::test]
    async fn the_redirect_is_unsigned_and_stable() {
        // Nothing is signed and nothing expires, so the same request always
        // produces the same target — unlike a presigned URL, which changes on
        // every call. That is what lets the object be cached for a year.
        let (_, first, _) = redirect_for("audio/mpeg").await.expect("audio redirects");
        let (_, second, _) = redirect_for("audio/mpeg").await.expect("audio redirects");

        assert_eq!(first, second);
        assert!(
            !first.contains("X-Amz-Signature") && !first.contains("X-Amz-Expires"),
            "the bucket is public, so nothing should be signed: {first}"
        );
    }

    #[tokio::test]
    async fn proxy_mode_keeps_the_sha256_as_the_etag() {
        // The disk and bucket branches must agree on the ETag, or a client
        // switching between them would see a spurious change.
        let key = media_key(SHA, "audio/mpeg", 1).expect("key");
        assert_eq!(format!("\"{}\"", key.sha256), format!("\"{SHA}\""));
    }
}
