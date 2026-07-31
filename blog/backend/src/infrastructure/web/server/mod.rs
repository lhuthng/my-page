use std::{
    env,
    io::{Error, ErrorKind},
    path::PathBuf,
};

use jsonwebtoken::Algorithm;

use crate::{
    domain::entities::{auth::AuthConfig, media::MediaType},
    infrastructure::{persistence, web::api},
};

pub struct AppConfig {
    pub auth: AuthConfig,
    pub mail: Option<MailConfig>,
    pub app_base_url: String,
    pub database_source: DatabaseSource,
}

pub enum DatabaseSource {
    Sqlite { path: PathBuf },
}

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

#[derive(Clone)]
pub struct MailConfig {
    pub transport: MailTransportConfig,
    pub from: String,
    pub to: String,
}

#[derive(Clone)]
pub enum MailTransportConfig {
    BrevoApi {
        api_key: String,
    },
    Smtp {
        host: String,
        port: u16,
        username: String,
        password: String,
    },
}

pub struct MediaConfig {
    pub dir: PathBuf,
    pub allowed_file_types: Vec<MediaType>,
    pub allowed_avatar_types: Vec<MediaType>,
    pub allowed_cover_types: Vec<MediaType>,
}

pub struct ProjectDemoConfig {
    pub dir: PathBuf,
    pub max_archive_size: u64,
    pub max_extracted_size: u64,
    pub max_files: usize,
    pub max_jsdos_size: u64,
    pub jsdos_chunk_size: u64,
    pub upload_session_ttl_hours: u64,
    pub max_v86_base_size: u64,
    pub max_v86_game_zip_size: u64,
    pub max_v86_game_extracted_size: u64,
    pub max_v86_game_files: usize,
    pub v86_upload_chunk_size: u64,
    pub v86_download_chunk_size: u64,
    pub v86_assets_dir: PathBuf,
    pub xorriso_bin: String,
}

pub struct AppState {
    pub config: AppConfig,
    pub media_config: MediaConfig,
    pub project_demo_config: ProjectDemoConfig,
    pub analytics_service: persistence::analytics::AnalyticsServiceImpl,
    pub auth_service: persistence::auth::AuthServiceImpl,
    pub user_service: persistence::user::UserServiceImpl,
    pub media_service: persistence::media::MediaServiceImpl,
    pub post_service: persistence::post::PostServiceImpl,
    pub project_service: persistence::project::ProjectServiceImpl,
    pub series_service: persistence::series::SeriesServiceImpl,
    pub dashboard_service: persistence::dashboard::DashboardServiceImpl,
    pub graphql_schema: crate::infrastructure::web::graphql::BlogSchema,
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
            MediaType::ModelGlb,
            MediaType::Lottie,
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
            MediaType::VideoMp4,
            MediaType::VideoWebm,
        ];

        Self {
            dir,
            allowed_file_types,
            allowed_avatar_types,
            allowed_cover_types,
        }
    }
}

