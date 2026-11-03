// Captcha settings for the public auth endpoints.
//
// The backend is reachable directly at the API origin, so a check that only
// runs in the SvelteKit proxy protects just one of the two entry points.
// Turnstile is verified here instead, and the proxy forwards the token.

pub struct SecurityConfig {
    /// Cloudflare Turnstile secret. Unset in development.
    pub turnstile_secret: Option<String>,
}

impl SecurityConfig {
    pub fn from_env() -> Self {
        let turnstile_secret = std::env::var("TURNSTILE_SECRET_KEY")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        Self { turnstile_secret }
    }
}
