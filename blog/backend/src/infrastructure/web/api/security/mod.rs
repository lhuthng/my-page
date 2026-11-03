// Guards for the public auth endpoints: Turnstile verification, client
// identity, and a process-local fixed-window rate limiter.
//
// The backend is reachable directly at the API origin, so these checks live
// here rather than only in the SvelteKit proxy.
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use axum::http::HeaderMap;
use sha2::{Digest, Sha256};
use tracing::error;

use crate::domain::errors::auth::AuthError;

const TURNSTILE_VERIFY_URL: &str = "https://challenges.cloudflare.com/turnstile/v0/siteverify";

/// Best-effort client address for throttling: Cloudflare's real-client header
/// when present, then the address the SvelteKit proxy forwards, then the first
/// forwarded hop, then one shared bucket. Un-proxied callers share the fallback,
/// which throttles them collectively rather than letting them through.
pub fn client_ip(headers: &HeaderMap) -> Option<String> {
    // Cloudflare overwrites this header, so it cannot be forged by a client
    // that reaches the origin through the CDN.
    for name in ["cf-connecting-ip", "x-client-ip"] {
        if let Some(ip) = headers.get(name).and_then(|v| v.to_str().ok()) {
            let ip = ip.trim();
            if !ip.is_empty() {
                return Some(ip.to_string());
            }
        }
    }

    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|forwarded| forwarded.split(',').next())
        .map(str::trim)
        .filter(|first| !first.is_empty())
        .map(str::to_string)
}

fn digest_key(scope: &str, value: &str) -> String {
    let digest = Sha256::digest(format!("{scope}|{value}").as_bytes());
    hex::encode(&digest[..12])
}

/// Rate-limit key for a client address, hashed so the limiter never stores raw
/// IPs.
pub fn client_key(scope: &str, headers: &HeaderMap) -> String {
    let ip = client_ip(headers).unwrap_or_else(|| "unknown".to_string());
    format!("{scope}:ip:{}", digest_key(scope, &ip))
}

/// Rate-limit key for a submitted username, email, or identifier.
pub fn identifier_key(scope: &str, identifier: &str) -> String {
    format!(
        "{scope}:id:{}",
        digest_key(scope, &identifier.trim().to_lowercase())
    )
}

/// Process-local fixed-window limiter. Entries are pruned on every hit once
/// the map grows past a small bound, so memory stays flat under spraying.
#[derive(Default)]
pub struct RateLimiter {
    hits: Mutex<HashMap<String, Vec<i64>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn check(&self, key: &str, max_requests: usize, window: Duration) -> Result<(), AuthError> {
        let now = chrono::Utc::now().timestamp_millis();
        let window_ms = i64::try_from(window.as_millis()).unwrap_or(i64::MAX);
        let cutoff = now - window_ms;

        let mut hits = self
            .hits
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if hits.len() > 10_000 {
            hits.retain(|_, stamps| stamps.last().is_some_and(|last| *last > cutoff));
        }

        let stamps = hits.entry(key.to_string()).or_default();
        stamps.retain(|stamp| *stamp > cutoff);

        if stamps.len() >= max_requests {
            let retry_after = stamps
                .first()
                .map(|oldest| ((oldest + window_ms - now) / 1000).max(1))
                .unwrap_or(1);
            return Err(AuthError::RateLimited {
                retry_after_secs: u64::try_from(retry_after).unwrap_or(1),
            });
        }

        stamps.push(now);
        Ok(())
    }
}

#[derive(serde::Deserialize)]
struct TurnstileResponse {
    success: bool,
}

/// Verify a Turnstile token against Cloudflare.
///
/// Without a secret, development skips the check so `cargo run` works. In a
/// release build the absence of a secret fails closed: a deployment that forgot
/// the key should stop registrations, not silently lose the captcha.
pub async fn verify_turnstile(
    client: &reqwest::Client,
    secret: Option<&str>,
    remote_ip: Option<&str>,
    token: Option<&str>,
) -> Result<(), AuthError> {
    let Some(secret) = secret else {
        return if cfg!(debug_assertions) {
            Ok(())
        } else {
            error!("TURNSTILE_SECRET_KEY is not set; refusing unverified auth requests");
            Err(AuthError::CaptchaUnavailable)
        };
    };

    let Some(token) = token.map(str::trim).filter(|token| !token.is_empty()) else {
        return Err(AuthError::CaptchaRequired);
    };

    let mut form = HashMap::new();
    form.insert("secret", secret);
    form.insert("response", token);
    if let Some(ip) = remote_ip {
        form.insert("remoteip", ip);
    }

    let response = client
        .post(TURNSTILE_VERIFY_URL)
        .form(&form)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|err| {
            error!("Turnstile verification request failed: {err}");
            AuthError::CaptchaUnavailable
        })?;

    if !response.status().is_success() {
        error!("Turnstile verification returned HTTP {}", response.status());
        return Err(AuthError::CaptchaUnavailable);
    }

    let payload = response.json::<TurnstileResponse>().await.map_err(|err| {
        error!("Turnstile verification response was not JSON: {err}");
        AuthError::CaptchaUnavailable
    })?;

    if payload.success {
        Ok(())
    } else {
        Err(AuthError::CaptchaFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.insert(*name, HeaderValue::from_static(value));
        }
        headers
    }

    #[test]
    fn client_ip_prefers_cloudflare_then_proxy_header() {
        assert_eq!(
            client_ip(&headers(&[
                ("cf-connecting-ip", "203.0.113.7"),
                ("x-client-ip", "10.0.0.2"),
            ])),
            Some("203.0.113.7".to_string())
        );
        assert_eq!(
            client_ip(&headers(&[("x-client-ip", "10.0.0.2")])),
            Some("10.0.0.2".to_string())
        );
        assert_eq!(
            client_ip(&headers(&[("x-forwarded-for", "198.51.100.9, 10.0.0.2")])),
            Some("198.51.100.9".to_string())
        );
        assert_eq!(client_ip(&HeaderMap::new()), None);
    }

    #[test]
    fn identifier_keys_do_not_keep_the_submitted_value() {
        let key = identifier_key("login", "Alice@Example.com ");
        assert!(key.starts_with("login:id:"));
        assert!(!key.to_lowercase().contains("alice"));
        // Case and surrounding whitespace do not create extra buckets.
        assert_eq!(key, identifier_key("login", "alice@example.com"));
    }

    #[test]
    fn limiter_counts_then_refuses_inside_the_window() {
        let limiter = RateLimiter::new();
        let window = Duration::from_secs(60);
        for _ in 0..3 {
            assert!(limiter.check("k", 3, window).is_ok());
        }

        match limiter.check("k", 3, window) {
            Err(AuthError::RateLimited { retry_after_secs }) => {
                assert!((1..=60).contains(&retry_after_secs));
            }
            _ => panic!("expected a rate limit error"),
        }

        // A different key has its own window.
        assert!(limiter.check("other", 3, window).is_ok());
    }
}