impl ProjectDemoConfig {
    pub fn from_env() -> Self {
        let demo_path = env::var("PROJECT_DEMOS_PATH").expect("PROJECT_DEMOS_PATH must be set");
        let dir = PathBuf::from(&demo_path);
        if !dir.exists() {
            std::fs::create_dir_all(&dir).expect("Failed to create project demo directory");
        }

        let max_archive_size = env::var("PROJECT_DEMO_MAX_ARCHIVE_BYTES")
            .unwrap_or_else(|_| (100 * 1024 * 1024_u64).to_string())
            .parse::<u64>()
            .expect("PROJECT_DEMO_MAX_ARCHIVE_BYTES must be an integer");
        let max_extracted_size = env::var("PROJECT_DEMO_MAX_EXTRACTED_BYTES")
            .unwrap_or_else(|_| (200 * 1024 * 1024_u64).to_string())
            .parse::<u64>()
            .expect("PROJECT_DEMO_MAX_EXTRACTED_BYTES must be an integer");
        let max_files = env::var("PROJECT_DEMO_MAX_FILES")
            .unwrap_or_else(|_| "2000".to_string())
            .parse::<usize>()
            .expect("PROJECT_DEMO_MAX_FILES must be an integer");
        let max_jsdos_size = env::var("PROJECT_JSDOS_MAX_BYTES")
            .unwrap_or_else(|_| (500 * 1024 * 1024_u64).to_string())
            .parse::<u64>()
            .expect("PROJECT_JSDOS_MAX_BYTES must be an integer");
        let jsdos_chunk_size = env::var("PROJECT_JSDOS_CHUNK_BYTES")
            .unwrap_or_else(|_| (8 * 1024 * 1024_u64).to_string())
            .parse::<u64>()
            .expect("PROJECT_JSDOS_CHUNK_BYTES must be an integer");
        let upload_session_ttl_hours = env::var("PROJECT_JSDOS_UPLOAD_TTL_HOURS")
            .unwrap_or_else(|_| "24".to_string())
            .parse::<u64>()
            .expect("PROJECT_JSDOS_UPLOAD_TTL_HOURS must be an integer");
        let max_v86_base_size = env::var("PROJECT_V86_BASE_MAX_BYTES")
            .unwrap_or_else(|_| (2 * 1024 * 1024 * 1024_u64).to_string())
            .parse::<u64>()
            .expect("PROJECT_V86_BASE_MAX_BYTES must be an integer");
        let max_v86_game_zip_size = env::var("PROJECT_V86_GAME_ZIP_MAX_BYTES")
            .unwrap_or_else(|_| (500 * 1024 * 1024_u64).to_string())
            .parse::<u64>()
            .expect("PROJECT_V86_GAME_ZIP_MAX_BYTES must be an integer");
        let max_v86_game_extracted_size = env::var("PROJECT_V86_GAME_EXTRACTED_MAX_BYTES")
            .unwrap_or_else(|_| (1024 * 1024 * 1024_u64).to_string())
            .parse::<u64>()
            .expect("PROJECT_V86_GAME_EXTRACTED_MAX_BYTES must be an integer");
        let max_v86_game_files = env::var("PROJECT_V86_GAME_MAX_FILES")
            .unwrap_or_else(|_| "10000".to_string())
            .parse::<usize>()
            .expect("PROJECT_V86_GAME_MAX_FILES must be an integer");
        let v86_upload_chunk_size = env::var("PROJECT_V86_UPLOAD_CHUNK_BYTES")
            .unwrap_or_else(|_| (8 * 1024 * 1024_u64).to_string())
            .parse::<u64>()
            .expect("PROJECT_V86_UPLOAD_CHUNK_BYTES must be an integer");
        let v86_download_chunk_size = env::var("PROJECT_V86_DOWNLOAD_CHUNK_BYTES")
            .unwrap_or_else(|_| (256 * 1024_u64).to_string())
            .parse::<u64>()
            .expect("PROJECT_V86_DOWNLOAD_CHUNK_BYTES must be an integer");
        let xorriso_bin =
            env::var("PROJECT_V86_XORRISO_BIN").unwrap_or_else(|_| "xorriso".to_string());
        let v86_assets_dir = env::var("PROJECT_V86_ASSETS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"));

        Self {
            dir,
            max_archive_size,
            max_extracted_size,
            max_files,
            max_jsdos_size,
            jsdos_chunk_size,
            upload_session_ttl_hours,
            max_v86_base_size,
            max_v86_game_zip_size,
            max_v86_game_extracted_size,
            max_v86_game_files,
            v86_upload_chunk_size,
            v86_download_chunk_size,
            v86_assets_dir,
            xorriso_bin,
        }
    }
}

