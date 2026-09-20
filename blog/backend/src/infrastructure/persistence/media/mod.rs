// Media persistence adapter: implements
// `application::services::media::MediaService` against SQLite + the object
// store. Split by concern; each trait-impl file carries its own
// `#[async_trait] impl MediaService` block.
use sqlx::SqlitePool;

mod aliases;
mod avatar;
mod covers;
mod crud;
mod files;
mod hashing;
mod rows;
mod search;
mod upload;
mod validation;

pub use files::clean_up_files;
pub use hashing::hash_bytes;

pub struct MediaServiceImpl {
    pub pool: SqlitePool,
}

impl MediaServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

