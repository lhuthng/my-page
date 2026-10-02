// The media read queries are built with `sqlx::query_as` over string SQL, so a
// column added to the SELECT and not to the tuple (or vice versa) is a *runtime*
// failure that `cargo check` cannot see. These run the real queries against a
// migrated database instead.
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

use crate::application::commands::media::{
    GetLinkCommand, GetMediaDetailsCommand, SearchMediaCommand,
};

use super::MediaServiceImpl;

/// A temp database with this crate's real migrations applied, so the schema
/// under test is the one that ships rather than one restated here.
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

/// Insert one media row with an explicit upload time, so the assertion is about
/// the value travelling through the query rather than about "now".
async fn seed_media(pool: &sqlx::SqlitePool, short_name: &str, created_at: &str) {
    sqlx::query(
        r#"
        INSERT INTO media (hash, short_name, file_name, file_type, url, size, description, uploader_id, created_at)
        VALUES (?, ?, ?, 'image/webp', './media/webp/1/x.webp', 10, '', 1, ?)
        "#,
    )
    .bind(format!("hash-{short_name}"))
    .bind(short_name)
    .bind(format!("{short_name}.webp"))
    .bind(created_at)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn search_reports_each_result_upload_time() {
    let pool = test_pool("media-search-created-at").await;
    seed_media(&pool, "banner.one", "2026-09-29 21:04:33").await;
    seed_media(&pool, "banner.two", "2026-09-30 08:15:00").await;

    let service = MediaServiceImpl::new(pool.clone());
    let results = service
        .search(SearchMediaCommand {
            term: Some("banner".to_string()),
            size: 10,
            skip: 0,
        })
        .await
        .unwrap();

    assert_eq!(results.len(), 2, "both seeded rows match the term");

    // The SQL orders by `created_at DESC`, so the newest upload comes first —
    // which is what makes an empty search read as "my most recent uploads".
    assert_eq!(results[0].short_name.as_deref(), Some("banner.two"));
    assert_eq!(
        results[0].created_at.as_deref(),
        Some("2026-09-30 08:15:00")
    );
    assert_eq!(
        results[1].created_at.as_deref(),
        Some("2026-09-29 21:04:33")
    );
}

#[tokio::test]
async fn an_empty_term_lists_the_most_recent_uploads() {
    let pool = test_pool("media-search-empty-term").await;
    seed_media(&pool, "older", "2026-09-01 00:00:00").await;
    seed_media(&pool, "newer", "2026-09-30 00:00:00").await;

    let service = MediaServiceImpl::new(pool.clone());
    let results = service
        .search(SearchMediaCommand {
            // An empty string is what the library sends when the box is empty.
            term: Some(String::new()),
            size: 10,
            skip: 0,
        })
        .await
        .unwrap();

    assert_eq!(
        results.len(),
        2,
        "an empty term must match everything, not nothing"
    );
    assert_eq!(
        results[0].short_name.as_deref(),
        Some("newer"),
        "the library opens on the most recent upload"
    );
}

#[tokio::test]
async fn details_and_link_both_carry_the_upload_time() {
    let pool = test_pool("media-details-created-at").await;
    seed_media(&pool, "album.art", "2026-09-29 21:04:33").await;

    let service = MediaServiceImpl::new(pool.clone());

    let details = service
        .get_details(GetMediaDetailsCommand {
            short_name: "album.art".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(details.created_at.as_deref(), Some("2026-09-29 21:04:33"));

    // `get_link` feeds the byte-serving path, where the field goes unused — but
    // it is selected on the same row shape, so it is asserted here too: a
    // mismatch there would be a runtime error on every media request.
    let link = service
        .get_link(GetLinkCommand {
            short_name: "album.art".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(link.created_at.as_deref(), Some("2026-09-29 21:04:33"));
}

#[tokio::test]
async fn a_row_with_no_upload_time_reads_as_none_rather_than_failing() {
    let pool = test_pool("media-null-created-at").await;
    // The column is `TEXT DEFAULT CURRENT_TIMESTAMP`, not `NOT NULL`, so a row
    // can genuinely have no value and the read path must tolerate it.
    sqlx::query(
        r#"
        INSERT INTO media (hash, short_name, file_name, file_type, url, size, description, uploader_id, created_at)
        VALUES ('h', 'undated', 'undated.png', 'image/png', './media/png/1/u.png', 1, '', 1, NULL)
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let service = MediaServiceImpl::new(pool.clone());
    let details = service
        .get_details(GetMediaDetailsCommand {
            short_name: "undated".to_string(),
        })
        .await
        .unwrap();

    assert_eq!(details.created_at, None);
}
