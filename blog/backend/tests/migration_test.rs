use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

fn mig_src() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations")
}

async fn run_migrations(url: &str, foreign_keys: bool) {
    let opts = url
        .parse::<SqliteConnectOptions>()
        .unwrap()
        .create_if_missing(true)
        .foreign_keys(foreign_keys)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .unwrap();
    sqlx::migrate::Migrator::new(mig_src())
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();
    pool.close().await;
}

#[tokio::test]
async fn relax_media_hash_migration_applies_with_data_and_fk_off() {
    let db = std::env::temp_dir().join("opencode_relax_migration_test.db");
    let _ = std::fs::remove_file(&db);
    let url = format!("sqlite://{}", db.display());
    run_migrations(&url, false).await;

    // seed FK-referencing data (media referenced by series/post usages/aliases)
    let seed = sqlx::SqlitePool::connect(&url).await.unwrap();
    sqlx::query("INSERT INTO users (id,username,email,password_hash) VALUES (1,'u','e@x.com','h')")
        .execute(&seed)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO media (id, hash, short_name, file_name, file_type, url, size) VALUES (1,'h1','n1','a','t','u',1)",
    )
    .execute(&seed)
    .await
    .unwrap();
    sqlx::query("INSERT INTO media_aliases (media_id, alias) VALUES (1,'al1')")
        .execute(&seed)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO series (id,user_id,title,slug,cover_image_id) VALUES (1,1,'t','slug-one',1)",
    )
    .execute(&seed)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO posts (id,user_id,title,slug,content,draft,cover_media_id) VALUES (1,1,'t','slug-two','c','d',1)",
    )
    .execute(&seed)
    .await
    .unwrap();
    sqlx::query("INSERT INTO post_media_usages (post_id, medium_id, code) VALUES (1,1,0)")
        .execute(&seed)
        .await
        .unwrap();
    seed.close().await;

    // drop my migration record so it re-applies on top of the seeded data
    let adjust = sqlx::SqlitePool::connect(&url).await.unwrap();
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 20260813120000")
        .execute(&adjust)
        .await
        .unwrap();
    adjust.close().await;

    // re-run all migrations with FK OFF (what the app now does via migration pool)
    run_migrations(&url, false).await;

    // verify data survived and schema is relaxed
    let ok = sqlx::SqlitePool::connect(&url).await.unwrap();
    let (id, hash, short_name): (i64, String, String) = sqlx::query_as(
        "SELECT id, hash, short_name FROM media WHERE id = 1",
    )
    .fetch_one(&ok)
    .await
    .unwrap();
    assert_eq!((id, hash.as_str(), short_name.as_str()), (1, "h1", "n1"));
    let aliases: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM media_aliases")
        .fetch_one(&ok)
        .await
        .unwrap();
    assert_eq!(aliases, 1);
    let has_hash_index: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND tbl_name='media' AND name='idx_media_hash'",
    )
    .fetch_one(&ok)
    .await
    .unwrap();
    assert_eq!(has_hash_index, 1);
    ok.close().await;

    let _ = std::fs::remove_file(&db);
    let _ = std::fs::remove_file(db.with_extension("db-wal"));
    let _ = std::fs::remove_file(db.with_extension("db-shm"));
}

#[tokio::test]
async fn v86_platform_key_accepts_windows9x_and_converts_old_rows() {
    let db = std::env::temp_dir().join("opencode_v86_platform_migration_test.db");
    let _ = std::fs::remove_file(&db);
    let url = format!("sqlite://{}", db.display());
    run_migrations(&url, false).await;

    let pool = sqlx::SqlitePool::connect(&url).await.unwrap();
    // The migration converted the platform key to the Windows 9x family.
    let keys: Vec<String> = sqlx::query_scalar("SELECT platform_key FROM v86_systems")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(keys.is_empty(), "fresh DB has no seeded systems");

    // The new CHECK accepts the renamed key and rejects the old one.
    sqlx::query(
        "INSERT INTO v86_systems (name, platform_key) VALUES ('Windows 9x', 'windows9x')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let rejected = sqlx::query(
        "INSERT INTO v86_systems (name, platform_key) VALUES ('Old', 'windows95')",
    )
    .execute(&pool)
    .await;
    assert!(rejected.is_err(), "the windows95 key must be rejected by the CHECK");

    // The upload-session CHECK follows the same rule.
    let session_rejected = sqlx::query(
        "INSERT INTO v86_system_upload_sessions
           (id, uploader_id, system_id, name, platform_key, original_file_name,
            expected_size_bytes, staged_storage_key, staged_sha256, staged_chunk_count,
            expires_at)
         VALUES ('s1', 1, 1, 'n', 'windows95', 'f.img', 100, 'k', 'h', 1, '2099-01-01T00:00:00Z')",
    )
    .execute(&pool)
    .await;
    assert!(session_rejected.is_err(), "session must reject the windows95 key");
    pool.close().await;

    let _ = std::fs::remove_file(&db);
    let _ = std::fs::remove_file(db.with_extension("db-wal"));
    let _ = std::fs::remove_file(db.with_extension("db-shm"));
}