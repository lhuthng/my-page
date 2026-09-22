// sqlx FromRow struct for subscribers.
use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;

#[derive(FromRow, Debug)]
pub(crate) struct SubscriberRow {
    pub(crate) id: i64,
    #[allow(dead_code)]
    pub(crate) email: String,
    pub(crate) status: String,
    pub(crate) confirm_token_hash: Option<String>,
    pub(crate) confirm_token_expires_at: Option<DateTime<Utc>>,
}
