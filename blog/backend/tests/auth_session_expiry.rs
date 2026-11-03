//! Session lifetime for plain versus remember-me logins.
//!
//! Runs the real migrations and the real `AuthServiceImpl` against a throwaway
//! SQLite file, so the SQL that stores `expires_at` is covered too.

use std::path::PathBuf;

use backend::{
    application::{commands::auth::LoginCommand, services::auth::AuthService},
    domain::{entities::auth::AuthConfig, errors::auth::AuthError},
    infrastructure::persistence::auth::AuthServiceImpl,
};
use chrono::{DateTime, Utc};
use jsonwebtoken::Algorithm;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

fn auth_config() -> AuthConfig {
    AuthConfig::new(
        Algorithm::HS256,
        b"test-secret-that-is-long-enough-32".to_vec(),
        24,
        360,
        2160,
    )
}

async fn fixture() -> (sqlx::SqlitePool, AuthServiceImpl, PathBuf) {
    let path =
        std::env::temp_dir().join(format!("auth_session_expiry_{}.db", uuid::Uuid::new_v4()));

    let options = format!("sqlite://{}", path.display())
        .parse::<SqliteConnectOptions>()
        .unwrap()
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();

    let migrations = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations");
    sqlx::migrate::Migrator::new(migrations)
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();

    let password_hash = bcrypt::hash("verify-pass-123", 4).unwrap();
    sqlx::query(
        r#"
        INSERT INTO users (id, username, password_hash, email, role, email_verified_at)
        VALUES (1, 'verify-user', ?, 'verify@example.com', 'user', ?)
        "#,
    )
    .bind(password_hash)
    .bind(Utc::now().to_rfc3339())
    .execute(&pool)
    .await
    .unwrap();

    (pool.clone(), AuthServiceImpl::new(pool), path)
}

async fn session_hours(pool: &sqlx::SqlitePool) -> i64 {
    let expires_at: String =
        sqlx::query_scalar("SELECT expires_at FROM sessions WHERE user_id = 1")
            .fetch_one(pool)
            .await
            .unwrap();
    let expires_at = DateTime::parse_from_rfc3339(&expires_at).unwrap();
    (expires_at.with_timezone(&Utc) - Utc::now()).num_hours()
}

fn command(remember: bool) -> LoginCommand {
    LoginCommand {
        username: "verify-user".to_string(),
        password: "verify-pass-123".to_string(),
        remember,
    }
}

#[tokio::test]
async fn remember_me_extends_the_server_side_session() {
    let (pool, service, path) = fixture().await;

    assert!(service.login(command(false), auth_config()).await.is_ok());
    assert_eq!(session_hours(&pool).await, 359);

    assert!(service.login(command(true), auth_config()).await.is_ok());
    assert_eq!(session_hours(&pool).await, 2159);

    pool.close().await;
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn unverified_accounts_cannot_start_a_session() {
    let (pool, service, path) = fixture().await;
    sqlx::query("UPDATE users SET email_verified_at = NULL WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();

    let result = service.login(command(false), auth_config()).await;

    assert!(matches!(
        result,
        Ok(backend::domain::entities::auth::LoginResult::VerificationRequired { .. })
    ));

    pool.close().await;
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn wrong_password_is_rejected() {
    let (pool, service, path) = fixture().await;

    let result = service
        .login(
            LoginCommand {
                username: "verify-user".to_string(),
                password: "not-the-password".to_string(),
                remember: false,
            },
            auth_config(),
        )
        .await;

    assert!(matches!(result, Err(AuthError::InvalidCredentials)));

    pool.close().await;
    let _ = std::fs::remove_file(path);
}
