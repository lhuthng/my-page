// Server lifecycle: builder, migration run, background maintenance spawn,
// and the axum serve loop.
use std::io::{Error, ErrorKind};

use super::config::{media::MediaConfig, project_demo::ProjectDemoConfig};
use super::maintenance::{backfill_reading_times, cleanup_orphaned_uploads, purge_expired_trash};
use super::state::{AppConfig, AppState};
use crate::infrastructure::{persistence, storage::ObjectStore, web::api};

pub struct HTTPServer<'a> {
    addr: Option<&'a str>,
    port: Option<&'a str>,
    db_url: Option<&'a str>,
}

impl<'a> HTTPServer<'a> {
    /// The port the server binds when `PORT` says nothing usable.
    ///
    /// This is the local-development port, not the container's: Docker runs the
    /// same binary with `PORT` set explicitly, because its port mapping and
    /// health check are written against the container's own port.
    pub const DEFAULT_PORT: &'static str = "5174";

    /// The port to bind, read from `PORT`.
    ///
    /// Only a real TCP port counts. Anything else falls back to
    /// [`Self::DEFAULT_PORT`] instead of failing the bind, because an unusable
    /// `PORT` is usually stray environment rather than intent — and one value
    /// is actively misleading: `PORT=0` tells the OS to pick any free port, so
    /// a shell or dev tool exporting it would silently move the server off the
    /// port the frontend's `API_URL` points at.
    pub fn port_from_env() -> String {
        match std::env::var("PORT") {
            Ok(raw) => {
                let port = Self::resolve_port(&raw);
                if port != raw {
                    println!("PORT=\"{}\" is not a usable TCP port; using {}", raw, port);
                }
                port
            }
            Err(_) => Self::DEFAULT_PORT.to_string(),
        }
    }

    /// The pure half of [`Self::port_from_env`], so the fallback rules can be
    /// tested without touching the process environment.
    fn resolve_port(raw: &str) -> String {
        match raw.trim().parse::<u16>() {
            Ok(port) if port > 0 => port.to_string(),
            _ => Self::DEFAULT_PORT.to_string(),
        }
    }

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
        let storage = ObjectStore::from_env(&project_demo_config.dir)
            .map_err(|message| Error::new(ErrorKind::InvalidData, message))?;
        // Fails startup on a half-configured audio bucket — AUDIO_BACKEND=r2
        // with an incomplete R2_* set or no R2_PUBLIC_URL — rather than at the
        // first upload.
        let media_config = MediaConfig::from_env()
            .map_err(|message| Error::new(ErrorKind::InvalidData, message))?;
        cleanup_orphaned_uploads(&pool, &storage, &project_demo_config.dir).await?;
        purge_expired_trash(&pool, &storage, &project_demo_config.dir).await?;
        // spawn periodic purge every hour
        let purge_pool = pool.clone();
        let purge_storage = storage.clone();
        let purge_dir = project_demo_config.dir.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600));
            loop {
                interval.tick().await;
                let _ = purge_expired_trash(&purge_pool, &purge_storage, &purge_dir).await;
            }
        });
        let graphql_schema = crate::infrastructure::web::graphql::build_schema(pool.clone());
        let state = std::sync::Arc::new(AppState {
            config: AppConfig::from_env(),
            media_config,
            project_demo_config,
            storage,
            analytics_service: persistence::analytics::AnalyticsServiceImpl::new(pool.clone()),
            auth_service: persistence::auth::AuthServiceImpl::new(pool.clone()),
            user_service: persistence::user::UserServiceImpl::new(pool.clone()),
            media_service: persistence::media::MediaServiceImpl::new(pool.clone()),
            post_service: persistence::post::PostServiceImpl::new(pool.clone()),
            project_service: persistence::project::ProjectServiceImpl::new(pool.clone()),
            game_service: persistence::game::GameServiceImpl::new(pool.clone()),
            series_service: persistence::series::SeriesServiceImpl::new(pool.clone()),
            audiobook_service: persistence::audiobook::AudiobookServiceImpl::new(pool.clone()),
            dashboard_service: persistence::dashboard::DashboardServiceImpl::new(pool.clone()),
            newsletter_service: persistence::newsletter::NewsletterServiceImpl::new(pool.clone()),
            graphql_schema,
        });
        let router = api::router::build_router(state);

        let addr = self.addr.unwrap_or("127.0.0.1");
        let port = self.port.unwrap_or(Self::DEFAULT_PORT);

        let addr = format!("{}:{}", addr, port);
        println!("Starting {}", addr);
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, router).await.unwrap();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_port_accepts_real_ports() {
        assert_eq!(HTTPServer::resolve_port("5174"), "5174");
        assert_eq!(HTTPServer::resolve_port(" 6200 "), "6200");
        assert_eq!(HTTPServer::resolve_port("1"), "1");
        assert_eq!(HTTPServer::resolve_port("65535"), "65535");
    }

    #[test]
    fn resolve_port_falls_back_on_unusable_values() {
        // `0` would make the OS pick a random free port — never what a
        // `.env`-driven dev setup means by port zero.
        assert_eq!(HTTPServer::resolve_port("0"), HTTPServer::DEFAULT_PORT);
        assert_eq!(HTTPServer::resolve_port(""), HTTPServer::DEFAULT_PORT);
        assert_eq!(HTTPServer::resolve_port("   "), HTTPServer::DEFAULT_PORT);
        assert_eq!(
            HTTPServer::resolve_port("http://localhost:5174"),
            HTTPServer::DEFAULT_PORT
        );
        assert_eq!(HTTPServer::resolve_port("70000"), HTTPServer::DEFAULT_PORT);
        assert_eq!(HTTPServer::resolve_port("-1"), HTTPServer::DEFAULT_PORT);
        assert_eq!(HTTPServer::resolve_port("5174x"), HTTPServer::DEFAULT_PORT);
    }
}

impl<'a> Default for HTTPServer<'a> {
    fn default() -> Self {
        Self::new()
    }
}
