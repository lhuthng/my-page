// Chapter play beacon: a public, fire-and-forget counter bump.
//
// The counter measures listening time, so the player reports once per ten
// seconds of real playback and every report that arrives is counted — there is
// no per-listener or per-day cap any more. Anti-spam is therefore just that
// cadence plus the in-memory window below, which is deliberately the same ten
// seconds: a scripted client can do no better than an honest listener.
//
// The listener identity is a salted, truncated SHA-256 of the best-effort
// client address, and it is used for nothing but the rate-limit key. It is
// never written down — not raw, not hashed.
//
// Residual risk, stated honestly: the window is per process, so it resets on
// restart, and a client that rotates its source address gets a fresh window.
// Capping that would need heavier machinery than a listening-time counter on a
// personal blog justifies.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use sha2::{Digest, Sha256};

use crate::{
    application::{
        commands::audiobook::RecordTrackPlayCommand, services::audiobook::AudiobookService,
    },
    domain::errors::audiobook::AudiobookError,
    infrastructure::web::{
        api::handlers::audiobook::response::TrackPlayResponse, server::AppState,
    },
};

/// Mixed into the listener hash. Rotating this value invalidates every key in
/// the in-memory window at once — and with it, every client's current rate
/// allowance.
const LISTENER_SALT: &str = "audiobook-play-listener-v1";

/// Minimum spacing between reports from one listener for one track. The player
/// reports every ten seconds of *real* time — its counter is driven by the
/// listener's clock, not by the audio, so playback speed does not change the
/// cadence — which makes this exactly the honest rate at every speed. Anything
/// closer together than an honest player can manage is a script, not a
/// listener.
const REPORT_MIN_INTERVAL_MS: i64 = 10_000;

/// Best-effort client identity for the rate-limit key: Cloudflare's real-client
/// header when present, else the first forwarded address, else one shared
/// bucket. Direct, un-proxied clients all share the fallback, which throttles
/// them collectively rather than letting them through — the safe direction.
fn listener_identity(headers: &HeaderMap) -> String {
    if let Some(ip) = headers
        .get("cf-connecting-ip")
        .and_then(|v| v.to_str().ok())
    {
        return ip.trim().to_string();
    }
    if let Some(forwarded) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok())
        && let Some(first) = forwarded.split(',').next()
    {
        let first = first.trim();
        if !first.is_empty() {
            return first.to_string();
        }
    }
    "unknown".to_string()
}

/// 128 bits of the salted digest: enough to key on, without keeping the address
/// around in any form.
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
) -> Result<Response, AudiobookError> {
    // The book id is route sugar (the player already knows both); the track id
    // alone identifies what was played. Malformed ids are a silent no-op.
    let _ = audiobook_id;
    let Ok(track_id) = track_id_str.parse::<i64>() else {
        return Ok(StatusCode::NO_CONTENT.into_response());
    };

    let listener = listener_hash(&listener_identity(&headers));
    if report_limited(&format!("{listener}:{track_id}")) {
        return Ok(StatusCode::NO_CONTENT.into_response());
    }

    // A counted report answers with the new total, so the player renders what
    // the server counted rather than its own assumption. An uncounted one
    // (unknown track, or a book that is not published) says nothing, and the
    // caller must leave its counter alone.
    match state
        .audiobook_service
        .record_track_play(RecordTrackPlayCommand { track_id })
        .await?
    {
        Some(play_count) => {
            Ok((StatusCode::OK, Json(TrackPlayResponse { play_count })).into_response())
        }
        None => Ok(StatusCode::NO_CONTENT.into_response()),
    }
}
