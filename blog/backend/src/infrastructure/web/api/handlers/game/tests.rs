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
