// Database stats and dashboard overview.
use std::collections::HashMap;

use async_graphql::{Context, Object};
use sqlx::SqlitePool;

use super::super::helpers::{DASHBOARD_POST_COLUMNS, DASHBOARD_POST_JOINS, attach_tags_to_posts};
use super::super::rows::{DashboardPostRow, GrowthDayRow, RoleCountRow, UserInfoRow};
use super::super::types::{
    DbStats, GqlDashboardOverview, GqlDashboardUser, GqlGrowthPoint, GqlRoleCounts,
};

#[derive(Default)]
pub struct StatsQuery;

#[Object]
impl StatsQuery {
    async fn db_stats(&self, ctx: &Context<'_>) -> async_graphql::Result<DbStats> {
        let pool = ctx.data::<SqlitePool>()?;

        let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let total_posts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts")
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let total_comments: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM comments WHERE is_deleted = 0")
                .fetch_one(pool)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let total_media: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM media")
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let total_series: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM series")
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let total_tags: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tags")
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let total_categories: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM categories")
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(DbStats {
            total_users,
            total_posts,
            total_comments,
            total_media,
            total_series,
            total_tags,
            total_categories,
        })
    }

    async fn overview(&self, ctx: &Context<'_>) -> async_graphql::Result<GqlDashboardOverview> {
        let pool = ctx.data::<SqlitePool>()?;

        let total_published: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM posts WHERE status = 'published' AND content_kind = 'post'",
        )
        .fetch_one(pool)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let total_drafts: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM posts WHERE status = 'draft' AND content_kind = 'post'",
        )
        .fetch_one(pool)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let total_comments: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM comments WHERE is_deleted = 0")
                .fetch_one(pool)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let top_sql = |order: &str| -> String {
            format!(
                r#"SELECT {} {} WHERE p.content_kind = 'post' AND p.status = 'published' ORDER BY ps.{} DESC LIMIT 5"#,
                DASHBOARD_POST_COLUMNS, DASHBOARD_POST_JOINS, order
            )
        };

        let top_by_views = attach_tags_to_posts(
            pool,
            sqlx::query_as::<_, DashboardPostRow>(&top_sql("views"))
                .fetch_all(pool)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?,
        )
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let top_by_likes = attach_tags_to_posts(
            pool,
            sqlx::query_as::<_, DashboardPostRow>(&top_sql("likes"))
                .fetch_all(pool)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?,
        )
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let top_by_comments = attach_tags_to_posts(
            pool,
            sqlx::query_as::<_, DashboardPostRow>(&top_sql("comments_count"))
                .fetch_all(pool)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?,
        )
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let recent_sql = format!(
            r#"SELECT {} {} WHERE p.content_kind = 'post' ORDER BY p.created_at DESC LIMIT 5"#,
            DASHBOARD_POST_COLUMNS, DASHBOARD_POST_JOINS
        );
        let recent_posts = attach_tags_to_posts(
            pool,
            sqlx::query_as::<_, DashboardPostRow>(&recent_sql)
                .fetch_all(pool)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?,
        )
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let recent_users: Vec<GqlDashboardUser> = sqlx::query_as::<_, UserInfoRow>(
            r#"
            SELECT u.username, um.display_name, u.role, 'media/i/' || m.short_name AS avatar_url, u.created_at
            FROM users u
            JOIN user_meta um ON um.user_id = u.id
            LEFT JOIN media m ON m.id = um.avatar_image_id
            ORDER BY u.created_at DESC
            LIMIT 5
            "#,
        )
        .fetch_all(pool).await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?
        .into_iter()
        .map(|r| GqlDashboardUser {
            username: r.username,
            display_name: r.display_name,
            role: r.role,
            avatar_url: r.avatar_url,
            created_at: r.created_at,
        })
        .collect();

        let role_rows = sqlx::query_as::<_, RoleCountRow>(
            "SELECT role, COUNT(*) AS count FROM users GROUP BY role",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let mut role_counts = GqlRoleCounts {
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

        let post_growth = sqlx::query_as::<_, GrowthDayRow>(
            r#"
            SELECT date(created_at) AS date, COUNT(*) AS count
            FROM posts
            WHERE content_kind = 'post' AND date(created_at) >= date('now', '-30 days')
            GROUP BY date(created_at)
            ORDER BY date ASC
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let user_growth = sqlx::query_as::<_, GrowthDayRow>(
            r#"
            SELECT date(created_at) AS date, COUNT(*) AS count
            FROM users
            WHERE date(created_at) >= date('now', '-30 days')
            GROUP BY date(created_at)
            ORDER BY date ASC
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let mut growth_map: HashMap<String, (i64, i64)> = HashMap::new();
        for r in &post_growth {
            growth_map.entry(r.date.clone()).or_insert((0, 0)).0 = r.count;
        }
        for r in &user_growth {
            growth_map.entry(r.date.clone()).or_insert((0, 0)).1 = r.count;
        }
        let mut growth: Vec<GqlGrowthPoint> = growth_map
            .into_iter()
            .map(|(date, (new_posts, new_users))| GqlGrowthPoint {
                date,
                new_posts,
                new_users,
            })
            .collect();
        growth.sort_by(|a, b| a.date.cmp(&b.date));

        Ok(GqlDashboardOverview {
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
