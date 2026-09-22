// sqlx FromRow structs for the auth flows.
use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;

#[derive(FromRow, Debug)]
pub(crate) struct UserRow {
    pub(crate) id: i64,
    pub(crate) username: String,
    pub(crate) password_hash: String,
    pub(crate) email: String,
    pub(crate) role: String,
    pub(crate) email_verified_at: Option<DateTime<Utc>>,
}

#[derive(FromRow, Debug)]
pub(crate) struct SessionRow {
    pub(crate) user_id: i64,
    pub(crate) role: String,
    pub(crate) token_hash: String,
    pub(crate) expires_at: DateTime<Utc>,
}

#[derive(FromRow, Debug)]
pub(crate) struct VerificationRow {
    pub(crate) token_hash: String,
    pub(crate) expires_at: DateTime<Utc>,
    pub(crate) sent_at: DateTime<Utc>,
}
