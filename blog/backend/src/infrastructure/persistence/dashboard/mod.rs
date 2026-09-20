// Dashboard persistence adapter: implements
// `application::services::dashboard::DashboardService` against SQLite.
use sqlx::SqlitePool;

mod overview;
mod rows;
mod shared;
mod tags;
mod views;

pub struct DashboardServiceImpl {
    pub pool: SqlitePool,
}

impl DashboardServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

