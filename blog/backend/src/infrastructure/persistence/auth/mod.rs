// Auth persistence adapter: implements
// `application::services::auth::AuthService` against SQLite.
use sqlx::SqlitePool;

mod credentials;
mod rows;
mod sessions;
mod tokens;

const DELIMITER: char = '`';
const EMAIL_VERIFICATION_EXPIRY_MINUTES: i64 = 30;
const RESEND_VERIFICATION_COOLDOWN_SECONDS: i64 = 60;
const PASSWORD_RESET_EXPIRY_MINUTES: i64 = 30;
const PASSWORD_RESET_COOLDOWN_SECONDS: i64 = 60;

pub struct AuthServiceImpl {
    pub pool: SqlitePool,
}

impl AuthServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

