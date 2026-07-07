use std::collections::HashMap;

use sqlx::{FromRow, SqlitePool};

use crate::{
    application::{commands::dashboard::*, services::dashboard::DashboardService},
    domain::{
        entities::{
            dashboard::*,
            post::{PostSnapshot, PostStats},
            project::ProjectSnapshot,
        },
        errors::user::UserError,
    },
    infrastructure::persistence::post::PostRow,
};

pub struct DashboardServiceImpl {
    pub pool: SqlitePool,
}

impl DashboardServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct RoleCountRow {
    role: String,
    count: i64,
}

#[derive(FromRow)]
struct GrowthDayRow {
    date: String,
    count: i64,
}

#[derive(FromRow)]
struct UserInfoRow {
    username: String,
    display_name: String,
    role: String,
    avatar_url: Option<String>,
    created_at: String,
}

#[derive(FromRow)]
struct DashTagRow {
    post_id: i64,
    tag_name: String,
    tag_slug: String,
}

#[derive(FromRow)]
struct DashProjectRow {
    project_id: i64,
    post_id: i64,
    title: String,
    slug: String,
    excerpt: String,
    author_name: String,
    author_slug: String,
    status: String,
    url: Option<String>,
    cover_media_type: Option<String>,
    demo_type: String,
    views: i64,
    likes: i64,
    comments_count: i64,
}

#[derive(FromRow)]
struct DashboardTagRecordRow {
    id: i64,
    name: String,
    slug: String,
    description: Option<String>,
    post_count: i64,
}

impl DashProjectRow {
    fn into_snapshot(self, tag_names: Vec<String>, tag_slugs: Vec<String>) -> ProjectSnapshot {
        ProjectSnapshot {
            id: self.project_id,
            post_id: self.post_id,
            title: self.title,
            slug: self.slug,
            tag_names,
            tag_slugs,
            excerpt: self.excerpt,
            author_name: self.author_name,
            author_slug: self.author_slug,
            status: self.status,
            url: self.url,
            cover_media_type: self.cover_media_type,
            demo_type: self.demo_type,
            stats: PostStats {
                views: self.views,
                likes: self.likes,
                comments: self.comments_count,
            },
        }
    }
}

async fn fetch_project_snapshots_with_tags(
    pool: &SqlitePool,
    rows: Vec<DashProjectRow>,
) -> Result<Vec<ProjectSnapshot>, UserError> {
    if rows.is_empty() {
        return Ok(vec![]);
    }

    let placeholders = rows.iter().map(|_| "?").collect::<Vec<_>>().join(", ");

    let sql = format!(
        r#"
        SELECT pj.id AS post_id, tags.name AS tag_name, tags.slug AS tag_slug
        FROM projects pj
        JOIN post_tags ON post_tags.post_id = pj.post_id
        JOIN tags ON tags.id = post_tags.tag_id
        WHERE pj.id IN ({})
        "#,
        placeholders
    );

    let mut query = sqlx::query_as::<_, DashTagRow>(&sql);
    let mut map: HashMap<i64, usize> = HashMap::new();
    let mut snapshots = vec![];

    for row in rows {
        map.insert(row.project_id, snapshots.len());
        query = query.bind(row.project_id);
        snapshots.push(row.into_snapshot(vec![], vec![]));
    }

    let tag_rows = query.fetch_all(pool).await?;

    for tag in tag_rows {
        if let Some(&idx) = map.get(&tag.post_id)
            && let Some(project) = snapshots.get_mut(idx)
        {
            project.tag_names.push(tag.tag_name);
            project.tag_slugs.push(tag.tag_slug);
        }
    }

    Ok(snapshots)
}

async fn fetch_snapshots_with_tags(
    pool: &SqlitePool,
    post_rows: Vec<PostRow>,
) -> Result<Vec<PostSnapshot>, UserError> {
    if post_rows.is_empty() {
        return Ok(vec![]);
    }

    let placeholders = post_rows.iter().map(|_| "?").collect::<Vec<_>>().join(", ");

    let sql = format!(
        r#"
        SELECT post_tags.post_id, tags.name AS tag_name, tags.slug AS tag_slug
        FROM post_tags
        JOIN tags ON tags.id = post_tags.tag_id
        WHERE post_tags.post_id IN ({})
        "#,
        placeholders
    );

    let mut query = sqlx::query_as::<_, DashTagRow>(&sql);
    let mut posts_map: HashMap<i64, usize> = HashMap::new();
    let mut snapshots: Vec<PostSnapshot> = vec![];

    for row in post_rows {
        posts_map.insert(row.post_id, snapshots.len());
        query = query.bind(row.post_id);
        snapshots.push(row.into_snapshot(vec![], vec![]));
    }

    let tag_rows = query.fetch_all(pool).await?;

    for tag in tag_rows {
        if let Some(&idx) = posts_map.get(&tag.post_id)
            && let Some(post) = snapshots.get_mut(idx)
        {
            post.tag_names.push(tag.tag_name);
            post.tag_slugs.push(tag.tag_slug);
        }
    }

    Ok(snapshots)
}