impl AppConfig {
    fn mail_from_env() -> Option<MailConfig> {
        let brevo_api_key = env::var("BREVO_API_KEY").ok();
        let host = env::var("SMTP_HOST").ok();
        let port = env::var("SMTP_PORT").ok();
        let username = env::var("SMTP_USERNAME").ok();
        let password = env::var("SMTP_PASSWORD").ok();
        let from = env::var("SMTP_FROM").ok();
        let to = env::var("SMTP_TO").ok();

        let any_set = [
            &brevo_api_key,
            &host,
            &port,
            &username,
            &password,
            &from,
            &to,
        ]
        .into_iter()
        .any(|value| value.is_some());

        if !any_set {
            return None;
        }

        let from = from.expect("SMTP_FROM must be set when mail is enabled");
        let to = to.expect("SMTP_TO must be set when mail is enabled");

        if let Some(api_key) = brevo_api_key {
            return Some(MailConfig {
                transport: MailTransportConfig::BrevoApi { api_key },
                from,
                to,
            });
        }

        Some(MailConfig {
            transport: MailTransportConfig::Smtp {
                host: host.expect("SMTP_HOST must be set when SMTP is enabled"),
                port: port
                    .expect("SMTP_PORT must be set when SMTP is enabled")
                    .parse::<u16>()
                    .expect("SMTP_PORT must be a valid port number"),
                username: username.expect("SMTP_USERNAME must be set when SMTP is enabled"),
                password: password.expect("SMTP_PASSWORD must be set when SMTP is enabled"),
            },
            from,
            to,
        })
    }

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
            mail: Self::mail_from_env(),
            app_base_url: env::var("APP_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:5000".to_string()),
            database_source: DatabaseSource::from_env(),
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

        let pool = sqlx::SqlitePool::connect_with(
            db_url
                .parse::<sqlx::sqlite::SqliteConnectOptions>()?
                .create_if_missing(true),
        )
        .await?;
        sqlx::migrate!().run(&pool).await?;
        cleanup_orphaned_uploads(&pool).await?;
        let graphql_schema = crate::infrastructure::web::graphql::build_schema(pool.clone());
        let state = std::sync::Arc::new(AppState {
            config: AppConfig::from_env(),
            media_config: MediaConfig::from_env(),
            project_demo_config: ProjectDemoConfig::from_env(),
            analytics_service: persistence::analytics::AnalyticsServiceImpl::new(pool.clone()),
            auth_service: persistence::auth::AuthServiceImpl::new(pool.clone()),
            user_service: persistence::user::UserServiceImpl::new(pool.clone()),
            media_service: persistence::media::MediaServiceImpl::new(pool.clone()),
            post_service: persistence::post::PostServiceImpl::new(pool.clone()),
            project_service: persistence::project::ProjectServiceImpl::new(pool.clone()),
            series_service: persistence::series::SeriesServiceImpl::new(pool.clone()),
            dashboard_service: persistence::dashboard::DashboardServiceImpl::new(pool.clone()),
            graphql_schema,
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

impl<'a> Default for HTTPServer<'a> {
    fn default() -> Self {
        Self::new()
    }
}

async fn cleanup_orphaned_uploads(pool: &sqlx::SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    let rows: Vec<(String, i64, i64)> = sqlx::query_as(
        r#"SELECT s.id, s.system_id, s.expected_current_version
           FROM v86_system_upload_sessions s
           WHERE s.system_id IS NOT NULL
             AND (s.status = 'building'
                  OR (s.status = 'active' AND s.expires_at < datetime('now')))"#,
    )
    .fetch_all(pool)
    .await?;

    for (session_id, system_id, expected_version) in &rows {
        if *expected_version == 0 {
            sqlx::query("DELETE FROM v86_systems WHERE id = ?")
                .bind(system_id)
                .execute(pool)
                .await?;
        } else {
            sqlx::query(
                "UPDATE v86_systems SET current_version = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND current_version = ?",
            )
            .bind(expected_version)
            .bind(system_id)
            .bind(expected_version + 1)
            .execute(pool)
            .await?;
        }
        sqlx::query(
            "UPDATE v86_system_upload_sessions SET status = 'expired', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(session_id)
        .execute(pool)
        .await?;
    }

    if !rows.is_empty() {
        println!("Cleaned up {} orphaned upload session(s)", rows.len());
    }

    Ok(())
}
