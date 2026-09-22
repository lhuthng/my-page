// Global HTTP layer policy: CORS origins, response compression predicate,
// and request tracing. Wired once in `router.rs`.
use axum::http::header::CONTENT_TYPE;
use http::{HeaderValue, Method};
use tower_http::classify::{ServerErrorsAsFailures, SharedClassifier};
use tower_http::compression::CompressionLayer;
use tower_http::compression::predicate::{DefaultPredicate, NotForContentType, Predicate};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;

/// CORS for every browser-reachable route, not just mail: with
/// BACKEND_ORIGIN set, browsers fetch media and (in fs mode) v86 artifacts
/// cross-origin via XHR, which needs ACAO headers here. Authed client calls
/// stay on the same-origin SvelteKit proxy, so credentials are never involved
/// and the default (no allow_credentials) is correct.
pub fn cors() -> CorsLayer {
    let cors_origins = std::env::var("ALLOWED_ORIGINS")
        .or_else(|_| std::env::var("ALLOWED_ORIGIN"))
        .unwrap_or_else(|_| {
            if cfg!(debug_assertions) {
                "http://localhost:3004,http://localhost:5000,http://localhost:3000,https://portfolio.huuthangle.site".to_string()
            } else {
                "https://portfolio.huuthangle.site".to_string()
            }
        });
    let cors_origins = cors_origins
        .split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .map(|origin| origin.parse::<HeaderValue>().unwrap())
        .collect::<Vec<_>>();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(cors_origins))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([CONTENT_TYPE])
}

/// Saves, ISO/disk chunks, and js-dos bundles are already compressed and
/// media files are binary; recompressing them per download burns CPU for no
/// size win.
pub fn compression() -> CompressionLayer<impl tower_http::compression::Predicate> {
    CompressionLayer::new().compress_when(
        DefaultPredicate::new()
            .and(NotForContentType::const_new("application/octet-stream"))
            .and(NotForContentType::const_new("video/"))
            .and(NotForContentType::const_new("audio/")),
    )
}

pub fn trace() -> TraceLayer<SharedClassifier<ServerErrorsAsFailures>> {
    TraceLayer::new_for_http()
}
