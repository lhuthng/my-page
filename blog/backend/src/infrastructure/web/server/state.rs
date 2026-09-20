// The composition root's state: `AppState` (one service per aggregate) and
// the top-level `AppConfig`. Sub-configs live in `config/` and are selected
// from the environment there.
use std::path::PathBuf;

use crate::domain::entities::auth::AuthConfig;
use crate::infrastructure::{persistence, storage::ObjectStore, web::graphql};

pub use super::config::mail::{MailConfig, MailTransportConfig};
pub use super::config::media::MediaConfig;
pub use super::config::project_demo::ProjectDemoConfig;

pub struct AppConfig {
    pub auth: AuthConfig,
    pub mail: Option<MailConfig>,
    pub app_base_url: String,
    pub database_source: DatabaseSource,
    pub r2_public_url: Option<String>,
}

pub enum DatabaseSource {
    Sqlite { path: PathBuf },
}


pub struct AppState {
    pub config: AppConfig,
    pub media_config: MediaConfig,
    pub project_demo_config: ProjectDemoConfig,
    pub storage: ObjectStore,
    pub analytics_service: persistence::analytics::AnalyticsServiceImpl,
    pub auth_service: persistence::auth::AuthServiceImpl,
    pub user_service: persistence::user::UserServiceImpl,
    pub media_service: persistence::media::MediaServiceImpl,
    pub post_service: persistence::post::PostServiceImpl,
    pub project_service: persistence::project::ProjectServiceImpl,
    pub game_service: persistence::game::GameServiceImpl,
    pub series_service: persistence::series::SeriesServiceImpl,
    pub audiobook_service: persistence::audiobook::AudiobookServiceImpl,
    pub dashboard_service: persistence::dashboard::DashboardServiceImpl,
    pub newsletter_service: persistence::newsletter::NewsletterServiceImpl,
    pub graphql_schema: crate::infrastructure::web::graphql::BlogSchema,
}

impl AppState {
    /// Public base URL for v86 artifact links: the R2 public domain in R2
    /// mode, None in filesystem mode (relative URLs served by this backend).
    pub fn artifact_base_url(&self) -> Option<&str> {
        match self.storage {
            ObjectStore::R2(_) => self.config.r2_public_url.as_deref(),
            ObjectStore::Fs(_) => None,
        }
    }
}


impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            auth: super::config::auth::from_env(),
            mail: super::config::mail::MailConfig::from_env(),
            app_base_url: std::env::var("APP_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:5000".to_string()),
            database_source: DatabaseSource::from_env(),
            r2_public_url: std::env::var("R2_PUBLIC_URL").ok(),
        }
    }
}
