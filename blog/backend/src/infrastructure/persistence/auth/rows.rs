// sqlx FromRow structs for the auth flows.
use sqlx::prelude::FromRow;

#[derive(FromRow, Debug)]
struct UserRow {
    id: i64,
    username: String,
    password_hash: String,
    email: String,
    role: String,
    email_verified_at: Option<DateTime<Utc>>,
}

#[derive(FromRow, Debug)]
struct SessionRow {
    user_id: i64,
    role: String,
    token_hash: String,
    expires_at: DateTime<Utc>,
}

#[derive(FromRow, Debug)]
struct VerificationRow {
    token_hash: String,
    expires_at: DateTime<Utc>,
    sent_at: DateTime<Utc>,
}

