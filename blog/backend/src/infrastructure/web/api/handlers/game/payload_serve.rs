// Serving an unpacked payload bundle, file by file: /games/s/<slug>/payload/<path>.
// The bundle tree is the URL space, exactly as it is on the game's own page:
// Range reads for the streamed archives, brotli declared where the bundle
// stores pre-compressed members, and the isolation headers a wasmshell's
// SharedArrayBuffer needs on everything under its prefix.
use std::{path::Component, sync::Arc};

use axum::{
    Extension,
    extract::{Path as AxumPath, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::Response,
};
use axum_extra::extract::cookie::CookieJar;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;

use crate::{
    domain::{entities::secret::Claims, errors::game::GameError},
    infrastructure::web::server::AppState,
};

use super::access::{entitled, game_access_by_slug};

/// Headers a wasmshell's page requires on its whole prefix (see the gta-vc
/// hosting README): without them the page is not cross-origin isolated, its
/// SharedArrayBuffer never appears, and the streaming reads fail at boot.
const WASMSHELL_HEADERS: [(&str, &str); 3] = [
    ("Cross-Origin-Opener-Policy", "same-origin"),
    ("Cross-Origin-Embedder-Policy", "require-corp"),
    ("Cross-Origin-Resource-Policy", "cross-origin"),
];

/// Content types a bundle actually carries. Everything else is octet-stream,
/// which is what a game's own data files are anyway.
fn content_type(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    let extension = lower.rsplit('.').next().unwrap_or("");
    match extension {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "wasm" => "application/wasm",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "txt" => "text/plain; charset=utf-8",
        "xml" => "application/xml",
        _ => "application/octet-stream",
    }
}

/// `Range: bytes=start-end`, `bytes=start-`, or the last `bytes=-n`. One range
/// only; anything else is served whole, which is also what the engine's reads
/// ask for - one 1 MiB member or one audio window at a time.
fn range_bounds(range: &str, size: u64) -> Option<(u64, u64)> {
    let spec = range.trim().strip_prefix("bytes=")?;
    let (start_raw, end_raw) = spec.split_once('-')?;
    if start_raw.is_empty() {
        // A suffix range: the final N bytes.
        let length: u64 = end_raw.trim().parse().ok()?;
        if length == 0 || length > size {
            return None;
        }
        return Some((size - length, size - 1));
    }
    let start: u64 = start_raw.trim().parse().ok()?;
    let end: u64 = if end_raw.is_empty() {
        size - 1
    } else {
        end_raw.trim().parse().ok()?
    };
    if start > end || start >= size {
        return None;
    }
    Some((start, end.min(size - 1)))
}

async fn payload_row(state: &AppState, slug: &str) -> Result<Option<(String, String)>, GameError> {
    Ok(sqlx::query_as(
        r#"SELECT b.kind, b.storage_key FROM game_payload_bundles b
           JOIN games g ON g.id = b.game_id
           JOIN posts ON posts.id = g.post_id
           WHERE posts.slug = ? AND posts.status = 'published'
             AND g.launcher_type IN ('shimkit', 'wasmshell')"#,
    )
    .bind(slug)
    .fetch_optional(&state.game_service.pool)
    .await?)
}

pub async fn get_payload_file(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    jar: CookieJar,
    AxumPath((slug, path)): AxumPath<(String, String)>,
    headers: HeaderMap,
) -> Result<Response, GameError> {
    // An empty path is a directory, and a directory is not a file this handler
    // serves; the player boots `<payload>/index.html` by name.
    if path.is_empty() || path.ends_with('/') {
        return Err(GameError::GameNotFound);
    }
    // Invited-only games answer a stranger with the same 404 a draft would,
    // so the files do not even confirm the game exists.
    let access = game_access_by_slug(&state.game_service.pool, &slug).await?;
    let allowed = match &access {
        Some(access) => entitled(&state, access, opt_claims.as_ref(), &jar).await?,
        None => false,
    };
    if !allowed {
        return Err(GameError::GameNotFound);
    }
    let Some((kind, storage_key)) = payload_row(&state, &slug).await? else {
        return Err(GameError::GameNotFound);
    };
    // The storage key and the request path are joined the way the zip wrote
    // them, so a traversal in either is a filesystem escape: reject it before
    // touching the disk.
    if [storage_key.as_str(), path.as_str()].iter().any(|part| {
        std::path::Path::new(part).components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    }) {
        return Err(GameError::InternalError("Invalid payload path".to_string()));
    }
    let full = state.project_demo_config.dir.join(&storage_key).join(&path);
    let file = match tokio::fs::File::open(&full).await {
        Ok(file) => file,
        Err(_) => return Err(GameError::GameNotFound),
    };
    let size = file.metadata().await?.len();
    let mut builder = Response::builder();

    // The storage key ends in the zip's sha256, which is the bundle's
    // identity: the shell reads it here and versions every asset URL with it,
    // so a re-uploaded bundle gets fresh URLs instead of colliding with the
    // browser's cached copy of the previous revision.
    let bundle_sha = storage_key
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_string();
    if let Ok(value) = HeaderValue::from_str(&bundle_sha) {
        builder = builder.header("x-bundle-sha", value);
    }

    // Pre-compressed members: the browser inflates them because the header says
    // brotli, which is the whole trick the bundle's block store relies on.
    let is_block = path.starts_with("blocks/");
    if is_block {
        builder = builder.header(header::CONTENT_ENCODING, "br");
    }

    let mut response = match headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
    {
        Some(range) if !range.is_empty() => match range_bounds(range, size) {
            Some((start, end)) => {
                let length = end - start + 1;
                let mut file = file;
                file.seek(std::io::SeekFrom::Start(start)).await?;
                builder
                    .status(StatusCode::PARTIAL_CONTENT)
                    .header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{size}"))
                    .body(axum::body::Body::from_stream(ReaderStream::new(
                        file.take(length),
                    )))
            }
            None => {
                // A range the file cannot satisfy says so, with the size it has.
                builder
                    .status(StatusCode::RANGE_NOT_SATISFIABLE)
                    .header(header::CONTENT_RANGE, format!("bytes */{size}"))
                    .body(axum::body::Body::empty())
            }
        },
        _ => builder
            .header(header::CONTENT_LENGTH, size)
            .body(axum::body::Body::from_stream(ReaderStream::new(file))),
    }
    .map_err(|e| GameError::InternalError(e.to_string()))?;

    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(content_type(&path)),
    );
    response
        .headers_mut()
        .insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        // Block members are content-defined (an archive's member N is the same
        // bytes in every build of that archive), so they cache forever; the
        // bundle root is re-uploaded in place, so everything else revalidates
        // quickly.
        HeaderValue::from_static(if is_block {
            "public, max-age=31536000, immutable"
        } else {
            "public, max-age=300"
        }),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    if kind == "wasmshell" {
        for (name, value) in WASMSHELL_HEADERS {
            if let Ok(header_value) = HeaderValue::from_str(value) {
                response.headers_mut().insert(
                    axum::http::HeaderName::from_bytes(name.as_bytes())
                        .map_err(|e| GameError::InternalError(e.to_string()))?,
                    header_value,
                );
            }
        }
    }
    Ok(response)
}
