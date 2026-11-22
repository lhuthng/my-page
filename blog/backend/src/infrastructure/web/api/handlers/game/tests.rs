use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

use super::update::repoint_game_system;
use crate::domain::errors::game::GameError;

/// Migrated temp pool. FK checks stay off (as in the app's own migration
/// run) so the fixture can seed only the three tables under test without
/// users/posts parents.
async fn test_pool(label: &str) -> sqlx::SqlitePool {
    let db = std::env::temp_dir().join(format!("{label}-{}.db", uuid::Uuid::new_v4()));
    let _ = std::fs::remove_file(&db);
    let opts = format!("sqlite://{}", db.display())
        .parse::<SqliteConnectOptions>()
        .unwrap()
        .create_if_missing(true)
        .foreign_keys(false)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .unwrap();
    sqlx::migrate::Migrator::new(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations"),
    )
    .await
    .unwrap()
    .run(&pool)
    .await
    .unwrap();
    pool
}

/// Systems 1-3 active (windows9x, windows9x, windowsxp), system 4 inactive
/// (windows9x); versions 11-14 point at them in the same order. Game 1
/// carries an artifact on version 11.
async fn seed(pool: &sqlx::SqlitePool) {
    for (id, name, platform, active) in [
        (1, "Windows 95", "windows9x", 1),
        (2, "Windows 98", "windows9x", 1),
        (3, "Windows XP", "windowsxp", 1),
        (4, "Windows ME", "windows9x", 0),
    ] {
        sqlx::query(
            "INSERT INTO v86_systems (id, name, platform_key, is_active, current_version)
             VALUES (?, ?, ?, ?, 1)",
        )
        .bind(id)
        .bind(name)
        .bind(platform)
        .bind(active)
        .execute(pool)
        .await
        .unwrap();
    }
    for (id, system_id, key) in [
        (11, 1, "img-11"),
        (12, 2, "img-12"),
        (13, 3, "img-13"),
        (14, 4, "img-14"),
    ] {
        sqlx::query(
            "INSERT INTO v86_system_versions
                 (id, system_id, version_number, original_file_name, storage_key,
                  size_bytes, sha256, chunk_size_bytes, chunk_count)
             VALUES (?, ?, 1, 'disk.img', ?, 100, 'sha', 262144, 1)",
        )
        .bind(id)
        .bind(system_id)
        .bind(key)
        .execute(pool)
        .await
        .unwrap();
    }
    sqlx::query(
        "INSERT INTO game_v86_games
             (game_id, system_version_id, manifest_text, manifest_sha256,
              original_file_name, zip_storage_key, zip_size_bytes, zip_sha256,
              iso_storage_key, iso_size_bytes, iso_sha256,
              chunk_size_bytes, chunk_count, artifact_revision)
         VALUES (1, 11, 'exe=a.exe', 'hash', 'game.zip', 'zip-1', 100, 'ziphash',
                 'iso-1', 100, 'isohash', 262144, 1, 1)",
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn artifact_row(pool: &sqlx::SqlitePool) -> (i64, i64) {
    sqlx::query_as::<_, (i64, i64)>(
        "SELECT system_version_id, artifact_revision FROM game_v86_games WHERE game_id = 1",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn repoints_within_the_same_platform_and_bumps_the_revision() {
    let pool = test_pool("game-repoint-happy").await;
    seed(&pool).await;

    repoint_game_system(&pool, 1, 12).await.unwrap();

    assert_eq!(artifact_row(&pool).await, (12, 2));
}

#[tokio::test]
async fn same_version_is_a_noop() {
    let pool = test_pool("game-repoint-noop").await;
    seed(&pool).await;

    repoint_game_system(&pool, 1, 11).await.unwrap();

    assert_eq!(artifact_row(&pool).await, (11, 1));
}

#[tokio::test]
async fn rejects_cross_platform_inactive_unknown_and_missing_artifact() {
    let pool = test_pool("game-repoint-rejects").await;
    seed(&pool).await;

    let err = repoint_game_system(&pool, 1, 13).await.unwrap_err(); // windowsxp
    assert!(matches!(err, GameError::InvalidDemo(_)), "{err:?}");
    let err = repoint_game_system(&pool, 1, 14).await.unwrap_err(); // inactive
    assert!(matches!(err, GameError::InvalidDemo(_)), "{err:?}");
    let err = repoint_game_system(&pool, 1, 99).await.unwrap_err(); // no version
    assert!(matches!(err, GameError::InvalidDemo(_)), "{err:?}");
    let err = repoint_game_system(&pool, 2, 12).await.unwrap_err(); // no artifact
    assert!(matches!(err, GameError::InvalidDemo(_)), "{err:?}");

    // Nothing above may have mutated the artifact.
    assert_eq!(artifact_row(&pool).await, (11, 1));
}

// --- invitations: the full-stack half -------------------------------------------------------
//
// These drive the real game router with `oneshot`, the way author/tests.rs
// does, because the interesting behavior only exists across the layers: the
// optional-auth tier, the cookie jar, the atomic redemption, and the gate in
// front of the payload files.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode, header::SET_COOKIE},
};
use tower::ServiceExt;

use crate::domain::entities::secret::Claims;
use crate::infrastructure::game_invites::{
    generate_invitation_code, hash_invitation_code, normalize_invitation_code,
};
use crate::infrastructure::persistence;
use crate::infrastructure::storage::{ObjectStore, fs::FsStore};
use crate::infrastructure::web::api::handlers::game::routes;
use crate::infrastructure::web::api::security::RateLimiter;
use crate::infrastructure::web::graphql::build_schema;
use crate::infrastructure::web::server::{
    AppConfig, AppState, DatabaseSource, MediaConfig, ProjectDemoConfig, SecurityConfig,
};

fn invite_state(pool: sqlx::SqlitePool, dir: std::path::PathBuf) -> Arc<AppState> {
    let secret = b"test-secret";
    Arc::new(AppState {
        config: AppConfig {
            auth: crate::domain::entities::auth::AuthConfig {
                algorithm: jsonwebtoken::Algorithm::HS256,
                header: jsonwebtoken::Header::default(),
                encoding_key: jsonwebtoken::EncodingKey::from_secret(secret),
                decoding_key: jsonwebtoken::DecodingKey::from_secret(secret),
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
            allowed_file_types: vec![],
            allowed_avatar_types: vec![],
            allowed_cover_types: vec![],
            allowed_audio_types: vec![],
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
        storage: ObjectStore::Fs(FsStore::new(dir.clone())),
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

/// In-memory, fully migrated pool. One connection, so `:memory:` sticks.
async fn invite_pool() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(SqliteConnectOptions::new().filename(":memory:"))
        .await
        .unwrap();
    sqlx::migrate::Migrator::new(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations"),
    )
    .await
    .unwrap()
    .run(&pool)
    .await
    .unwrap();
    pool
}

/// User 1 authors game 1 (post 1, slug 'locked'); user 2 is a plain player.
/// `access_mode` and the launcher decide which serving routes apply.
async fn seed_game(pool: &sqlx::SqlitePool, access_mode: &str) {
    for id in 1..=2 {
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, role, email_verified_at)
             VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)",
        )
        .bind(id)
        .bind(format!("user{id}"))
        .bind(format!("user{id}@test.local"))
        .bind("x")
        .bind(if id == 1 { "moderator" } else { "user" })
        .execute(pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO user_meta (user_id, display_name) VALUES (?, ?)")
            .bind(id)
            .bind(format!("User {id}"))
            .execute(pool)
            .await
            .unwrap();
    }
    sqlx::query(
        "INSERT INTO posts (id, user_id, title, slug, excerpt, content, status, content_kind)
         VALUES (1, 1, 'Locked', 'locked', 'e', 'c', 'published', 'game')",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO games (id, post_id, launcher_type, access_mode) VALUES (1, 1, 'wasmshell', ?)",
    )
    .bind(access_mode)
    .execute(pool)
    .await
    .unwrap();
}

fn mint(user_id: i64, role: &str) -> String {
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &Claims {
            user_id: user_id.to_string(),
            role: role.to_string(),
            email_verified: true,
            exp: usize::MAX / 2,
        },
        &jsonwebtoken::EncodingKey::from_secret(b"test-secret"),
    )
    .unwrap()
}

async fn request(
    pool: sqlx::SqlitePool,
    dir: std::path::PathBuf,
    method: &str,
    uri: &str,
    headers: &[(String, String)],
    body: Option<String>,
) -> axum::response::Response {
    let state = invite_state(pool, dir);
    let app = routes(state.clone()).with_state(state);
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("host", "localhost");
    for (name, value) in headers {
        builder = builder.header(name.as_str(), value.as_str());
    }
    app.oneshot(
        builder
            .body(match body {
                Some(text) => Body::from(text),
                None => Body::empty(),
            })
            .unwrap(),
    )
    .await
    .unwrap()
}

async fn redeem(
    pool: sqlx::SqlitePool,
    dir: std::path::PathBuf,
    code: &str,
    bearer: Option<String>,
    cookie: Option<String>,
) -> axum::response::Response {
    let mut headers = vec![("content-type".to_string(), "application/json".to_string())];
    if let Some(bearer) = bearer {
        headers.push(("authorization".to_string(), format!("Bearer {bearer}")));
    }
    if let Some(cookie) = cookie {
        headers.push(("cookie".to_string(), cookie));
    }
    request(
        pool,
        dir,
        "POST",
        "/s/locked/invitations/redeem",
        &headers,
        Some(format!("{{\"code\": \"{code}\"}}")),
    )
    .await
}

fn invite_cookie_header(response: &axum::response::Response) -> String {
    response
        .headers()
        .get(SET_COOKIE)
        .expect("redeem sets a cookie")
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

async fn insert_code(pool: &sqlx::SqlitePool, max_uses: i64) -> String {
    let code = generate_invitation_code();
    let normalized = normalize_invitation_code(&code).unwrap();
    sqlx::query(
        "INSERT INTO game_invitations (game_id, code_hash, code_hint, max_uses, uses_count, note, created_by)
         VALUES (1, ?, ?, ?, 0, 'e2e', 1)",
    )
    .bind(hash_invitation_code(&normalized))
    .bind(&code[15..])
    .bind(max_uses)
    .execute(pool)
    .await
    .unwrap();
    code
}

#[tokio::test]
async fn an_open_game_serves_its_payload_with_no_credentials() {
    let pool = invite_pool().await;
    seed_game(&pool, "open").await;
    let dir = std::env::temp_dir().join(format!("game-invite-open-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(&dir).await.unwrap();
    let tree = dir.join("payloads/1/bundletree");
    tokio::fs::create_dir_all(&tree).await.unwrap();
    tokio::fs::write(tree.join("index.html"), "<html>game</html>")
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO game_payload_bundles (game_id, kind, storage_key, zip_sha256, zip_size_bytes, manifest_json)
         VALUES (1, 'wasmshell', 'payloads/1/bundletree', 'sha', 10, '{}')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let response = request(pool, dir, "GET", "/s/locked/payload/index.html", &[], None).await;
    assert_eq!(response.status(), StatusCode::OK);
}

/// The editor's mouse-capture checkbox survives a PATCH and comes back on the
/// details response the editor and the game page both read.
#[tokio::test]
async fn capture_mouse_round_trips_through_the_editor() {
    let pool = invite_pool().await;
    seed_game(&pool, "open").await;
    let dir = std::env::temp_dir().join(format!("game-capture-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(&dir).await.unwrap();
    let bearer = format!("Bearer {}", mint(1, "moderator"));
    let headers = [
        ("authorization".to_string(), bearer),
        (
            "content-type".to_string(),
            "multipart/form-data; boundary=testboundary".to_string(),
        ),
    ];
    let patch = |capture_mouse: bool| {
        let json = format!(r#"{{"number_of_files":0,"capture_mouse":{capture_mouse}}}"#);
        format!(
            "--testboundary\r\nContent-Disposition: form-data; name=\"game_data\"\r\nContent-Type: application/json\r\n\r\n{json}\r\n--testboundary--\r\n"
        )
    };

    let response = request(
        pool.clone(),
        dir.clone(),
        "PATCH",
        "/id/1",
        &headers,
        Some(patch(true)),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = request(pool.clone(), dir.clone(), "GET", "/id/1", &headers, None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let details: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(details["capture_mouse"], true);

    let response = request(pool, dir, "PATCH", "/id/1", &headers, Some(patch(false))).await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn an_invited_game_hides_its_payload_from_strangers() {
    let pool = invite_pool().await;
    seed_game(&pool, "invited").await;
    let dir = std::env::temp_dir().join(format!("game-invite-stranger-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(&dir).await.unwrap();
    let tree = dir.join("payloads/1/bundletree");
    tokio::fs::create_dir_all(&tree).await.unwrap();
    tokio::fs::write(tree.join("index.html"), "<html>game</html>")
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO game_payload_bundles (game_id, kind, storage_key, zip_sha256, zip_size_bytes, manifest_json)
         VALUES (1, 'wasmshell', 'payloads/1/bundletree', 'sha', 10, '{}')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let response = request(
        pool.clone(),
        dir.clone(),
        "GET",
        "/s/locked/payload/index.html",
        &[],
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    // A plain player (registered, no grant) is still a stranger.
    let response = request(
        pool.clone(),
        dir.clone(),
        "GET",
        "/s/locked/payload/index.html",
        &[(
            "authorization".to_string(),
            format!("Bearer {}", mint(2, "user")),
        )],
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    // The author walks straight in.
    let response = request(
        pool,
        dir,
        "GET",
        "/s/locked/payload/index.html",
        &[(
            "authorization".to_string(),
            format!("Bearer {}", mint(1, "moderator")),
        )],
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn a_single_use_code_admits_exactly_one_browser() {
    let pool = invite_pool().await;
    seed_game(&pool, "invited").await;
    let dir = std::env::temp_dir().join(format!("game-invite-single-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(dir.join("payloads/1/bundletree"))
        .await
        .unwrap();
    tokio::fs::write(
        dir.join("payloads/1/bundletree/index.html"),
        "<html>g</html>",
    )
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO game_payload_bundles (game_id, kind, storage_key, zip_sha256, zip_size_bytes, manifest_json)
         VALUES (1, 'wasmshell', 'payloads/1/bundletree', 'sha', 10, '{}')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let code = insert_code(&pool, 1).await;

    // A wrong code is refused and spends nothing.
    let response = redeem(pool.clone(), dir.clone(), "2222-2222-2222-2222", None, None).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response = redeem(pool.clone(), dir.clone(), &code, None, None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = invite_cookie_header(&response);
    assert!(cookie.starts_with("game-invite-1="));

    // The winning browser plays through the cookie alone.
    let response = request(
        pool.clone(),
        dir.clone(),
        "GET",
        "/s/locked/payload/index.html",
        &[("cookie".to_string(), cookie.clone())],
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    // The code itself is spent: the second browser is refused, the cookie's
    // owner is not (their redemption is a no-op, not a second use).
    let response = redeem(pool.clone(), dir.clone(), &code, None, None).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    assert!(String::from_utf8_lossy(&body).contains("already been used"));
    let response = redeem(pool.clone(), dir.clone(), &code, None, Some(cookie)).await;
    assert_eq!(response.status(), StatusCode::OK);

    let uses: i64 = sqlx::query_scalar("SELECT uses_count FROM game_invitations WHERE game_id = 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(uses, 1);
}

#[tokio::test]
async fn an_unlimited_code_admits_again_and_again() {
    let pool = invite_pool().await;
    seed_game(&pool, "invited").await;
    let dir = std::env::temp_dir().join(format!("game-invite-any-{}", uuid::Uuid::new_v4()));
    let code = insert_code(&pool, 0).await;

    for _ in 0..3 {
        let response = redeem(pool.clone(), dir.clone(), &code, None, None).await;
        assert_eq!(response.status(), StatusCode::OK);
    }
}

#[tokio::test]
async fn a_registered_redeemer_gets_an_account_bound_grant() {
    let pool = invite_pool().await;
    seed_game(&pool, "invited").await;
    let dir = std::env::temp_dir().join(format!("game-invite-grant-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(dir.join("payloads/1/bundletree"))
        .await
        .unwrap();
    tokio::fs::write(
        dir.join("payloads/1/bundletree/index.html"),
        "<html>g</html>",
    )
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO game_payload_bundles (game_id, kind, storage_key, zip_sha256, zip_size_bytes, manifest_json)
         VALUES (1, 'wasmshell', 'payloads/1/bundletree', 'sha', 10, '{}')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let code = insert_code(&pool, 1).await;

    let response = redeem(
        pool.clone(),
        dir.clone(),
        &code,
        Some(mint(2, "user")),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    // The grant exists and carries the serving routes with no cookie at all -
    // on any device this account signs in from.
    let granted: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM game_entitlements WHERE game_id = 1 AND user_id = 2",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(granted, 1);
    let response = request(
        pool,
        dir,
        "GET",
        "/s/locked/payload/index.html",
        &[(
            "authorization".to_string(),
            format!("Bearer {}", mint(2, "user")),
        )],
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn revoking_the_code_takes_the_guests_access_away() {
    let pool = invite_pool().await;
    seed_game(&pool, "invited").await;
    let dir = std::env::temp_dir().join(format!("game-invite-revoke-{}", uuid::Uuid::new_v4()));
    let code = insert_code(&pool, 1).await;

    let response = redeem(pool.clone(), dir.clone(), &code, None, None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = invite_cookie_header(&response);

    sqlx::query("UPDATE game_invitations SET revoked_at = CURRENT_TIMESTAMP WHERE game_id = 1")
        .execute(&pool)
        .await
        .unwrap();

    // The token is still perfectly signed; the code behind it is gone.
    let response = request(
        pool,
        dir,
        "GET",
        "/s/locked/payload/index.html",
        &[("cookie".to_string(), cookie)],
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_author_manages_codes_and_nobody_else_does() {
    let pool = invite_pool().await;
    seed_game(&pool, "invited").await;
    let dir = std::env::temp_dir().join(format!("game-invite-manage-{}", uuid::Uuid::new_v4()));

    // A non-owner moderator sees Forbidden, not the codes. (Game authoring
    // sits behind the moderator gate, so a plain 'user' never even reaches
    // the ownership check - the layer answers 401 first.)
    let response = request(
        pool.clone(),
        dir.clone(),
        "GET",
        "/id/1/invitations",
        &[(
            "authorization".to_string(),
            format!("Bearer {}", mint(2, "moderator")),
        )],
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // The owner creates one: the raw code comes back exactly once, listed
    // responses carry only the hint.
    let response = request(
        pool.clone(),
        dir.clone(),
        "POST",
        "/id/1/invitations",
        &[
            (
                "authorization".to_string(),
                format!("Bearer {}", mint(1, "moderator")),
            ),
            ("content-type".to_string(), "application/json".to_string()),
        ],
        Some("{\"max_uses\": 2, \"note\": \"for a friend\"}".to_string()),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8_lossy(&body).to_string();
    assert!(text.contains("\"code\""));
    assert!(!text.contains("\"code_hash\""));

    let response = request(
        pool.clone(),
        dir.clone(),
        "GET",
        "/id/1/invitations",
        &[(
            "authorization".to_string(),
            format!("Bearer {}", mint(1, "moderator")),
        )],
        None,
    )
    .await;
    let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8_lossy(&body).to_string();
    assert!(text.contains("\"hint\""));
    assert!(text.contains("for a friend"));
    assert!(!text.contains("\"code\":"));

    // Revoke the one code; a second revoke is a silent no-op, and a foreign
    // invitation id does not exist.
    let response = request(
        pool.clone(),
        dir.clone(),
        "DELETE",
        "/id/1/invitations/1",
        &[(
            "authorization".to_string(),
            format!("Bearer {}", mint(1, "moderator")),
        )],
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let response = request(
        pool.clone(),
        dir,
        "DELETE",
        "/id/1/invitations/1",
        &[(
            "authorization".to_string(),
            format!("Bearer {}", mint(1, "moderator")),
        )],
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let response = request(
        invite_pool().await,
        std::env::temp_dir(),
        "DELETE",
        "/id/1/invitations/99",
        &[(
            "authorization".to_string(),
            format!("Bearer {}", mint(1, "moderator")),
        )],
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
