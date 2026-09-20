// One-shot reading-time backfill, run at boot.
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
