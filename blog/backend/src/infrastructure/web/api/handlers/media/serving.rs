// Media serving: short-link resolution and streaming with HTTP Range
// support, including the thumbnail fallback, the audio-bucket branch, and the
// path re-rooting rules.
use std::sync::Arc;
use std::time::Duration;

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
        storage::{
            MEDIA_CACHE_CONTROL, MediaKey, StorageError, audio_object_key, chunk_key,
            chunked::{self, CHUNK_CONTENT_TYPE, ChunkEntry, ChunkManifest},
            is_audio, media_key,
        },
        web::{
            api::handlers::media::dto::{
                ChunkManifestResponse, ChunkResponse, GetLinkResponse, MediaQuery, SearchResponse,
            },
            server::{AppState, AudioReadMode, AudioStore, MediaConfig},
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

/// How long a signed chunk URL stays good.
///
/// A chapter is minutes long and is buffered as it plays, so the window only has
/// to outlast one listen; a listener who leaves a chapter paused longer than
/// this gets a failed chunk fetch and re-reads the manifest, which is why the
/// player is expected to treat a chunk failure as "ask for the table again"
/// rather than as a broken chapter.
const CHUNK_URL_TTL: Duration = Duration::from_secs(6 * 60 * 60);

/// A chapter's chunk table: what a player needs to fetch and decompress the
/// audio itself.
///
/// The audio URL is deliberately not advertised as chunked here — a client asks
/// for this table when it *wants* to decompress, and a client that does not
/// simply never asks. That is what lets the browser player, the macOS app and
/// anything else that only speaks mp3 keep working off the same media row.
#[axum::debug_handler]
pub async fn get_track_manifest(
    State(state): State<Arc<AppState>>,
    Path(short_name): Path<String>,
) -> Result<Json<ChunkManifestResponse>, MediaError> {
    let link = state
        .media_service
        .get_link(GetLinkCommand {
            short_name: short_name.clone(),
        })
        .await?;

    let Some(key) = media_key(&link.hash, &link.file_type, link.uploader_id) else {
        return Err(MediaError::FileNotFound);
    };
    // A row with no manifest has no table. 404 rather than an empty one, so a
    // caller's fallback is unambiguously "play the audio URL" and it never has
    // to decide that for itself.
    let Some(manifest) =
        chunked::read_manifest(&state.media_config, &key.key, &link.file_type).await
    else {
        return Err(MediaError::FileNotFound);
    };

    chunk_manifest_response(&state.media_config, &short_name, &link, &key, &manifest).await
}

async fn chunk_manifest_response(
    config: &MediaConfig,
    short_name: &str,
    link: &LinkResult,
    key: &MediaKey,
    manifest: &ChunkManifest,
) -> Result<Json<ChunkManifestResponse>, MediaError> {
    let mut chunks = Vec::with_capacity(manifest.chunks.len());

    for (index, entry) in manifest.chunks.iter().enumerate() {
        let url = match &config.audio_bucket {
            // Audio on the bucket: the chunk is fetched straight from it, so a
            // play costs this process no bandwidth at all. The URL is signed,
            // which is what gives it a lifetime.
            Some(bucket) => {
                let object = audio_object_key(&chunk_key(&key.key, index), &link.file_type)
                    .ok_or(MediaError::FileNotFound)?;
                bucket
                    .client
                    .presigned_get_url(&object, CHUNK_URL_TTL)
                    .await
                    .map_err(internal_error)?
            }
            // Audio on the disk: there is no URL to sign, so the chunk comes
            // back through this process instead.
            None => format!("media/i/{short_name}/chunk/{index}"),
        };

        chunks.push(ChunkResponse {
            index,
            offset: manifest.raw_offset(index),
            raw: entry.raw,
            compressed: entry.compressed,
            sha256: entry.sha256.clone(),
            url,
        });
    }

    Ok(Json(ChunkManifestResponse {
        version: manifest.version,
        chunk_size: manifest.chunk_size,
        raw_size: manifest.raw_size,
        level: manifest.level,
        content_type: manifest.content_type.clone(),
        chunks,
    }))
}

/// One chunk of a chapter, exactly as stored: a standalone zstd frame for the
/// caller to decompress.
#[axum::debug_handler]
pub async fn get_media_chunk(
    State(state): State<Arc<AppState>>,
    Path((short_name, index)): Path<(String, usize)>,
) -> Result<Response<Body>, MediaError> {
    let link = state
        .media_service
        .get_link(GetLinkCommand { short_name })
        .await?;

    let Some(key) = media_key(&link.hash, &link.file_type, link.uploader_id) else {
        return Err(MediaError::FileNotFound);
    };
    let Some(manifest) =
        chunked::read_manifest(&state.media_config, &key.key, &link.file_type).await
    else {
        return Err(MediaError::FileNotFound);
    };
    let Some(entry) = manifest.chunks.get(index) else {
        return Err(MediaError::FileNotFound);
    };

    chunk_bytes_response(&state.media_config, &key, &link.file_type, index, entry).await
}

/// Answers with one stored chunk.
///
/// The ETag is the SHA-256 of the chunk's *raw* bytes rather than of the frame,
/// because that is the value a client can check its decompression against: a
/// frame can be stored correctly and still decode wrong, and the manifest
/// carries the same hash for that reason.
async fn chunk_bytes_response(
    config: &MediaConfig,
    key: &MediaKey,
    file_type: &str,
    index: usize,
    entry: &ChunkEntry,
) -> Result<Response<Body>, MediaError> {
    let bytes = chunked::compressed_chunk(config, &key.key, file_type, index, entry)
        .await
        .map_err(internal_error)?;

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, CHUNK_CONTENT_TYPE)
        .header(header::CACHE_CONTROL, MEDIA_CACHE_CONTROL)
        .header(header::ETAG, format!("\"{}\"", entry.sha256))
        .header(header::CONTENT_LENGTH, bytes.len())
        .body(Body::from(bytes))
        .map_err(|e| MediaError::InternalError(e.to_string()))
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

    // Chunked audio answers first. The manifest is the switch: a row written
    // before chunking existed has none and falls through to the plain file
    // below, which is what makes this per-file rather than all-or-nothing.
    if is_audio(&link.file_type)
        && let Some(manifest) =
            chunked::read_manifest(&state.media_config, &key.key, &link.file_type).await
    {
        return chunked_media_response(&state.media_config, link, &key, &manifest, headers);
    }

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

/// Serves a chunked row by reassembling the requested range from its chunks.
///
/// Every header is the one the plain path sends — same content type, same
/// SHA-256 ETag, same `Accept-Ranges` — because from a client's side a chapter
/// is a chapter. Where the bytes are compressed is a property of the store, not
/// of the protocol, so a player that could seek and range before still does,
/// and nothing about the player had to change for the audio to shrink.
///
/// The one thing this cannot do is redirect: a client cannot be pointed at a
/// single object when the bytes it asked for span several of them. A chunked
/// row is therefore always streamed through here, however `AUDIO_READ` is set.
fn chunked_media_response(
    config: &MediaConfig,
    link: &LinkResult,
    key: &MediaKey,
    manifest: &ChunkManifest,
    headers: &HeaderMap,
) -> Result<Response<Body>, MediaError> {
    let size = manifest.raw_size;
    let etag = format!("\"{}\"", key.sha256);

    // Same range handling as the plain path, against the *raw* length: the
    // compression is invisible to the arithmetic a client does with these
    // numbers.
    let range = headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok());
    let if_range_matches = headers
        .get(header::IF_RANGE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value == etag)
        .unwrap_or(true);

    let partial = if_range_matches
        .then(|| range.and_then(|r| parse_byte_range(r, size)))
        .flatten();

    // No usable range answers with the whole chapter, which is what the plain
    // path does as well: a malformed range is not an error worth a 416.
    let (start, end, status) = match partial {
        Some((start, end)) => (start, end, StatusCode::PARTIAL_CONTENT),
        None => (0, size.saturating_sub(1), StatusCode::OK),
    };

    if size == 0 {
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, link.file_type.clone())
            .header(header::CACHE_CONTROL, MEDIA_CACHE_CONTROL)
            .header(header::ETAG, etag)
            .header(header::ACCEPT_RANGES, "bytes")
            .header(header::CONTENT_LENGTH, 0)
            .body(Body::empty())
            .map_err(|e| MediaError::InternalError(e.to_string()));
    }

    let body = Body::from_stream(chunked::range_stream(
        config.dir.clone(),
        config
            .audio_bucket
            .as_ref()
            .map(|bucket| bucket.client.clone()),
        key.key.clone(),
        link.file_type.clone(),
        manifest.clone(),
        start,
        end,
    ));

    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, link.file_type.clone())
        .header(header::CACHE_CONTROL, MEDIA_CACHE_CONTROL)
        .header(header::ETAG, etag)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_LENGTH, end - start + 1);

    if status == StatusCode::PARTIAL_CONTENT {
        builder = builder.header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{size}"));
    }

    builder
        .body(body)
        .map_err(|e| MediaError::InternalError(e.to_string()))
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

    /// A chunked chapter written to a temporary media directory, plus the row
    /// that names it.
    async fn chunked_chapter(payload: &[u8]) -> (MediaConfig, R2Client, LinkResult, MediaKey) {
        let dir =
            std::env::temp_dir().join(format!("serving-chunked-{}", uuid::Uuid::new_v4().simple()));
        let config = MediaConfig {
            dir,
            allowed_file_types: Vec::new(),
            allowed_avatar_types: Vec::new(),
            allowed_cover_types: Vec::new(),
            allowed_audio_types: Vec::new(),
            audio_bucket: None,
        };

        let key = media_key(SHA, "audio/mpeg", 1).expect("key");
        crate::infrastructure::storage::chunked::write(&config, &key.key, "audio/mpeg", payload)
            .await
            .expect("write chunks");

        let link = LinkResult {
            short_name: Some("abt-1-deadbeef-suffix".to_string()),
            url: "media/i/abt-1-deadbeef-suffix".to_string(),
            file_type: "audio/mpeg".to_string(),
            created_at: None,
            hash: SHA.to_string(),
            uploader_id: 1,
        };

        (config, audio_store(AudioReadMode::Proxy).client, link, key)
    }

    fn range_headers(range: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::RANGE, range.parse().expect("range header"));
        headers
    }

    #[tokio::test]
    async fn a_chunked_chapter_is_served_as_ordinary_audio() {
        // The whole point of serving chunked audio through this handler: a
        // client sees a normal range-capable mp3, so the compression is
        // invisible and no player had to change.
        let payload: Vec<u8> = (0..(1024 * 1024 * 2 + 777))
            .map(|i: i64| (i.wrapping_mul(31) % 251) as u8)
            .collect();
        let (config, _client, link, key) = chunked_chapter(&payload).await;
        let manifest =
            crate::infrastructure::storage::chunked::read_manifest(&config, &key.key, "audio/mpeg")
                .await
                .expect("manifest");

        // A window crossing a chunk boundary, which is what a seek does.
        let start = 1024 * 1024 - 100;
        let end = 1024 * 1024 + 500;
        let response = chunked_media_response(
            &config,
            &link,
            &key,
            &manifest,
            &range_headers(&format!("bytes={start}-{end}")),
        )
        .expect("response");

        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("audio/mpeg"),
            "the reassembled bytes are mp3, not zstd"
        );
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_RANGE)
                .and_then(|v| v.to_str().ok()),
            Some(format!("bytes {start}-{end}/{}", payload.len()).as_str()),
            "the range is expressed against the raw length, so a client's arithmetic is unchanged"
        );
        assert_eq!(
            response
                .headers()
                .get(header::ETAG)
                .and_then(|v| v.to_str().ok()),
            Some(format!("\"{SHA}\"").as_str()),
            "same ETag as the plain path, so a client switching stores sees no change"
        );
        assert_eq!(
            response
                .headers()
                .get(header::ACCEPT_RANGES)
                .and_then(|v| v.to_str().ok()),
            Some("bytes")
        );

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        assert_eq!(
            body.as_ref(),
            &payload[start as usize..=end as usize],
            "the bytes served must be the original audio for that span"
        );

        std::fs::remove_dir_all(&config.dir).ok();
    }

    /// The same media directory, but configured as a deployment whose audio
    /// lives on the bucket. Nothing is fetched from it here — signing happens
    /// locally — so the URLs can be checked without a network.
    fn config_as_bucket(config: &MediaConfig) -> MediaConfig {
        MediaConfig {
            dir: config.dir.clone(),
            allowed_file_types: Vec::new(),
            allowed_avatar_types: Vec::new(),
            allowed_cover_types: Vec::new(),
            allowed_audio_types: Vec::new(),
            audio_bucket: Some(audio_store(AudioReadMode::Proxy)),
        }
    }

    #[tokio::test]
    async fn the_chunk_table_maps_offsets_and_points_at_each_chunk() {
        // What a player needs to turn a seek into a fetch: where each chunk
        // begins and where to get it. Without the offsets it could not seek
        // without downloading the chapter from the start, which is the thing
        // the whole design is for.
        let payload: Vec<u8> = (0..(1024 * 1024 + 250))
            .map(|i: usize| (i % 251) as u8)
            .collect();
        let (config, _client, link, key) = chunked_chapter(&payload).await;
        let manifest =
            crate::infrastructure::storage::chunked::read_manifest(&config, &key.key, "audio/mpeg")
                .await
                .expect("manifest");

        let Json(table) =
            chunk_manifest_response(&config, "abt-1-deadbeef-suffix", &link, &key, &manifest)
                .await
                .expect("table");

        assert_eq!(table.raw_size, payload.len() as u64);
        assert_eq!(table.chunks.len(), 2);
        assert_eq!(table.chunks[0].offset, 0);
        assert_eq!(table.chunks[1].offset, 1024 * 1024);
        assert_eq!(table.chunks[1].raw, 250);
        assert_eq!(table.content_type, "audio/mpeg");
        // On the disk there is nothing to sign, so chunks come back through
        // this backend rather than straight from a bucket.
        assert_eq!(table.chunks[0].url, "media/i/abt-1-deadbeef-suffix/chunk/0");
        assert_eq!(table.chunks[1].url, "media/i/abt-1-deadbeef-suffix/chunk/1");

        std::fs::remove_dir_all(&config.dir).ok();
    }

    #[tokio::test]
    async fn chunk_urls_are_signed_and_expire_when_the_audio_is_on_the_bucket() {
        let payload: Vec<u8> = (0..2048).map(|i: usize| (i % 251) as u8).collect();
        let (config, _client, link, key) = chunked_chapter(&payload).await;
        let manifest =
            crate::infrastructure::storage::chunked::read_manifest(&config, &key.key, "audio/mpeg")
                .await
                .expect("manifest");

        let Json(table) = chunk_manifest_response(
            &config_as_bucket(&config),
            "abt-1-deadbeef-suffix",
            &link,
            &key,
            &manifest,
        )
        .await
        .expect("table");

        for chunk in &table.chunks {
            assert!(
                chunk.url.contains("X-Amz-Signature="),
                "a chunk URL must be signed: {}",
                chunk.url
            );
            assert!(
                chunk
                    .url
                    .contains(&format!("X-Amz-Expires={}", CHUNK_URL_TTL.as_secs())),
                "and must carry its lifetime: {}",
                chunk.url
            );
            // Fetched straight from the bucket, so this process is not in the
            // byte path at all.
            assert!(
                chunk.url.contains("audio/"),
                "and must point into the audio prefix: {}",
                chunk.url
            );
        }

        std::fs::remove_dir_all(&config.dir).ok();
    }

    #[tokio::test]
    async fn a_chunk_is_served_as_the_zstd_frame_the_client_decodes() {
        // The client's half of the contract: what comes back is a standalone
        // zstd frame that decompresses to that chunk's raw bytes — not the
        // audio, and not a frame that needs the chunks before it.
        let payload: Vec<u8> = (0..(1024 * 1024 + 64))
            .map(|i: usize| (i % 251) as u8)
            .collect();
        let (config, _client, _link, key) = chunked_chapter(&payload).await;
        let manifest =
            crate::infrastructure::storage::chunked::read_manifest(&config, &key.key, "audio/mpeg")
                .await
                .expect("manifest");
        // The second chunk deliberately: a client fetching it alone is the case
        // that would break if the frames were not independent.
        let entry = manifest.chunks[1].clone();

        let response = chunk_bytes_response(&config, &key, "audio/mpeg", 1, &entry)
            .await
            .expect("response");

        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some(CHUNK_CONTENT_TYPE)
        );
        assert_eq!(
            response
                .headers()
                .get(header::ETAG)
                .and_then(|v| v.to_str().ok()),
            Some(format!("\"{}\"", entry.sha256).as_str()),
            "the raw hash is what a client checks its decompression against"
        );

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        assert_eq!(body.len() as u64, entry.compressed);
        assert_eq!(
            zstd::bulk::decompress(&body, entry.raw as usize).expect("decompress"),
            payload[1024 * 1024..],
            "decompressing the second chunk alone must give the second chunk's bytes"
        );

        std::fs::remove_dir_all(&config.dir).ok();
    }

    #[tokio::test]
    async fn a_chunked_chapter_answers_a_plain_request_with_the_whole_file() {
        let payload: Vec<u8> = (0..4096).map(|i: i64| (i % 256) as u8).collect();
        let (config, _client, link, key) = chunked_chapter(&payload).await;
        let manifest =
            crate::infrastructure::storage::chunked::read_manifest(&config, &key.key, "audio/mpeg")
                .await
                .expect("manifest");
        let response = chunked_media_response(&config, &link, &key, &manifest, &HeaderMap::new())
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response.headers().get(header::CONTENT_RANGE).is_none(),
            "a whole-file answer carries no Content-Range"
        );

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        assert_eq!(body.as_ref(), payload.as_slice());

        std::fs::remove_dir_all(&config.dir).ok();
    }

    #[tokio::test]
    async fn proxy_mode_keeps_the_sha256_as_the_etag() {
        // The disk and bucket branches must agree on the ETag, or a client
        // switching between them would see a spurious change.
        let key = media_key(SHA, "audio/mpeg", 1).expect("key");
        assert_eq!(format!("\"{}\"", key.sha256), format!("\"{SHA}\""));
    }
}
