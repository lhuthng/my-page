// Dashboard overview: counts, growth, and top-content SQL.
use std::collections::HashMap;

use sqlx::Row;

use crate::application::commands::dashboard::GetOverviewCommand;
use crate::domain::errors::dashboard::DashboardError;

use super::rows::{GrowthDayRow, RoleCountRow};
use super::DashboardServiceImpl;

const TOP_POSTS_SQL: &str = r#"
    SELECT p.id AS post_id, p.title, p.slug, p.excerpt,
           u.username AS author_slug, um.display_name AS author_name,
           'media/i/' || m.short_name AS url, m.file_type AS cover_media_type, p.status,
           ps.views, ps.likes, ps.comments_count, p.reading_time_minutes
    FROM posts p
    JOIN users u ON u.id = p.user_id
    JOIN user_meta um ON um.user_id = p.user_id
    JOIN post_stats ps ON ps.post_id = p.id
    LEFT JOIN media m ON m.id = p.cover_media_id
    WHERE p.status = 'published' AND p.deleted_at IS NULL AND p.content_kind = 'post'
    ORDER BY ps.{ORDER_COL} DESC
    LIMIT 5
"#;


#[async_trait::async_trait]
impl DashboardService for DashboardServiceImpl {
    async fn get_overview(&self, _cmd: GetOverviewCommand) -> Result<DashboardOverview, UserError> {
        // --- Scalar counts ---
        let total_published: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM posts WHERE status = 'published' AND deleted_at IS NULL AND content_kind = 'post'",
        )
        .fetch_one(&self.pool)
        .await?;

        let total_drafts: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM posts WHERE status = 'draft' AND content_kind = 'post'",
        )
        .fetch_one(&self.pool)
        .await?;

        let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;

        let total_comments: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM comments WHERE is_deleted = 0")
                .fetch_one(&self.pool)
                .await?;

        // --- Top posts ---
        let top_views_rows =
            sqlx::query_as::<_, PostRow>(&TOP_POSTS_SQL.replace("{ORDER_COL}", "views"))
                .fetch_all(&self.pool)
                .await?;

        let top_likes_rows =
            sqlx::query_as::<_, PostRow>(&TOP_POSTS_SQL.replace("{ORDER_COL}", "likes"))
                .fetch_all(&self.pool)
                .await?;

        let top_comments_rows =
            sqlx::query_as::<_, PostRow>(&TOP_POSTS_SQL.replace("{ORDER_COL}", "comments_count"))
                .fetch_all(&self.pool)
                .await?;

        // --- Recent posts ---
        let recent_rows = sqlx::query_as::<_, PostRow>(
            r#"
            SELECT p.id AS post_id, p.title, p.slug, p.excerpt,
                   u.username AS author_slug, um.display_name AS author_name,
                   'media/i/' || m.short_name AS url, m.file_type AS cover_media_type, p.status,
                   ps.views, ps.likes, ps.comments_count, p.reading_time_minutes
            FROM posts p
            JOIN users u ON u.id = p.user_id
            JOIN user_meta um ON um.user_id = p.user_id
            JOIN post_stats ps ON ps.post_id = p.id
            LEFT JOIN media m ON m.id = p.cover_media_id
            WHERE p.content_kind = 'post'
            ORDER BY p.created_at DESC
            LIMIT 5
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let top_by_views = fetch_snapshots_with_tags(&self.pool, top_views_rows).await?;
        let top_by_likes = fetch_snapshots_with_tags(&self.pool, top_likes_rows).await?;
        let top_by_comments = fetch_snapshots_with_tags(&self.pool, top_comments_rows).await?;
        let recent_posts = fetch_snapshots_with_tags(&self.pool, recent_rows).await?;

        // --- Recent users ---
        let recent_users: Vec<DashboardUserInfo> = sqlx::query_as::<_, UserInfoRow>(
            r#"
            SELECT u.username, um.display_name, u.role, 'media/i/' || m.short_name AS avatar_url, u.created_at
            FROM users u
            JOIN user_meta um ON um.user_id = u.id
            LEFT JOIN media m ON m.id = um.avatar_image_id
            ORDER BY u.created_at DESC
            LIMIT 5
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|r| DashboardUserInfo {
            username: r.username,
            display_name: r.display_name,
            role: r.role,
            avatar_url: r.avatar_url,
            created_at: r.created_at,
        })
        .collect();

        // --- Role counts ---
        let role_rows = sqlx::query_as::<_, RoleCountRow>(
            "SELECT role, COUNT(*) AS count FROM users GROUP BY role",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut role_counts = RoleCounts {
            admin: 0,
            moderator: 0,
            user: 0,
        };
        for r in role_rows {
            match r.role.as_str() {
                "admin" => role_counts.admin = r.count,
                "moderator" => role_counts.moderator = r.count,
                "user" => role_counts.user = r.count,
                _ => {}
            }
        }

        // --- Growth: new posts per day (last 30 days) ---
        let post_growth = sqlx::query_as::<_, GrowthDayRow>(
            r#"
            SELECT date(created_at) AS date, COUNT(*) AS count
            FROM posts
            WHERE content_kind = 'post' AND date(created_at) >= date('now', '-30 days')
            GROUP BY date(created_at)
            ORDER BY date ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let user_growth = sqlx::query_as::<_, GrowthDayRow>(
            r#"
            SELECT date(created_at) AS date, COUNT(*) AS count
            FROM users
            WHERE date(created_at) >= date('now', '-30 days')
            GROUP BY date(created_at)
            ORDER BY date ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        // Merge growth by date: (new_posts, new_users)
        let mut growth_map: HashMap<String, (i64, i64)> = HashMap::new();
        for r in &post_growth {
            growth_map.entry(r.date.clone()).or_insert((0, 0)).0 = r.count;
        }
        for r in &user_growth {
            growth_map.entry(r.date.clone()).or_insert((0, 0)).1 = r.count;
        }
        let mut growth: Vec<GrowthPoint> = growth_map
            .into_iter()
            .map(|(date, (new_posts, new_users))| GrowthPoint {
                date,
                new_posts,
                new_users,
            })
            .collect();
        growth.sort_by(|a, b| a.date.cmp(&b.date));

        Ok(DashboardOverview {
            total_published,
            total_drafts,
            total_users,
            total_comments,
            top_posts_by_views: top_by_views,
            top_posts_by_likes: top_by_likes,
            top_posts_by_comments: top_by_comments,
            recent_posts,
            recent_users,
            role_counts,
            growth,
        })
    }

}
