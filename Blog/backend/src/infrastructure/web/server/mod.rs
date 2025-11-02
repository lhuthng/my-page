use std::env;

use dotenvy::dotenv;
use jsonwebtoken::{DecodingKey, EncodingKey, Header};

use crate::{
    domain::entities::auth::AuthConfig,
    infrastructure::web::api::{self, services},
};

pub struct AppConfig {
    pub auth: AuthConfig,
}

pub struct AppState {
    pub config: AppConfig,
    pub auth_service: services::auth::AuthServiceImpl,
}

pub struct HTTPServer {
    addr: String,
    db_url: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        dotenv().ok();

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
            auth: AuthConfig {
                header: Header::default(),
                encoding_key: EncodingKey::from_secret(&jwt_secret),
                decoding_key: DecodingKey::from_secret(&jwt_secret),
                access_expire_hours,
                refresh_expire_hours,
            },
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
            auth_service: services::auth::AuthServiceImpl::new(pool.clone()),
        });
        let router = api::router::create(state);

        let listener = tokio::net::TcpListener::bind(self.addr).await?;
        axum::serve(listener, router).await.unwrap();

        Ok(())
    }
}