const TOP_POSTS_SQL: &str = r#"
    SELECT p.id AS post_id, p.title, p.slug, p.excerpt,
           u.username AS author_slug, um.display_name AS author_name,
           'media/i/' || m.short_name AS url, m.file_type AS cover_media_type, p.status,
           ps.views, ps.likes, ps.comments_count
    FROM posts p
    JOIN users u ON u.id = p.user_id
    JOIN user_meta um ON um.user_id = p.user_id
    JOIN post_stats ps ON ps.post_id = p.id
    LEFT JOIN media m ON m.id = p.cover_media_id
    WHERE p.status = 'published' AND p.content_kind = 'post'
    ORDER BY ps.{ORDER_COL} DESC
    LIMIT 5
"#;

#[async_trait::async_trait]
impl DashboardService for DashboardServiceImpl {
    async fn get_overview(&self, _cmd: GetOverviewCommand) -> Result<DashboardOverview, UserError> {
        // --- Scalar counts ---
        let total_published: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM posts WHERE status = 'published' AND content_kind = 'post'",
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
                   ps.views, ps.likes, ps.comments_count
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

    async fn get_posts(
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
                   ps.views, ps.likes, ps.comments_count
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

    async fn get_users(
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
        if let Some(ref rf) = cmd.role_filter {
            if is_admin {
                count_query = count_query.bind(rf);
            }
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
        if let Some(ref rf) = cmd.role_filter {
            if is_admin {
                data_query = data_query.bind(rf);
            }
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

    async fn get_projects(
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
                post_stats.comments_count
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

    async fn update_tag(
        &self,
        cmd: UpdateDashboardTagCommand,
    ) -> Result<DashboardTagRecord, UserError> {
        let name = cmd.name.trim().to_string();
        if name.len() < 2 {
            return Err(UserError::InvalidData(
                "Tag name must be at least 2 characters.".to_string(),
            ));
        }

        let slug = cmd.slug.trim().to_lowercase();
        if slug.len() < 2 {
            return Err(UserError::InvalidData(
                "Tag slug must be at least 2 characters.".to_string(),
            ));
        }
        if !slug
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_')
        {
            return Err(UserError::InvalidData(
                "Tag slug may only contain lowercase letters, numbers, hyphens, and underscores."
                    .to_string(),
            ));
        }

        let description = cmd
            .description
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let duplicate: Option<i64> =
            sqlx::query_scalar("SELECT id FROM tags WHERE slug = ? AND id != ?")
                .bind(&slug)
                .bind(cmd.id)
                .fetch_optional(&self.pool)
                .await?;

        if duplicate.is_some() {
            return Err(UserError::InvalidData(
                "Another tag already uses that slug.".to_string(),
            ));
        }

        let updated = sqlx::query(
            r#"
            UPDATE tags
            SET name = ?, slug = ?, description = ?
            WHERE id = ?
            "#,
        )
        .bind(&name)
        .bind(&slug)
        .bind(&description)
        .bind(cmd.id)
        .execute(&self.pool)
        .await?;

        if updated.rows_affected() == 0 {
            return Err(UserError::NotFound);
        }

        let row = sqlx::query_as::<_, DashboardTagRecordRow>(
            r#"
            SELECT t.id, t.name, t.slug, t.description,
                   COUNT(pt.post_id) AS post_count
            FROM tags t
            LEFT JOIN post_tags pt ON pt.tag_id = t.id
            WHERE t.id = ?
            GROUP BY t.id
            "#,
        )
        .bind(cmd.id)
        .fetch_one(&self.pool)
        .await?;

        Ok(DashboardTagRecord {
            id: row.id,
            name: row.name,
            slug: row.slug,
            description: row.description,
            post_count: row.post_count,
        })
    }

    async fn delete_tag(&self, cmd: DeleteDashboardTagCommand) -> Result<(), UserError> {
        let usage_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM post_tags WHERE tag_id = ?")
            .bind(cmd.id)
            .fetch_one(&self.pool)
            .await?;

        if usage_count > 0 {
            return Err(UserError::InvalidData(
                "Only unused tags can be deleted.".to_string(),
            ));
        }

        let deleted = sqlx::query("DELETE FROM tags WHERE id = ?")
            .bind(cmd.id)
            .execute(&self.pool)
            .await?;

        if deleted.rows_affected() == 0 {
            return Err(UserError::NotFound);
        }

        Ok(())
    }
}
