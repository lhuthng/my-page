// Server lifecycle: builder, migration run, background maintenance spawn,
// and the axum serve loop.
use std::io::{Error, ErrorKind};

use super::config::{media::MediaConfig, project_demo::ProjectDemoConfig, storage, AppConfig, AppState};
use super::maintenance::{
    backfill_reading_times, cleanup_orphaned_uploads, purge_expired_trash,
};
use crate::infrastructure::{persistence, web::api};

pub struct HTTPServer<'a> {
    addr: Option<&'a str>,
    port: Option<&'a str>,
    db_url: Option<&'a str>,
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
        let storage = ObjectStore::from_env(&project_demo_config.dir)
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
            media_config: MediaConfig::from_env(),
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
