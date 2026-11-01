// Read notification retention: drop read rows after the retention window.

const READ_RETENTION_DAYS: i64 = 180;

/// Unread rows are kept forever: they are the record a returning user has not
/// seen yet. Only rows they already read age out.
pub async fn prune_read_notifications(
    pool: &sqlx::SqlitePool,
) -> Result<(), Box<dyn std::error::Error>> {
    let result = sqlx::query(
        r#"
        DELETE FROM notifications
        WHERE read_at IS NOT NULL
          AND read_at < datetime('now', ?)
        "#,
    )
    .bind(format!("-{READ_RETENTION_DAYS} days"))
    .execute(pool)
    .await?;

    if result.rows_affected() > 0 {
        println!("Pruned {} read notification(s)", result.rows_affected());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::prune_read_notifications;

    #[tokio::test]
    async fn prunes_only_read_rows_past_retention() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            r#"
            CREATE TABLE notifications (
                id INTEGER PRIMARY KEY,
                recipient_user_id INTEGER NOT NULL,
                read_at TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r#"
            INSERT INTO notifications (id, recipient_user_id, read_at) VALUES
                (1, 1, datetime('now', '-200 days')),
                (2, 1, datetime('now', '-10 days')),
                (3, 1, NULL)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        prune_read_notifications(&pool).await.unwrap();

        let ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM notifications ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(ids, vec![2, 3]);
    }
}
