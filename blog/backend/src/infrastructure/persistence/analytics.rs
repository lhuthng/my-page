use serde::Serialize;
use sqlx::{FromRow, SqlitePool};

#[derive(Clone)]
pub struct AnalyticsServiceImpl {
    pool: SqlitePool,
}

#[derive(Debug, FromRow, Serialize)]
pub struct VisitorCountryStat {
    pub date: String,
    pub country_code: String,
    pub visits: i64,
}

impl AnalyticsServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn record_country_visit(
        &self,
        day: &str,
        country_code: &str,
        path_group: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO visitor_country_stats (day, country_code, path_group, visit_count)
            VALUES (?, ?, ?, 1)
            ON CONFLICT(day, country_code, path_group)
            DO UPDATE SET visit_count = visit_count + 1
            "#,
        )
        .bind(day)
        .bind(country_code)
        .bind(path_group)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_country_stats(
        &self,
        days: i64,
    ) -> Result<Vec<VisitorCountryStat>, sqlx::Error> {
        sqlx::query_as::<_, VisitorCountryStat>(
            r#"
            SELECT
                day AS date,
                country_code,
                SUM(visit_count) AS visits
            FROM visitor_country_stats
            WHERE day >= date('now', '-' || ? || ' days')
            GROUP BY day, country_code
            ORDER BY day DESC, visits DESC, country_code ASC
            "#,
        )
        .bind(days)
        .fetch_all(&self.pool)
        .await
    }
}

#[cfg(test)]
mod tests {
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
}
