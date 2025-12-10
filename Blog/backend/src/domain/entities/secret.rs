use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub user_id: String,
    pub role: String,
    pub exp: usize,
}

impl Claims {
    pub fn new(user_id: String, role: String, expire_hours: i64) -> Self {
        let exp = Utc::now()
            .checked_add_signed(Duration::hours(expire_hours))
            .expect("valid timestamp")
            .timestamp() as usize;
        return Self { user_id, role, exp };
    }
}
