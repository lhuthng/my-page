// Database source selection from the environment.
use std::{env, path::PathBuf};

use super::state::DatabaseSource;

impl DatabaseSource {
    pub fn from_env() -> Self {
        let url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        if let Some(path) = url.strip_prefix("sqlite:") {
            DatabaseSource::Sqlite {
                path: PathBuf::from(path),
            }
        } else {
            panic!(
                "Unsupported DATABASE_URL scheme: {}. Only sqlite: is supported.",
                url
            );
        }
    }
}
