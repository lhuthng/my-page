//! Route reachability and the per-book gate.
//!
//! `handlers/sync.rs` shipped three key-management handlers that were never
//! mounted on any router. These tests drive the real router with `oneshot` and
//! assert that a missing route cannot masquerade as an auth failure: an
//! unauthenticated request to a mounted route must answer 401, not 404.
//!
//! The second half proves the switch is per book: a perfectly valid key is
//! refused while that one audiobook has `author_api_enabled = 0`, and works the
//! moment it is switched on.
use std::{path::PathBuf, sync::Arc};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header};
use sqlx::{
    SqlitePool,
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use tower::ServiceExt;

use crate::{
    domain::entities::{auth::AuthConfig, media::MediaType},
    infrastructure::{
        author_keys::{AUTHOR_KEY_PREFIX, hash_author_key},
        persistence,
        storage::{ObjectStore, fs::FsStore},
        web::{
            api::{handlers::author::routes, security::RateLimiter},
            graphql::build_schema,
            server::{
                AppConfig, AppState, DatabaseSource, MediaConfig, ProjectDemoConfig, SecurityConfig,
            },
        },
    },
};

const TEST_KEY: &str = "abk_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn test_state(pool: SqlitePool) -> Arc<AppState> {
    let dir = PathBuf::from("/tmp/author-api-tests");
    let secret = b"test-secret";
    Arc::new(AppState {
        config: AppConfig {
            auth: AuthConfig {
                algorithm: Algorithm::HS256,
                header: Header::default(),
                encoding_key: EncodingKey::from_secret(secret),
                decoding_key: DecodingKey::from_secret(secret),
                access_expire_hours: 24,
                refresh_expire_hours: 360,
                remember_expire_hours: 720,
            },
            security: SecurityConfig {
                turnstile_secret: None,
            },
            mail: None,
            app_base_url: "http://localhost:5000".to_string(),
            database_source: DatabaseSource::Sqlite {
                path: dir.join("blog.db"),
            },
            r2_public_url: None,
        },
        media_config: MediaConfig {
            dir: dir.clone(),
            allowed_file_types: vec![MediaType::AudioMp3],
            allowed_avatar_types: vec![],
            allowed_cover_types: vec![],
            allowed_audio_types: vec![MediaType::AudioMp3],
            audio_bucket: None,
        },
        project_demo_config: ProjectDemoConfig {
            dir: dir.clone(),
            max_archive_size: 100,
            max_extracted_size: 200,
            max_files: 10,
            max_jsdos_size: 100,
            jsdos_chunk_size: 10,
            upload_session_ttl_hours: 24,
            max_v86_base_size: 100,
            max_v86_game_extracted_size: 100,
            v86_upload_chunk_size: 10,
            v86_download_chunk_size: 10,
            v86_assets_dir: dir.clone(),
            max_payload_size: 100,
            max_payload_extracted_size: 200,
            payload_chunk_size: 10,
            max_payload_files: 20,
        },
        storage: ObjectStore::Fs(FsStore::new(dir)),
        http_client: reqwest::Client::new(),
        rate_limiter: RateLimiter::new(),
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
        graphql_schema: build_schema(pool),
    })
}

/// In-memory database with every migration applied. Single connection:
/// in-memory SQLite is per-connection.
async fn migrated_pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(SqliteConnectOptions::new().filename(":memory:"))
        .await
        .expect("in-memory pool");
    Migrator::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations"))
        .await
        .expect("migrations")
        .run(&pool)
        .await
        .expect("migrate");
    pool
}

async fn call_with(pool: SqlitePool, uri: &str, bearer: Option<&str>) -> (StatusCode, String) {
    let state = test_state(pool);
    let app = routes(state.clone()).with_state(state);
    let mut builder = Request::builder().uri(uri);
    if let Some(token) = bearer {
        builder = builder.header("Authorization", format!("Bearer {token}"));
    }
    let response = app
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .expect("router responds");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap_or_default();
    (status, String::from_utf8_lossy(&bytes).to_string())
}

async fn call(uri: &str) -> (StatusCode, String) {
    call_with(migrated_pool().await, uri, None).await
}

#[tokio::test]
async fn there_is_no_deployment_wide_switch() {
    // The old `/author/status` endpoint reported an env flag. Exposure is per
    // book now, so it must be gone rather than answering "enabled" globally.
    let (status, _) = call("/status").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn key_management_and_settings_routes_are_reachable() {
    // 401 from user_guard proves the route exists. The sync-key bug answered
    // 404 here because the handlers were never mounted.
    for uri in [
        "/audiobooks/id/1/keys",
        "/audiobooks/id/1/keys/7",
        "/audiobooks/id/1/settings",
    ] {
        let (status, _) = call(uri).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri} is not mounted");
    }
}

