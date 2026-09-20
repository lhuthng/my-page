// Newsletter persistence adapter: implements
// `application::services::newsletter::NewsletterService` against SQLite.
use sqlx::SqlitePool;

mod campaigns;
mod rows;
mod subscribers;

const DELIMITER: char = '`';
const CONFIRM_TOKEN_EXPIRY_MINUTES: i64 = 30;
const CAMPAIGN_CHUNK_SIZE: usize = 25;
const CAMPAIGN_CHUNK_DELAY_MS: u64 = 300;

pub struct NewsletterServiceImpl {
    pub pool: SqlitePool,
}

impl NewsletterServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

pub struct NewsletterServiceImpl {
    pub pool: SqlitePool,
}

impl NewsletterServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

