use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header};
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Clone)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub algorithm: Algorithm,
    pub header: Header,
    pub encoding_key: EncodingKey,
    pub decoding_key: DecodingKey,
    pub access_expire_hours: i64,
    pub refresh_expire_hours: i64,
}

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterCredentials {
    #[validate(length(min = 3, message = "Username must have at least 3 characters"))]
    pub username: String,

    #[validate(email(message = "Invalid email address"))]
    pub email: String,

    #[validate(length(min = 6, message = "Password must be at least 6 characters"))]
    pub password: String,
}

impl AuthConfig {
    pub fn new(
        algorithm: Algorithm,
        jwt_key: Vec<u8>,
        access_expire_hours: i64,
        refresh_expire_hours: i64,
    ) -> Self {
        Self {
            algorithm,
            header: Header::new(algorithm),
            encoding_key: EncodingKey::from_secret(&jwt_key),
            decoding_key: DecodingKey::from_secret(&jwt_key),
            access_expire_hours,
            refresh_expire_hours,
        }
    }
}
