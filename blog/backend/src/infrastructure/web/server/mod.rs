use std::{
    env,
    io::{Error, ErrorKind},
    path::PathBuf,
};

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
    pub allowed_avatar_types: Vec<MediaType>,
    pub allowed_cover_types: Vec<MediaType>,
}

pub struct AppState {
    pub config: AppConfig,
    pub media_config: MediaConfig,
    pub auth_service: services::auth::AuthServiceImpl,
    pub user_service: services::user::UserServiceImpl,
    pub media_service: services::media::MediaServiceImpl,
    pub post_service: services::post::PostServiceImpl,
    pub series_service: services::series::SeriesServiceImpl,
}

pub struct HTTPServer<'a> {
    addr: Option<&'a str>,
    port: Option<&'a str>,
    db_url: Option<&'a str>,
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
            MediaType::ImageJpeg,
            MediaType::VideoMp4,
            MediaType::VideoWebm,
            MediaType::AudioMp3,
            MediaType::AudioOgg,
            MediaType::AudioMp3,
        ];

        let allowed_avatar_types = vec![
            MediaType::ImagePng,
            MediaType::ImageGif,
            MediaType::ImageWebp,
            MediaType::ImageJpeg,
        ];

        let allowed_cover_types = vec![
            MediaType::ImagePng,
            MediaType::ImageGif,
            MediaType::ImageWebp,
            MediaType::ImageJpeg,
        ];

        return Self {
            dir,
            allowed_file_types,
            allowed_avatar_types,
            allowed_cover_types,
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

impl<'a> HTTPServer<'a> {
    pub fn new() -> Self {
        Self {
            addr: None,
            port: None,
            db_url: None,
        }
    }

    pub fn set_addr(&mut self, addr: &'a str) -> &mut Self {
        self.addr = Some(addr);
        self
    }

    pub fn set_port(&mut self, port: &'a str) -> &mut Self {
        self.port = Some(port);
        self
    }

    pub fn set_db(&mut self, db_url: &'a str) -> &mut Self {
        self.db_url = Some(db_url);
        self
    }

    pub async fn start(self) -> Result<(), Box<dyn std::error::Error>> {
        let db_url = self
            .db_url
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "missing database url"))?;

        println!("Connecting to {}", db_url);

        let pool = sqlx::SqlitePool::connect(&db_url).await?;
        let state = std::sync::Arc::new(AppState {
            config: AppConfig::from_env(),
            media_config: MediaConfig::from_env(),
            auth_service: services::auth::AuthServiceImpl::new(pool.clone()),
            user_service: services::user::UserServiceImpl::new(pool.clone()),
            media_service: services::media::MediaServiceImpl::new(pool.clone()),
            post_service: services::post::PostServiceImpl::new(pool.clone()),
            series_service: services::series::SeriesServiceImpl::new(pool.clone()),
        });
        let router = api::router::build_router(state);

        let addr = self.addr.unwrap_or("127.0.0.1");
        let port = self.port.unwrap_or("3000");

        let addr = format!("{}:{}", addr, port);
        println!("Starting {}", &addr);
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, router).await.unwrap();

        Ok(())
    }
}
