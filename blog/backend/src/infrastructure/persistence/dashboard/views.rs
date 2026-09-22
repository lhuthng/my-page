// Listing views: posts, users, and projects (with shared snapshot
// hydration).

use crate::application::commands::dashboard::{
    GetDashboardPostsCommand, GetDashboardProjectsCommand, GetDashboardUsersCommand,
};
use crate::domain::entities::dashboard::{
    DashboardPostsResult, DashboardProjectsResult, DashboardUserInfo, DashboardUsersResult,
    RoleCounts,
};
use crate::domain::errors::user::UserError;

use crate::infrastructure::persistence::post::PostRow;

use super::DashboardServiceImpl;
use super::rows::{DashProjectRow, RoleCountRow, UserInfoRow};
use super::shared::{fetch_project_snapshots_with_tags, fetch_snapshots_with_tags};

impl DashboardServiceImpl {
    pub(super) async fn get_posts(
        &self,
        cmd: GetDashboardPostsCommand,
    ) -> Result<DashboardPostsResult, UserError> {
        let is_admin = cmd.role == "admin";

        let mut where_parts: Vec<String> = vec!["p.content_kind = 'post'".to_string()];

        if !is_admin {
            where_parts.push(format!("p.user_id = {}", cmd.user_id));
        }

        if cmd.search.is_some() {
            where_parts.push(
                "(LOWER(p.title) LIKE '%' || LOWER(?) || '%' OR LOWER(p.slug) LIKE '%' || LOWER(?) || '%')".to_string()
            );
        }

        let where_clause = where_parts.join(" AND ");

        // Total count
        let count_sql = format!("SELECT COUNT(*) FROM posts p WHERE {}", where_clause);
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
        if let Some(ref s) = cmd.search {
            count_query = count_query.bind(s).bind(s);
        }
        let total: i64 = count_query
            .fetch_one(&self.pool)
            .await
            .map_err(|e| UserError::InternalError(e.to_string()))?;

        // Data query
        let data_sql = format!(
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
            WHERE {}
            ORDER BY p.updated_at DESC
            LIMIT {} OFFSET {}
            "#,
            where_clause, cmd.limit, cmd.offset
        );
        let mut data_query = sqlx::query_as::<_, PostRow>(&data_sql);
        if let Some(ref s) = cmd.search {
            data_query = data_query.bind(s).bind(s);
        }
        let post_rows = data_query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| UserError::InternalError(e.to_string()))?;

        let posts = fetch_snapshots_with_tags(&self.pool, post_rows).await?;

        Ok(DashboardPostsResult { posts, total })
    }

    pub(super) async fn get_users(
        &self,
        cmd: GetDashboardUsersCommand,
    ) -> Result<DashboardUsersResult, UserError> {
        let is_admin = cmd.role == "admin";

        let mut where_parts: Vec<String> = vec!["1=1".to_string()];

        if !is_admin {
            where_parts.push("u.role = 'user'".to_string());
        } else if cmd.role_filter.is_some() {
            where_parts.push("u.role = ?".to_string());
        }

        if cmd.search.is_some() {
            where_parts.push(
                "(LOWER(um.display_name) LIKE '%' || LOWER(?) || '%' OR LOWER(u.username) LIKE '%' || LOWER(?) || '%')".to_string()
            );
        }

        let where_clause = where_parts.join(" AND ");

        // Role counts (restricted for moderators to only show 'user' counts)
        let role_count_where = if is_admin { "1=1" } else { "role = 'user'" };
        let role_rows = sqlx::query_as::<_, RoleCountRow>(&format!(
            "SELECT role, COUNT(*) AS count FROM users WHERE {} GROUP BY role",
            role_count_where
        ))
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

        // Total count
        let count_sql = format!(
            "SELECT COUNT(*) FROM users u JOIN user_meta um ON um.user_id = u.id WHERE {}",
            where_clause
        );
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
        if let Some(ref rf) = cmd.role_filter
            && is_admin
        {
            count_query = count_query.bind(rf);
        }
        if let Some(ref s) = cmd.search {
            count_query = count_query.bind(s).bind(s);
        }
        let total: i64 = count_query.fetch_one(&self.pool).await?;

        // Data query
        let data_sql = format!(
            r#"
            SELECT u.username, um.display_name, u.role, 'media/i/' || m.short_name AS avatar_url, u.created_at
            FROM users u
            JOIN user_meta um ON um.user_id = u.id
            LEFT JOIN media m ON m.id = um.avatar_image_id
            WHERE {}
            ORDER BY u.created_at DESC
            LIMIT {} OFFSET {}
            "#,
            where_clause, cmd.limit, cmd.offset
        );
        let mut data_query = sqlx::query_as::<_, UserInfoRow>(&data_sql);
        if let Some(ref rf) = cmd.role_filter
            && is_admin
        {
            data_query = data_query.bind(rf);
        }
        if let Some(ref s) = cmd.search {
            data_query = data_query.bind(s).bind(s);
        }
        let users: Vec<DashboardUserInfo> = data_query
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

        Ok(DashboardUsersResult {
            users,
            total,
            role_counts,
        })
    }

    pub(super) async fn get_projects(
        &self,
        cmd: GetDashboardProjectsCommand,
    ) -> Result<DashboardProjectsResult, UserError> {
        let is_admin = cmd.role == "admin";

        let mut where_parts: Vec<String> = vec![];
        if !is_admin {
            where_parts.push("posts.user_id = ".to_string() + &cmd.user_id.to_string());
        }

        if cmd.search.is_some() {
            where_parts.push(
                "(LOWER(posts.title) LIKE '%' || LOWER(?) || '%' OR LOWER(posts.slug) LIKE '%' || LOWER(?) || '%')".to_string()
            );
        }

        let where_clause = if where_parts.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_parts.join(" AND "))
        };

        let count_sql = format!(
            r#"
            SELECT COUNT(*)
            FROM projects
            JOIN posts ON posts.id = projects.post_id
            {}
            "#,
            where_clause
        );
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
        if let Some(ref s) = cmd.search {
            count_query = count_query.bind(s).bind(s);
        }
        let total: i64 = count_query
            .fetch_one(&self.pool)
            .await
            .map_err(|e| UserError::InternalError(e.to_string()))?;

        let data_sql = format!(
            r#"
            SELECT
                projects.id AS project_id,
                posts.id AS post_id,
                posts.title,
                posts.slug,
                posts.excerpt,
                users.username AS author_slug,
                user_meta.display_name AS author_name,
                posts.status,
                'media/i/' || media.short_name AS url,
                media.file_type AS cover_media_type,
                projects.demo_type,
                post_stats.views,
                post_stats.likes,
                post_stats.comments_count,
                posts.reading_time_minutes
            FROM projects
            JOIN posts ON posts.id = projects.post_id
            JOIN users ON users.id = posts.user_id
            JOIN user_meta ON user_meta.user_id = posts.user_id
            JOIN post_stats ON post_stats.post_id = posts.id
            LEFT JOIN media ON media.id = posts.cover_media_id
            {}
            ORDER BY posts.updated_at DESC
            LIMIT {} OFFSET {}
            "#,
            where_clause, cmd.limit, cmd.offset
        );
        let mut data_query = sqlx::query_as::<_, DashProjectRow>(&data_sql);
        if let Some(ref s) = cmd.search {
            data_query = data_query.bind(s).bind(s);
        }
        let rows = data_query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| UserError::InternalError(e.to_string()))?;

        let projects = fetch_project_snapshots_with_tags(&self.pool, rows).await?;

        Ok(DashboardProjectsResult { projects, total })
    }
}
