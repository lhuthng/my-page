// Game persistence adapter: implements
// `application::services::game::GameService` against SQLite.
use sqlx::SqlitePool;

mod mapping;
mod read;
mod rows;
mod write;

pub struct GameServiceImpl {
    pub pool: SqlitePool,
}

impl GameServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

