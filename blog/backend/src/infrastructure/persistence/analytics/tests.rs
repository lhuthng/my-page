use sqlx::SqlitePool;

use super::AnalyticsServiceImpl;

#[tokio::test]
async fn repeated_visits_increment_existing_aggregate_row() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query(
        r#"
        CREATE TABLE visitor_country_stats (
            day TEXT NOT NULL,
            country_code TEXT NOT NULL,
            path_group TEXT NOT NULL,
            visit_count INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (day, country_code, path_group)
        )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let service = AnalyticsServiceImpl::new(pool.clone());
    service
        .record_country_visit("2026-07-09", "DE", "posts")
        .await
        .unwrap();
    service
        .record_country_visit("2026-07-09", "DE", "posts")
        .await
        .unwrap();

    let count: i64 = sqlx::query_scalar(
        "SELECT visit_count FROM visitor_country_stats WHERE day = ? AND country_code = ? AND path_group = ?",
    )
    .bind("2026-07-09")
    .bind("DE")
    .bind("posts")
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(count, 2);
}
