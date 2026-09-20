// sqlx FromRow struct for subscribers.
use sqlx::prelude::FromRow;

#[derive(FromRow, Debug)]
struct SubscriberRow {
    id: i64,
    #[allow(dead_code)]
    email: String,
    status: String,
    confirm_token_hash: Option<String>,
    confirm_token_expires_at: Option<DateTime<Utc>>,
}

