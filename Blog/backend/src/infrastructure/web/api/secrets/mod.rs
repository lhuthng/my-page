use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};

use crate::domain::{entities::secret::Claims, errors::auth::AuthError};

pub async fn encode_into_jwt_token(
    value: String,
    expire_hours: i64,
    header: &Header,
    encoding_key: &EncodingKey,
) -> Result<String, AuthError> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(expire_hours))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: value.to_string(),
        exp: expiration,
    };

    encode(&header, &claims, &encoding_key)
        .map_err(|_| AuthError::InternalError("Access Token Creation".to_string()))
}

pub async fn decode_from_jwt_token(
    token: String,
    algorithm: &Algorithm,
    decoding_key: &DecodingKey,
) -> Result<Claims, AuthError> {
    match decode::<Claims>(token, &decoding_key, &Validation::new(algorithm.clone())) {
        Ok(data) => Ok(data.claims),
        Err(_) => Err(AuthError::InvalidToken),
    }
}
