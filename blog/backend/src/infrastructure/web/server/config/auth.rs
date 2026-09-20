// JWT settings for the auth aggregate, built from the environment.
use jsonwebtoken::Algorithm;

use crate::domain::entities::auth::AuthConfig;

pub fn from_env() -> AuthConfig {
    let jwt_secret = std::env::var("JWT_SECRET")
        .expect("JWT_SECRET must be set")
        .into_bytes();

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
