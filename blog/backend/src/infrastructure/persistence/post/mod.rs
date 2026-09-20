// Post persistence adapter: implements `application::services::post::PostService`
// against SQLite. The service methods are split across `read`, `detail`,
// `write`, `comments`, and `threads`; each contributes its own
// `impl PostService for PostServiceImpl` block.
use sqlx::SqlitePool;

mod comments;
mod detail;
mod links;
mod mapping;
mod read;
mod rows;
mod threads;
mod write;

pub use rows::{
    MediumUsageRow, MediumUsageWithNameRow, PostContentRow, PostDetailsRow, PostRow,
    PostSearchRow, TagRow, TagSummaryRow,
};

pub struct PostServiceImpl {
    pub pool: SqlitePool,
}

impl PostServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

