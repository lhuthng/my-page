// Chapter play beacon: a public, fire-and-forget counter bump.
//
// Anti-spam layering (the storage half lives in
// `persistence::audiobook::plays`):
//   1. the player only reports after ~10 s of real playback;
//   2. this handler sheds bursts with an in-memory per-listener+track window;
//   3. the database counts at most one play per listener per track per day.
// The listener identity is a salted, truncated SHA-256 of the best-effort
// client address, so the database never stores a raw IP.
//
// Residual risk, stated honestly: a client rotating its source address still
// lands in a new dedup bucket each time. Layer 3 caps what one identity can
// inflate; capping a distributed flood would need heavier machinery than a
// personal blog's play counter justifies.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use sha2::{Digest, Sha256};

use crate::{
    application::{
        commands::audiobook::RecordTrackPlayCommand,
        services::audiobook::AudiobookService,
    },
    domain::errors::audiobook::AudiobookError,
    infrastructure::web::server::AppState,
};

/// Mixed into the listener hash. Rotating this value invalidates every stored
/// dedup key at once — and with it, re-opens one free play per listener.
const LISTENER_SALT: &str = "audiobook-play-listener-v1";

/// Minimum spacing between reports from one listener for one track. Legitimate
/// reports are already spaced out by the player's listening threshold; this
/// only stops a scripted client from hammering the endpoint.
const REPORT_MIN_INTERVAL_MS: i64 = 10_000;

/// Best-effort client identity for dedup: Cloudflare's real-client header when
/// present, else the first forwarded address, else one shared bucket. Direct,
/// un-proxied clients all share the fallback, which the per-day dedup makes
/// harmless for ordinary traffic.
fn listener_identity(headers: &HeaderMap) -> String {
    if let Some(ip) = headers.get("cf-connecting-ip").and_then(|v| v.to_str().ok()) {
        return ip.trim().to_string();
    }
    if let Some(forwarded) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(first) = forwarded.split(',').next() {
            let first = first.trim();
            if !first.is_empty() {
                return first.to_string();
            }
        }
    }
    "unknown".to_string()
}

/// 128 bits of the salted digest: enough to dedup on, without storing
/// addresses or making the stored key directly reversible.
fn listener_hash(identity: &str) -> String {
    let digest = Sha256::digest(format!("{LISTENER_SALT}|{identity}").as_bytes());
    hex::encode(&digest[..16])
}

/// Mirrors the v86 save rate limiter: process-local, best-effort burst
/// shedding rather than an exact quota.
fn report_limited(key: &str) -> bool {
    let now = chrono::Utc::now().timestamp_millis();
    static LAST: OnceLock<Mutex<HashMap<String, i64>>> = OnceLock::new();
    let map = LAST.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = map.lock().unwrap();
    if let Some(&last) = map.get(key)
        && now - last < REPORT_MIN_INTERVAL_MS
    {
        return true;
    }
    map.insert(key.to_string(), now);
    false
}

pub async fn record_track_play(
    State(state): State<Arc<AppState>>,
    Path((audiobook_id, track_id_str)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<StatusCode, AudiobookError> {
    // The book id is route sugar (the player already knows both); the track id
    // alone identifies what was played. Malformed ids are a silent no-op.
    let _ = audiobook_id;
    let Ok(track_id) = track_id_str.parse::<i64>() else {
        return Ok(StatusCode::NO_CONTENT);
    };

    let listener = listener_hash(&listener_identity(&headers));
    if report_limited(&format!("{listener}:{track_id}")) {
        return Ok(StatusCode::NO_CONTENT);
    }
    let day = chrono::Utc::now().date_naive().to_string();

    state
        .audiobook_service
        .record_track_play(RecordTrackPlayCommand {
            track_id,
            listener,
            day,
        })
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
