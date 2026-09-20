// Project persistence adapter: implements
// `application::services::project::ProjectService` against SQLite.
use sqlx::SqlitePool;

mod links;
mod mapping;
mod read;
mod rows;
mod write;

pub struct ProjectServiceImpl {
    pub pool: SqlitePool,
}

impl ProjectServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

