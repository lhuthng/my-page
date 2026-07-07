use std::collections::HashMap;

use async_graphql::{Context, EmptyMutation, EmptySubscription, Object, Schema};
use sqlx::SqlitePool;

use super::helpers::{
    DASHBOARD_POST_COLUMNS, DASHBOARD_POST_JOINS, attach_tags_to_posts, attach_tags_to_projects,
};
use super::rows::CategoryRow;
use super::rows::{
    CommentRow, DashboardPostRow, DashboardProjectRow, GqlPostRow, GrowthDayRow, MediaRow,
    PostDetailRow, RoleCountRow, SeriesPostRow, SeriesRow, TagRow, UserInfoRow, UserRow,
};
use super::types::{
    CategoryConnection, CommentConnection, DashboardPostConnection, DbStats, GqlCategory,
    GqlComment, GqlDashboardOverview, GqlDashboardPost, GqlDashboardProject, GqlDashboardUser,
    GqlGrowthPoint, GqlMedia, GqlPost, GqlPostDetail, GqlRoleCounts, GqlSeries, GqlSeriesPost,
    GqlTag, GqlUser, MediaConnection, PostConnection, ProjectConnection, SeriesConnection,
    TagConnection, UserConnection,
};

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn users(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
        search: Option<String>,
        role: Option<String>,
    ) -> async_graphql::Result<UserConnection> {
        let pool = ctx.data::<SqlitePool>()?;
        let limit = limit.unwrap_or(20) as i64;
        let offset = offset.unwrap_or(0) as i64;

        let mut where_parts: Vec<String> = vec!["1=1".to_string()];

        if search.is_some() {
            where_parts.push(
                "(LOWER(u.username) LIKE '%' || LOWER(?) || '%' OR LOWER(um.display_name) LIKE '%' || LOWER(?) || '%')".to_string()
            );
        }

        if role.is_some() {
            where_parts.push("u.role = ?".to_string());
        }

        let where_clause = where_parts.join(" AND ");

        let count_sql = format!(
            "SELECT COUNT(*) FROM users u JOIN user_meta um ON um.user_id = u.id WHERE {}",
            where_clause
        );
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
        if let Some(ref s) = search {
            count_query = count_query.bind(s).bind(s);
        }
        if let Some(ref r) = role {
            count_query = count_query.bind(r);
        }
        let total: i64 = count_query
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let data_sql = format!(
            r#"
            SELECT u.id, u.username, u.email, u.role, u.created_at,
                   um.display_name, um.bio, m.url AS avatar_url
            FROM users u
            JOIN user_meta um ON um.user_id = u.id
            LEFT JOIN media m ON m.id = um.avatar_image_id
            WHERE {}
            ORDER BY u.created_at DESC
            LIMIT {} OFFSET {}
            "#,
            where_clause, limit, offset
        );
        let mut data_query = sqlx::query_as::<_, UserRow>(&data_sql);
        if let Some(ref s) = search {
            data_query = data_query.bind(s).bind(s);
        }
        if let Some(ref r) = role {
            data_query = data_query.bind(r);
        }
        let rows = data_query
            .fetch_all(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let items = rows
            .into_iter()
            .map(|r| GqlUser {
                id: r.id,
                username: r.username,
                email: r.email,
                role: r.role,
                display_name: r.display_name,
                bio: r.bio,
                avatar_url: r.avatar_url,
                created_at: r.created_at,
            })
            .collect();

        Ok(UserConnection { items, total })
    }

    async fn posts(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
        search: Option<String>,
        status: Option<String>,
    ) -> async_graphql::Result<PostConnection> {
        let pool = ctx.data::<SqlitePool>()?;
        let limit = limit.unwrap_or(20) as i64;
        let offset = offset.unwrap_or(0) as i64;

        let mut where_parts: Vec<String> = vec!["1=1".to_string()];

        if search.is_some() {
            where_parts.push(
                "(LOWER(p.title) LIKE '%' || LOWER(?) || '%' OR LOWER(p.slug) LIKE '%' || LOWER(?) || '%')".to_string()
            );
        }

        if status.is_some() {
            where_parts.push("p.status = ?".to_string());
        }

        let where_clause = where_parts.join(" AND ");

        let count_sql = format!("SELECT COUNT(*) FROM posts p WHERE {}", where_clause);
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
        if let Some(ref s) = search {
            count_query = count_query.bind(s).bind(s);
        }
        if let Some(ref st) = status {
            count_query = count_query.bind(st);
        }
        let total: i64 = count_query
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let data_sql = format!(
            r#"
            SELECT p.id, p.title, p.slug, p.status, p.view_count, p.is_featured,
                   p.published_at, p.created_at, p.updated_at, p.excerpt,
                   u.username AS author_slug, um.display_name AS author_name,
                   s.title AS series_title
            FROM posts p
            LEFT JOIN users u ON u.id = p.user_id
            LEFT JOIN user_meta um ON um.user_id = p.user_id
            LEFT JOIN series s ON s.id = p.series_id
            WHERE {}
            ORDER BY p.created_at DESC
            LIMIT {} OFFSET {}
            "#,
            where_clause, limit, offset
        );
        let mut data_query = sqlx::query_as::<_, GqlPostRow>(&data_sql);
        if let Some(ref s) = search {
            data_query = data_query.bind(s).bind(s);
        }
        if let Some(ref st) = status {
            data_query = data_query.bind(st);
        }
        let rows = data_query
            .fetch_all(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let items = rows
            .into_iter()
            .map(|r| GqlPost {
                id: r.id,
                title: r.title,
                slug: r.slug,
                status: r.status,
                author_name: r.author_name,
                author_slug: r.author_slug,
                series_title: r.series_title,
                view_count: r.view_count,
                is_featured: r.is_featured != 0,
                published_at: r.published_at,
                created_at: r.created_at,
                updated_at: r.updated_at,
                excerpt: r.excerpt,
            })
            .collect();

        Ok(PostConnection { items, total })
    }

    async fn comments(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
        post_id: Option<i64>,
        include_deleted: Option<bool>,
    ) -> async_graphql::Result<CommentConnection> {
        let pool = ctx.data::<SqlitePool>()?;
        let limit = limit.unwrap_or(20) as i64;
        let offset = offset.unwrap_or(0) as i64;

        let mut where_parts: Vec<String> = vec!["1=1".to_string()];

        if post_id.is_some() {
            where_parts.push("c.post_id = ?".to_string());
        }

        if !include_deleted.unwrap_or(false) {
            where_parts.push("c.is_deleted = 0".to_string());
        }

        let where_clause = where_parts.join(" AND ");

        let count_sql = format!(
            "SELECT COUNT(*) FROM comments c JOIN posts p ON p.id = c.post_id WHERE {}",
            where_clause
        );
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
        if let Some(pid) = post_id {
            count_query = count_query.bind(pid);
        }
        let total: i64 = count_query
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let data_sql = format!(
            r#"
            SELECT c.id, c.content, c.parent_id, c.is_deleted, c.created_at,
                   p.title AS post_title, p.slug AS post_slug,
                   u.username AS author_username, um.display_name AS author_name
            FROM comments c
            JOIN posts p ON p.id = c.post_id
            LEFT JOIN users u ON u.id = c.user_id
            LEFT JOIN user_meta um ON um.user_id = c.user_id
            WHERE {}
            ORDER BY c.created_at DESC
            LIMIT {} OFFSET {}
            "#,
            where_clause, limit, offset
        );
        let mut data_query = sqlx::query_as::<_, CommentRow>(&data_sql);
        if let Some(pid) = post_id {
            data_query = data_query.bind(pid);
        }
        let rows = data_query
            .fetch_all(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let items = rows
            .into_iter()
            .map(|r| GqlComment {
                id: r.id,
                content: r.content,
                post_title: r.post_title,
                post_slug: r.post_slug,
                author_name: r.author_name,
                author_username: r.author_username,
                parent_id: r.parent_id,
                is_deleted: r.is_deleted != 0,
                created_at: r.created_at,
            })
            .collect();

        Ok(CommentConnection { items, total })
    }

    async fn media(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
        search: Option<String>,
    ) -> async_graphql::Result<MediaConnection> {
        let pool = ctx.data::<SqlitePool>()?;
        let limit = limit.unwrap_or(20) as i64;
        let offset = offset.unwrap_or(0) as i64;

        let mut where_parts: Vec<String> = vec!["1=1".to_string()];

        if search.is_some() {
            where_parts.push(
                "(LOWER(m.short_name) LIKE '%' || LOWER(?) || '%' OR LOWER(m.file_name) LIKE '%' || LOWER(?) || '%')".to_string()
            );
        }

        let where_clause = where_parts.join(" AND ");

        let count_sql = format!("SELECT COUNT(*) FROM media m WHERE {}", where_clause);
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
        if let Some(ref s) = search {
            count_query = count_query.bind(s).bind(s);
        }
        let total: i64 = count_query
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let data_sql = format!(
            r#"
            SELECT m.id, m.short_name, m.file_name, m.file_type, m.url, m.size,
                   m.description, m.use_count, m.created_at,
                   um.display_name AS uploader_name
            FROM media m
            LEFT JOIN users u ON u.id = m.uploader_id
            LEFT JOIN user_meta um ON um.user_id = m.uploader_id
            WHERE {}
            ORDER BY m.created_at DESC
            LIMIT {} OFFSET {}
            "#,
            where_clause, limit, offset
        );
        let mut data_query = sqlx::query_as::<_, MediaRow>(&data_sql);
        if let Some(ref s) = search {
            data_query = data_query.bind(s).bind(s);
        }
        let rows = data_query
            .fetch_all(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let items = rows
            .into_iter()
            .map(|r| GqlMedia {
                id: r.id,
                short_name: r.short_name,
                file_name: r.file_name,
                file_type: r.file_type,
                url: r.url,
                size: r.size,
                description: r.description,
                use_count: r.use_count,
                created_at: r.created_at,
                uploader_name: r.uploader_name,
            })
            .collect();

        Ok(MediaConnection { items, total })
    }

    async fn series(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> async_graphql::Result<SeriesConnection> {
        let pool = ctx.data::<SqlitePool>()?;
        let limit = limit.unwrap_or(20) as i64;
        let offset = offset.unwrap_or(0) as i64;

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM series")
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let data_sql = format!(
            r#"
            SELECT s.id, s.title, s.slug, s.description, s.created_at,
                   'media/i/' || m.short_name AS cover_url,
                   u.username AS owner_username,
                   COUNT(sp.post_id) AS post_count
            FROM series s
            LEFT JOIN series_post sp ON sp.series_id = s.id
            LEFT JOIN media m ON m.id = s.cover_image_id
            LEFT JOIN users u ON u.id = s.user_id
            GROUP BY s.id
            ORDER BY s.created_at DESC
            LIMIT {} OFFSET {}
            "#,
            limit, offset
        );

        let rows = sqlx::query_as::<_, SeriesRow>(&data_sql)
            .fetch_all(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let items = rows
            .into_iter()
            .map(|r| GqlSeries {
                id: r.id,
                title: r.title,
                slug: r.slug,
                description: r.description,
                cover_url: r.cover_url,
                owner_username: r.owner_username,
                post_count: r.post_count,
                created_at: r.created_at,
            })
            .collect();

        Ok(SeriesConnection { items, total })
    }

    async fn tags(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> async_graphql::Result<TagConnection> {
        let pool = ctx.data::<SqlitePool>()?;
        let limit = limit.unwrap_or(20) as i64;
        let offset = offset.unwrap_or(0) as i64;

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tags")
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let data_sql = format!(
            r#"
            SELECT t.id, t.name, t.slug, t.description,
                   COUNT(pt.post_id) AS post_count
            FROM tags t
            LEFT JOIN post_tags pt ON pt.tag_id = t.id
            GROUP BY t.id
            ORDER BY post_count DESC
            LIMIT {} OFFSET {}
            "#,
            limit, offset
        );

        let rows = sqlx::query_as::<_, TagRow>(&data_sql)
            .fetch_all(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let items = rows
            .into_iter()
            .map(|r| GqlTag {
                id: r.id,
                name: r.name,
                slug: r.slug,
                description: r.description,
                post_count: r.post_count,
            })
            .collect();

        Ok(TagConnection { items, total })
    }

    async fn categories(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> async_graphql::Result<CategoryConnection> {
        let pool = ctx.data::<SqlitePool>()?;
        let limit = limit.unwrap_or(20) as i64;
        let offset = offset.unwrap_or(0) as i64;

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM categories")
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let data_sql = format!(
            r#"
            SELECT c.id, c.name, c.slug, c.description,
                   COUNT(pc.post_id) AS post_count
            FROM categories c
            LEFT JOIN post_categories pc ON pc.category_id = c.id
            GROUP BY c.id
            ORDER BY post_count DESC
            LIMIT {} OFFSET {}
            "#,
            limit, offset
        );

        let rows = sqlx::query_as::<_, CategoryRow>(&data_sql)
            .fetch_all(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let items = rows
            .into_iter()
            .map(|r| GqlCategory {
                id: r.id,
                name: r.name,
                slug: r.slug,
                description: r.description,
                post_count: r.post_count,
            })
            .collect();

        Ok(CategoryConnection { items, total })
    }

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

    async fn dashboard_posts(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
        search: Option<String>,
    ) -> async_graphql::Result<DashboardPostConnection> {
        let pool = ctx.data::<SqlitePool>()?;
        let limit = limit.unwrap_or(20) as i64;
        let offset = offset.unwrap_or(0) as i64;

        let mut where_parts: Vec<String> = vec!["p.content_kind = 'post'".to_string()];
        if search.is_some() {
            where_parts.push(
                "(LOWER(p.title) LIKE '%' || LOWER(?) || '%' OR LOWER(p.slug) LIKE '%' || LOWER(?) || '%')".to_string()
            );
        }
        let where_clause = where_parts.join(" AND ");

        let count_sql = format!("SELECT COUNT(*) FROM posts p WHERE {}", where_clause);
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
        if let Some(ref s) = search {
            count_query = count_query.bind(s).bind(s);
        }
        let total: i64 = count_query
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let data_sql = format!(
            r#"SELECT {} {} WHERE {} ORDER BY p.updated_at DESC LIMIT {} OFFSET {}"#,
            DASHBOARD_POST_COLUMNS, DASHBOARD_POST_JOINS, where_clause, limit, offset
        );
        let mut data_query = sqlx::query_as::<_, DashboardPostRow>(&data_sql);
        if let Some(ref s) = search {
            data_query = data_query.bind(s).bind(s);
        }
        let rows = data_query
            .fetch_all(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let items = attach_tags_to_posts(pool, rows)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(DashboardPostConnection { items, total })
    }

    async fn dashboard_projects(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
        search: Option<String>,
    ) -> async_graphql::Result<ProjectConnection> {
        let pool = ctx.data::<SqlitePool>()?;
        let limit = limit.unwrap_or(20) as i64;
        let offset = offset.unwrap_or(0) as i64;

        let mut where_parts: Vec<String> = vec![];
        if search.is_some() {
            where_parts.push(
                "(LOWER(p.title) LIKE '%' || LOWER(?) || '%' OR LOWER(p.slug) LIKE '%' || LOWER(?) || '%')".to_string()
            );
        }
        let where_clause = if where_parts.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_parts.join(" AND "))
        };

        let count_sql = format!(
            r#"SELECT COUNT(*) FROM projects JOIN posts p ON p.id = projects.post_id {}"#,
            where_clause
        );
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
        if let Some(ref s) = search {
            count_query = count_query.bind(s).bind(s);
        }
        let total: i64 = count_query
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let data_sql = format!(
            r#"
            SELECT
                projects.id AS project_id, p.id AS post_id, p.title, p.slug, p.excerpt,
                u.username AS author_slug, um.display_name AS author_name, p.status,
                'media/i/' || m.short_name AS url, m.file_type AS cover_media_type,
                projects.demo_type, ps.views, ps.likes, ps.comments_count
            FROM projects
            JOIN posts p ON p.id = projects.post_id
            JOIN users u ON u.id = p.user_id
            JOIN user_meta um ON um.user_id = p.user_id
            JOIN post_stats ps ON ps.post_id = p.id
            LEFT JOIN media m ON m.id = p.cover_media_id
            {}
            ORDER BY p.updated_at DESC
            LIMIT {} OFFSET {}
            "#,
            where_clause, limit, offset
        );
        let mut data_query = sqlx::query_as::<_, DashboardProjectRow>(&data_sql);
        if let Some(ref s) = search {
            data_query = data_query.bind(s).bind(s);
        }
        let rows = data_query
            .fetch_all(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let items = attach_tags_to_projects(pool, rows)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(ProjectConnection { items, total })
    }

    async fn featured_posts(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
    ) -> async_graphql::Result<Vec<GqlDashboardPost>> {
        let pool = ctx.data::<SqlitePool>()?;
        let limit = limit.unwrap_or(100) as i64;

        let sql = format!(
            r#"SELECT {} {} WHERE p.is_featured = 1 AND p.status = 'published' ORDER BY p.updated_at DESC LIMIT {}"#,
            DASHBOARD_POST_COLUMNS, DASHBOARD_POST_JOINS, limit
        );
        let rows = sqlx::query_as::<_, DashboardPostRow>(&sql)
            .fetch_all(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        attach_tags_to_posts(pool, rows)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))
    }

    async fn featured_projects(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
    ) -> async_graphql::Result<Vec<GqlDashboardProject>> {
        let pool = ctx.data::<SqlitePool>()?;
        let limit = limit.unwrap_or(100) as i64;

        let sql = format!(
            r#"
            SELECT
                projects.id AS project_id, p.id AS post_id, p.title, p.slug, p.excerpt,
                u.username AS author_slug, um.display_name AS author_name, p.status,
                'media/i/' || m.short_name AS url, m.file_type AS cover_media_type,
                projects.demo_type, ps.views, ps.likes, ps.comments_count
            FROM projects
            JOIN posts p ON p.id = projects.post_id
            JOIN users u ON u.id = p.user_id
            JOIN user_meta um ON um.user_id = p.user_id
            JOIN post_stats ps ON ps.post_id = p.id
            LEFT JOIN media m ON m.id = p.cover_media_id
            WHERE p.is_featured = 1 AND p.status = 'published'
            ORDER BY p.updated_at DESC
            LIMIT {}
            "#,
            limit
        );
        let rows = sqlx::query_as::<_, DashboardProjectRow>(&sql)
            .fetch_all(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        attach_tags_to_projects(pool, rows)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))
    }

    async fn series_posts(
        &self,
        ctx: &Context<'_>,
        series_id: i64,
    ) -> async_graphql::Result<Vec<GqlSeriesPost>> {
        let pool = ctx.data::<SqlitePool>()?;

        let rows = sqlx::query_as::<_, SeriesPostRow>(
            r#"
            SELECT sp.post_id, p.title, p.slug, p.status, sp.number,
                   'media/i/' || m.short_name AS url
            FROM series_post sp
            JOIN posts p ON p.id = sp.post_id
            LEFT JOIN media m ON m.id = p.cover_media_id
            WHERE sp.series_id = ?
            ORDER BY sp.number ASC
            "#,
        )
        .bind(series_id)
        .fetch_all(pool)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let items = rows
            .into_iter()
            .map(|r| GqlSeriesPost {
                post_id: r.post_id,
                title: r.title,
                slug: r.slug,
                status: r.status,
                number: r.number,
                cover_url: r.url,
            })
            .collect();

        Ok(items)
    }

    async fn post_detail(
        &self,
        ctx: &Context<'_>,
        id: i64,
    ) -> async_graphql::Result<GqlPostDetail> {
        let pool = ctx.data::<SqlitePool>()?;

        let row = sqlx::query_as::<_, PostDetailRow>(
            r#"
            SELECT p.id, p.title, p.slug, p.excerpt, p.content, p.draft, p.status,
                   p.is_featured, p.view_count, p.published_at, p.created_at, p.updated_at,
                   u.username AS author_slug, um.display_name AS author_name,
                   'media/i/' || m.short_name AS cover_url, m.file_type AS cover_media_type,
                   s.title AS series_title, s.slug AS series_slug, p.og_image_seconds
            FROM posts p
            JOIN users u ON u.id = p.user_id
            JOIN user_meta um ON um.user_id = p.user_id
            LEFT JOIN media m ON m.id = p.cover_media_id
            LEFT JOIN series s ON s.id = p.series_id
            WHERE p.id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?
        .ok_or_else(|| async_graphql::Error::new("Post not found"))?;

        let tag_rows = sqlx::query_as::<_, (String, String)>(
            r#"SELECT tags.name, tags.slug FROM post_tags JOIN tags ON tags.id = post_tags.tag_id WHERE post_tags.post_id = ?"#,
        )
        .bind(id)
        .fetch_all(pool).await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let (tag_names, tag_slugs): (Vec<String>, Vec<String>) = tag_rows.into_iter().unzip();

        let medium_rows = sqlx::query_as::<_, (String, String)>(
            r#"SELECT m.short_name, 'media/i/' || m.short_name AS url FROM post_media pm JOIN media m ON m.id = pm.media_id WHERE pm.post_id = ?"#,
        )
        .bind(id)
        .fetch_all(pool).await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let (medium_short_names, medium_urls): (Vec<String>, Vec<String>) =
            medium_rows.into_iter().unzip();

        Ok(GqlPostDetail {
            id: row.id,
            title: row.title,
            slug: row.slug,
            excerpt: row.excerpt,
            content: row.content,
            draft: row.draft,
            status: row.status,
            is_featured: row.is_featured != 0,
            author_name: row.author_name,
            author_slug: row.author_slug,
            tag_names,
            tag_slugs,
            cover_url: row.cover_url,
            cover_media_type: row.cover_media_type,
            series_title: row.series_title,
            series_slug: row.series_slug,
            views: row.view_count,
            likes: 0,
            comments_count: 0,
            published_at: row.published_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
            medium_urls,
            medium_short_names,
            og_image_seconds: row.og_image_seconds,
        })
    }

    async fn project_detail(
        &self,
        ctx: &Context<'_>,
        id: i64,
    ) -> async_graphql::Result<GqlDashboardProject> {
        let pool = ctx.data::<SqlitePool>()?;

        let row = sqlx::query_as::<_, DashboardProjectRow>(
            r#"
            SELECT
                projects.id AS project_id, p.id AS post_id, p.title, p.slug, p.excerpt,
                u.username AS author_slug, um.display_name AS author_name, p.status,
                'media/i/' || m.short_name AS url, m.file_type AS cover_media_type,
                projects.demo_type, ps.views, ps.likes, ps.comments_count
            FROM projects
            JOIN posts p ON p.id = projects.post_id
            JOIN users u ON u.id = p.user_id
            JOIN user_meta um ON um.user_id = p.user_id
            JOIN post_stats ps ON ps.post_id = p.id
            LEFT JOIN media m ON m.id = p.cover_media_id
            WHERE projects.id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?
        .ok_or_else(|| async_graphql::Error::new("Project not found"))?;

        let tag_rows = sqlx::query_as::<_, (String, String)>(
            r#"SELECT tags.name, tags.slug FROM post_tags JOIN tags ON tags.id = post_tags.tag_id WHERE post_tags.post_id = ?"#,
        )
        .bind(row.post_id)
        .fetch_all(pool).await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let (tag_names, tag_slugs): (Vec<String>, Vec<String>) = tag_rows.into_iter().unzip();

        Ok(GqlDashboardProject {
            id: row.project_id,
            post_id: row.post_id,
            title: row.title,
            slug: row.slug,
            excerpt: row.excerpt,
            author_name: row.author_name,
            author_slug: row.author_slug,
            tag_names,
            tag_slugs,
            status: row.status,
            cover_url: row.url,
            cover_media_type: row.cover_media_type,
            demo_type: row.demo_type,
            views: row.views,
            likes: row.likes,
            comments_count: row.comments_count,
        })
    }

    async fn related_posts(
        &self,
        ctx: &Context<'_>,
        post_id: i64,
    ) -> async_graphql::Result<Vec<GqlDashboardPost>> {
        let pool = ctx.data::<SqlitePool>()?;

        let sql = format!(
            r#"SELECT {} {} JOIN related_posts rp ON rp.related_post_id = p.id WHERE rp.post_id = ? ORDER BY p.updated_at DESC"#,
            DASHBOARD_POST_COLUMNS, DASHBOARD_POST_JOINS
        );
        let rows = sqlx::query_as::<_, DashboardPostRow>(&sql)
            .bind(post_id)
            .fetch_all(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        attach_tags_to_posts(pool, rows)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))
    }

    async fn check_slug(&self, ctx: &Context<'_>, slug: String) -> async_graphql::Result<bool> {
        let pool = ctx.data::<SqlitePool>()?;

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE slug = ?")
            .bind(&slug)
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(count == 0)
    }

    async fn check_project_slug(
        &self,
        ctx: &Context<'_>,
        slug: String,
    ) -> async_graphql::Result<bool> {
        let pool = ctx.data::<SqlitePool>()?;

        let count: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM posts p JOIN projects ON projects.post_id = p.id WHERE p.slug = ?"#,
        )
        .bind(&slug)
        .fetch_one(pool).await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(count == 0)
    }
}

pub type BlogSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

pub fn build_schema(pool: SqlitePool) -> BlogSchema {
    Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(pool)
        .finish()
}
