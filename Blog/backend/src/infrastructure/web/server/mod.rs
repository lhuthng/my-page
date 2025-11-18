use std::{env, path::PathBuf};

use jsonwebtoken::Algorithm;

use crate::{
    domain::entities::{auth::AuthConfig, media::MediaType},
    infrastructure::web::api::{self, services},
};

pub struct AppConfig {
    pub auth: AuthConfig,
}

pub struct MediaConfig {
    pub dir: PathBuf,
    pub allowed_file_types: Vec<MediaType>,
}

pub struct AppState {
    pub config: AppConfig,
    pub media_config: MediaConfig,
    pub auth_service: services::auth::AuthServiceImpl,
    pub user_service: services::user::UserServiceImpl,
    pub media_service: services::media::MediaServiceImpl,
}

pub struct HTTPServer {
    addr: String,
    db_url: String,
}

impl MediaConfig {
    pub fn from_env() -> Self {
        let media_path = env::var("MEDIA_PATH").expect("MEDIA_PATH must be set");

        let dir = PathBuf::from(&media_path);
        if !dir.exists() {
            std::fs::create_dir_all(&dir).expect("Failed to create media directory");
        }

        let allowed_file_types = vec![
            MediaType::ImagePng,
            MediaType::ImageGif,
            MediaType::ImageWebp,
            MediaType::ImageJpeg
        ];

        return Self {
            dir,
            allowed_file_types,
        };
    }
}

impl AppConfig {
    pub fn from_env() -> Self {
        let jwt_secret = env::var("JWT_SECRET")
            .expect("JWT_SECRET must be set")
            .into_bytes();

        let access_expire_hours = env::var("ACCESS_JWT_EXP_HOURS")
            .unwrap_or_else(|_| "24".to_string())
            .parse::<i64>()
            .expect("ACCEST_JWT_EXP_HOURS must be an integer");

        let refresh_expire_hours = env::var("REFRESH_JWT_EXP_HOURS")
            .unwrap_or_else(|_| "360".to_string())
            .parse::<i64>()
            .expect("REFRESH_JWT_EXP_HOURS must be an integer");

        Self {
            auth: AuthConfig::new(
                Algorithm::HS256,
                jwt_secret,
                access_expire_hours,
                refresh_expire_hours,
            ),
        }
    }
}

impl HTTPServer {
    pub fn new(s_addr: &str, s_db_url: &str) -> HTTPServer {
        let addr = s_addr.to_string();
        let db_url = s_db_url.to_string();
        Self { addr, db_url }
    }

    pub async fn start(self) -> Result<(), Box<dyn std::error::Error>> {
        let pool = sqlx::SqlitePool::connect(&self.db_url).await?;
        let state = std::sync::Arc::new(AppState {
            config: AppConfig::from_env(),
            media_config: MediaConfig::from_env(),
            auth_service: services::auth::AuthServiceImpl::new(pool.clone()),
            user_service: services::user::UserServiceImpl::new(pool.clone()),
            media_service: services::media::MediaServiceImpl::new(pool.clone()),
        });
        let router = api::router::build_router(state);

        let listener = tokio::net::TcpListener::bind(self.addr).await?;
        axum::serve(listener, router).await.unwrap();

        Ok(())
    }
}
