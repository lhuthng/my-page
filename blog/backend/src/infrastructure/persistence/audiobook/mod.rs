// Audiobook persistence adapter: implements
// `application::services::audiobook::AudiobookService` against SQLite.
use sqlx::SqlitePool;

mod mapping;
mod medium;
mod read;
mod rows;
mod tags;
mod tracks;
mod validation;
mod write;

pub struct AudiobookServiceImpl {
    pub pool: SqlitePool,
}

impl AudiobookServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

/// Columns shared by every audiobook summary query, in a fixed order.
