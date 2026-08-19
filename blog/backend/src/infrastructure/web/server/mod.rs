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
    pub r2_public_url: Option<String>,
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
    pub max_v86_game_extracted_size: u64,
    pub v86_upload_chunk_size: u64,
    pub v86_download_chunk_size: u64,
    pub v86_assets_dir: PathBuf,
}

pub struct AppState {
    pub config: AppConfig,
    pub media_config: MediaConfig,
    pub project_demo_config: ProjectDemoConfig,
    pub r2: Option<crate::infrastructure::storage::r2::R2Client>,
    pub analytics_service: persistence::analytics::AnalyticsServiceImpl,
    pub auth_service: persistence::auth::AuthServiceImpl,
    pub user_service: persistence::user::UserServiceImpl,
    pub media_service: persistence::media::MediaServiceImpl,
    pub post_service: persistence::post::PostServiceImpl,
    pub project_service: persistence::project::ProjectServiceImpl,
    pub series_service: persistence::series::SeriesServiceImpl,
    pub dashboard_service: persistence::dashboard::DashboardServiceImpl,
    pub newsletter_service: persistence::newsletter::NewsletterServiceImpl,
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
        let max_v86_game_extracted_size = env::var("PROJECT_V86_GAME_EXTRACTED_MAX_BYTES")
            .unwrap_or_else(|_| (1024 * 1024 * 1024_u64).to_string())
            .parse::<u64>()
            .expect("PROJECT_V86_GAME_EXTRACTED_MAX_BYTES must be an integer");
        let v86_upload_chunk_size = env::var("PROJECT_V86_UPLOAD_CHUNK_BYTES")
            .unwrap_or_else(|_| (8 * 1024 * 1024_u64).to_string())
            .parse::<u64>()
            .expect("PROJECT_V86_UPLOAD_CHUNK_BYTES must be an integer");
        let v86_download_chunk_size = env::var("PROJECT_V86_DOWNLOAD_CHUNK_BYTES")
            .unwrap_or_else(|_| (256 * 1024_u64).to_string())
            .parse::<u64>()
            .expect("PROJECT_V86_DOWNLOAD_CHUNK_BYTES must be an integer");
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
            max_v86_game_extracted_size,
            v86_upload_chunk_size,
            v86_download_chunk_size,
            v86_assets_dir,
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
            r2_public_url: env::var("R2_PUBLIC_URL").ok(),
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
                .create_if_missing(true)
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
                .synchronous(sqlx::sqlite::SqliteSynchronous::Normal),
        )
        .await?;
        // Run migrations on a dedicated pool with foreign key enforcement
        // disabled. sqlx wraps each migration in a transaction, where SQLite
        // ignores PRAGMA foreign_keys, so table-rebuild migrations (e.g.
        // relaxing a UNIQUE constraint) need FK checks off at connect time.
        let migration_pool = sqlx::SqlitePool::connect_with(
            db_url
                .parse::<sqlx::sqlite::SqliteConnectOptions>()?
                .create_if_missing(true)
                .foreign_keys(false)
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
                .synchronous(sqlx::sqlite::SqliteSynchronous::Normal),
        )
        .await?;
        sqlx::migrate!().run(&migration_pool).await?;
        drop(migration_pool);
        backfill_reading_times(&pool).await?;
        let project_demo_config = ProjectDemoConfig::from_env();
        let r2 = crate::infrastructure::storage::r2::R2Client::from_env();
        cleanup_orphaned_uploads(&pool, &r2, &project_demo_config.dir).await?;
        let graphql_schema = crate::infrastructure::web::graphql::build_schema(pool.clone());
        let state = std::sync::Arc::new(AppState {
            config: AppConfig::from_env(),
            media_config: MediaConfig::from_env(),
            project_demo_config,
            r2,
            analytics_service: persistence::analytics::AnalyticsServiceImpl::new(pool.clone()),
            auth_service: persistence::auth::AuthServiceImpl::new(pool.clone()),
            user_service: persistence::user::UserServiceImpl::new(pool.clone()),
            media_service: persistence::media::MediaServiceImpl::new(pool.clone()),
            post_service: persistence::post::PostServiceImpl::new(pool.clone()),
            project_service: persistence::project::ProjectServiceImpl::new(pool.clone()),
            series_service: persistence::series::SeriesServiceImpl::new(pool.clone()),
            dashboard_service: persistence::dashboard::DashboardServiceImpl::new(pool.clone()),
            newsletter_service: persistence::newsletter::NewsletterServiceImpl::new(pool.clone()),
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

async fn cleanup_orphaned_uploads(
    pool: &sqlx::SqlitePool,
    r2: &Option<crate::infrastructure::storage::r2::R2Client>,
    demos_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    async fn clean_game_session(
        pool: &sqlx::SqlitePool,
        r2: &Option<crate::infrastructure::storage::r2::R2Client>,
        demos_dir: &std::path::Path,
        session_id: &str,
    ) {
        use sqlx::Row;
        let row = sqlx::query(
            "SELECT staged_disk_storage_key, disk_reuse FROM project_v86_upload_sessions WHERE id = ?",
        )
        .bind(session_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();
        if let Some(row) = row {
            // Game sessions upload straight to content-addressed keys; only the
            // objects this session uploaded (never shared/reused ones) are
            // removed, matching the abort path.
            if let Some(client) = r2 {
                let disk_reuse: i64 = row.get("disk_reuse");
                if disk_reuse == 0 {
                    if let Some(key) = row.get::<Option<String>, _>("staged_disk_storage_key") {
                        let _ = client.delete_prefix(&key).await;
                    }
                }
                let uploaded_isos: Vec<String> = sqlx::query_scalar(
                    "SELECT iso_storage_key FROM project_v86_staged_variants WHERE upload_id = ? AND reuse = 0",
                )
                .bind(session_id)
                .fetch_all(pool)
                .await
                .unwrap_or_default();
                for key in uploaded_isos {
                    let _ = client.delete_object(&format!("{key}/full.iso")).await;
                }
            }
            let _ = tokio::fs::remove_dir_all(demos_dir.join("v86/tmp/build").join(session_id))
                .await;
        }
    }

    async fn clean_system_session(
        pool: &sqlx::SqlitePool,
        r2: &Option<crate::infrastructure::storage::r2::R2Client>,
        session_id: &str,
    ) {
        use sqlx::Row;
        let row = sqlx::query(
            "SELECT staged_storage_key, reuse FROM v86_system_upload_sessions WHERE id = ?",
        )
        .bind(session_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();
        if let Some(row) = row {
            let reuse: i64 = row.get("reuse");
            if reuse == 0 {
                if let Some(client) = r2 {
                    if let Some(key) = row.get::<Option<String>, _>("staged_storage_key") {
                        // Only delete if no other active session still owns this key.
                        let other_owners: i64 = sqlx::query_scalar(
                            "SELECT COUNT(*) FROM v86_system_upload_sessions
                             WHERE id != ? AND staged_storage_key = ?
                             AND status IN ('active', 'building')",
                        )
                        .bind(session_id)
                        .bind(&key)
                        .fetch_one(pool)
                        .await
                        .unwrap_or(0);
                        if other_owners == 0 {
                            let _ = client.delete_prefix(&key).await;
                        }
                    }
                }
            }
        }
    }

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
        // Drop any version row a crashed build may have reserved before it finished.
        let orphan_version_id: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM v86_system_versions WHERE system_id = ? AND version_number = ?",
        )
        .bind(system_id)
        .bind(expected_version + 1)
        .fetch_optional(pool)
        .await?;
        if let Some(version_id) = orphan_version_id {
            // Grab the content-addressed storage key before dropping the row.
            let orphan_key: Option<String> = sqlx::query_scalar(
                "SELECT storage_key FROM v86_system_versions WHERE id = ?",
            )
            .bind(version_id)
            .fetch_optional(pool)
            .await?;
            sqlx::query("DELETE FROM v86_system_versions WHERE id = ?")
                .bind(version_id)
                .execute(pool)
                .await?;
            // Only free the objects if no other version (e.g. a shared/deduped
            // image) still references the same content-addressed key.
            if let Some(key) = orphan_key {
                let remaining: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM v86_system_versions WHERE storage_key = ?",
                )
                .bind(&key)
                .fetch_one(pool)
                .await?;
                if remaining == 0 {
                    if let Some(client) = r2 {
                        let _ = client.delete_prefix(&key).await;
                    }
                }
            }
        }

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
        clean_system_session(pool, r2, session_id).await;
    }

    let stale_games: Vec<String> = sqlx::query_scalar(
        r#"SELECT id FROM project_v86_upload_sessions
           WHERE status = 'building'
              OR (status = 'active' AND expires_at < datetime('now'))"#,
    )
    .fetch_all(pool)
    .await?;
    for session_id in &stale_games {
        sqlx::query(
            "UPDATE project_v86_upload_sessions SET status = 'expired', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(session_id)
        .execute(pool)
        .await?;
        clean_game_session(pool, r2, demos_dir, session_id).await;
    }

    // Finished sessions (consumed/aborted/ready/expired) are never removed
    // otherwise; once they age out of the TTL they are safe to drop.
    let terminal_games: Vec<String> = sqlx::query_scalar(
        r#"SELECT id FROM project_v86_upload_sessions
           WHERE status != 'active' AND status != 'building'
             AND expires_at < datetime('now')"#,
    )
    .fetch_all(pool)
    .await?;
    for session_id in &terminal_games {
        sqlx::query("DELETE FROM project_v86_upload_sessions WHERE id = ?")
            .bind(session_id)
            .execute(pool)
            .await?;
    }

    let stale_systems = rows.len();
    if stale_systems + stale_games.len() > 0 {
        println!(
            "Cleaned up {} orphaned upload session(s)",
            stale_systems + stale_games.len()
        );
    }

    Ok(())
}

async fn backfill_reading_times(
    pool: &sqlx::SqlitePool,
) -> Result<(), Box<dyn std::error::Error>> {
    let rows: Vec<(i64, String)> = sqlx::query_as(
        r#"SELECT id, content FROM posts
           WHERE reading_time_minutes = 0 AND content IS NOT NULL"#,
    )
    .fetch_all(pool)
    .await?;

    for (id, content) in rows {
        let minutes = crate::helper::reading_time::estimate_reading_time_minutes(&content);
        let updated = sqlx::query(
            "UPDATE posts SET reading_time_minutes = ? WHERE id = ? AND reading_time_minutes = 0",
        )
        .bind(minutes)
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();
        debug_assert!(updated <= 1);
    }

    Ok(())
}
