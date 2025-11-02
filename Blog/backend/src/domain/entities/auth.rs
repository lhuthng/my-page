use jsonwebtoken::{DecodingKey, EncodingKey, Header};

#[derive(Debug, Clone)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub header: Header,
    pub encoding_key: EncodingKey,
    pub decoding_key: DecodingKey,
    pub access_expire_hours: i64,
    pub refresh_expire_hours: i64,
}