#[tokio::test]
async fn key_tier_routes_are_reachable() {
    // 401 from author_key_guard for a request with no Bearer token.
    for uri in [
        "/audiobooks/id/1",
        "/audiobooks/id/1/chapters/1/audio",
        "/audiobooks/id/1/archive",
    ] {
        let (status, _) = call(uri).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri} is not mounted");
    }
}

/// Seeds one moderator, one book (API off), one media row, one chapter and one
/// key.
async fn seed(pool: &SqlitePool) {
    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash, role)
         VALUES (1, 'alice', 'a@example.com', 'h', 'moderator')",
    )
    .execute(pool)
    .await
    .expect("user");
    sqlx::query(
        "INSERT INTO audiobooks (id, user_id, title, slug, status)
         VALUES (7, 1, 'Book', 'book', 'draft')",
    )
    .execute(pool)
    .await
    .expect("book");
    let hash = "b4c0ffee".repeat(8);
    sqlx::query(
        "INSERT INTO media (id, hash, short_name, file_name, file_type, url, size, uploader_id)
         VALUES (3, ?, 'abt-one', '01.mp3', 'audio/mpeg', '/tmp/01.mp3', 42, 1)",
    )
    .bind(&hash)
    .execute(pool)
    .await
    .expect("media");
    sqlx::query(
        "INSERT INTO audiobook_tracks (id, audiobook_id, media_id, title, number)
         VALUES (11, 7, 3, 'Chapter 1', 1)",
    )
    .execute(pool)
    .await
    .expect("track");
    sqlx::query(
        "INSERT INTO author_api_keys (audiobook_id, user_id, token_hash, label, hint, scopes)
         VALUES (7, 1, ?, 'test', 'abk_0123…', 'read,write')",
    )
    .bind(hash_author_key(TEST_KEY))
    .execute(pool)
    .await
    .expect("key");
}

#[tokio::test]
async fn the_switch_is_per_book_and_off_until_turned_on() {
    let pool = migrated_pool().await;
    seed(&pool).await;

    // A valid, unrevoked, unexpired key — and the book is off.
    let (status, _) = call_with(pool.clone(), "/audiobooks/id/7", Some(TEST_KEY)).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "a key must not reach a book whose API is off"
    );

    // Switch this one book on.
    sqlx::query("UPDATE audiobooks SET author_api_enabled = 1 WHERE id = 7")
        .execute(&pool)
        .await
        .expect("enable");

    let (status, body) = call_with(pool.clone(), "/audiobooks/id/7", Some(TEST_KEY)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body.contains("\"chapter_count\":1"), "{body}");
}

#[tokio::test]
async fn one_books_switch_does_not_expose_another() {
    let pool = migrated_pool().await;
    seed(&pool).await;
    sqlx::query(
        "INSERT INTO audiobooks (id, user_id, title, slug, status)
         VALUES (8, 1, 'Other', 'other', 'draft')",
    )
    .execute(&pool)
    .await
    .expect("second book");
    sqlx::query("UPDATE audiobooks SET author_api_enabled = 1 WHERE id = 7")
        .execute(&pool)
        .await
        .expect("enable first");

    // The key is bound to book 7; book 8 is off and a different id.
    let (status, _) = call_with(pool.clone(), "/audiobooks/id/8", Some(TEST_KEY)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn chapter_queries_run_against_a_real_schema() {
    use crate::infrastructure::web::api::handlers::author::shared::{
        count_chapters, load_chapter, load_chapters,
    };

    let pool = migrated_pool().await;
    seed(&pool).await;

    assert_eq!(count_chapters(&pool, 7).await.expect("count"), 1);
    let all = load_chapters(&pool, 7, 0, None).await.expect("all");
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].hash, "b4c0ffee".repeat(8));
    assert_eq!(all[0].file_type, "audio/mpeg");
    let one = load_chapter(&pool, 7, 1)
        .await
        .expect("one")
        .expect("present");
    assert_eq!(one.track_id, 11);
    assert!(load_chapter(&pool, 7, 99).await.expect("missing").is_none());
    assert!(
        load_chapters(&pool, 7, 5, None)
            .await
            .expect("offset")
            .is_empty()
    );
}

#[test]
fn the_test_key_looks_like_a_real_one() {
    // Guards the fixture above: the guard hashes whatever it is given, so a
    // malformed constant would still "work" and prove nothing about prefixes.
    assert!(TEST_KEY.starts_with(AUTHOR_KEY_PREFIX));
}
