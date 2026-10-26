// JWT settings for the auth aggregate, built from the environment.
use jsonwebtoken::Algorithm;

use crate::domain::entities::auth::AuthConfig;

/// The HS256 signing key, validated once per use.
///
/// An empty or placeholder secret still signs and verifies tokens perfectly
/// well, so nothing downstream can tell that the deployment is forgeable. Both
/// consumers (token signing and the newsletter unsubscribe derivation) go
/// through here so the check cannot drift.
pub fn jwt_secret() -> String {
    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    assert!(
        !secret.contains("REPLACE_WITH_YOUR_JWT_SECRET") && !secret.contains("CHANGE_ME"),
        "JWT_SECRET is still a placeholder value; set a real random secret"
    );

    // An HS256 key shorter than the 32-byte hash output is brute-forceable.
    // Release builds refuse it; a debug build only warns so the checked-in
    // development secret does not block `cargo run`.
    if secret.len() < 32 {
        if cfg!(debug_assertions) {
            eprintln!(
                "warning: JWT_SECRET is shorter than 32 characters; \
                 run `openssl rand -hex 32` before deploying"
            );
        } else {
            panic!(
                "JWT_SECRET must be at least 32 characters; \
                 generate one with `openssl rand -hex 32`"
            );
        }
    }

    secret
}

pub fn from_env() -> AuthConfig {
    let jwt_secret = jwt_secret().into_bytes();

    let access_expire_hours = std::env::var("ACCESS_JWT_EXP_HOURS")
        .unwrap_or_else(|_| "24".to_string())
        .parse::<i64>()
        .expect("ACCEST_JWT_EXP_HOURS must be an integer");

    let refresh_expire_hours = std::env::var("REFRESH_JWT_EXP_HOURS")
        .unwrap_or_else(|_| "360".to_string())
        .parse::<i64>()
        .expect("REFRESH_JWT_EXP_HOURS must be an integer");

    AuthConfig::new(
        Algorithm::HS256,
        jwt_secret,
        access_expire_hours,
        refresh_expire_hours,
    )
}
