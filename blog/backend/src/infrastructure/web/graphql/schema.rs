use async_graphql::{Context, EmptyMutation, EmptySubscription, Object, Schema, SimpleObject};
use sqlx::SqlitePool;

// ─── GQL output types ─────────────────────────────────────────────────────────

#[derive(SimpleObject)]
pub struct GqlUser {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub role: String,
    pub display_name: String,
    pub bio: String,
    pub avatar_url: Option<String>,
    pub created_at: String,
}

#[derive(SimpleObject)]
pub struct UserConnection {
    pub items: Vec<GqlUser>,
    pub total: i64,
}

#[derive(SimpleObject)]
pub struct GqlPost {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub status: String,
    pub author_name: Option<String>,
    pub author_slug: Option<String>,
    pub series_title: Option<String>,
    pub view_count: i64,
    pub is_featured: bool,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub excerpt: Option<String>,
}

#[derive(SimpleObject)]
pub struct PostConnection {
    pub items: Vec<GqlPost>,
    pub total: i64,
}

#[derive(SimpleObject)]
pub struct GqlComment {
    pub id: i64,
    pub content: String,
    pub post_title: String,
    pub post_slug: String,
    pub author_name: Option<String>,
    pub author_username: Option<String>,
    pub parent_id: Option<i64>,
    pub is_deleted: bool,
    pub created_at: String,
}

#[derive(SimpleObject)]
pub struct CommentConnection {
    pub items: Vec<GqlComment>,
    pub total: i64,
}

#[derive(SimpleObject)]
pub struct GqlMedia {
    pub id: i64,
    pub short_name: String,
    pub file_name: String,
    pub file_type: String,
    pub url: String,
    pub size: i64,
    pub description: Option<String>,
    pub uploader_name: Option<String>,
    pub use_count: i64,
    pub created_at: String,
}

#[derive(SimpleObject)]
pub struct MediaConnection {
    pub items: Vec<GqlMedia>,
    pub total: i64,
}

#[derive(SimpleObject)]
pub struct GqlSeries {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub description: Option<String>,
    pub post_count: i64,
    pub created_at: String,
}

#[derive(SimpleObject)]
pub struct SeriesConnection {
    pub items: Vec<GqlSeries>,
    pub total: i64,
}

#[derive(SimpleObject)]
pub struct GqlTag {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub post_count: i64,
}

#[derive(SimpleObject)]
pub struct TagConnection {
    pub items: Vec<GqlTag>,
    pub total: i64,
}

#[derive(SimpleObject)]
pub struct GqlCategory {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub post_count: i64,
}

#[derive(SimpleObject)]
pub struct CategoryConnection {
    pub items: Vec<GqlCategory>,
    pub total: i64,
}

#[derive(SimpleObject)]
pub struct DbStats {
    pub total_users: i64,
    pub total_posts: i64,
    pub total_comments: i64,
    pub total_media: i64,
    pub total_series: i64,
    pub total_tags: i64,
    pub total_categories: i64,
}

// ─── SQL row structs ──────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct UserRow {
    id: i64,
    username: String,
    email: String,
    role: String,
    created_at: String,
    display_name: String,
    bio: String,
    avatar_url: Option<String>,
}

#[derive(sqlx::FromRow)]
struct GqlPostRow {
    id: i64,
    title: String,
    slug: String,
    status: String,
    view_count: i64,
    is_featured: i64,
    published_at: Option<String>,
    created_at: String,
    updated_at: String,
    excerpt: Option<String>,
    author_slug: Option<String>,
    author_name: Option<String>,
    series_title: Option<String>,
}

#[derive(sqlx::FromRow)]
struct CommentRow {
    id: i64,
    content: String,
    parent_id: Option<i64>,
    is_deleted: i64,
    created_at: String,
    post_title: String,
    post_slug: String,
    author_username: Option<String>,
    author_name: Option<String>,
}

#[derive(sqlx::FromRow)]
struct MediaRow {
    id: i64,
    short_name: String,
    file_name: String,
    file_type: String,
    url: String,
    size: i64,
    description: Option<String>,
    use_count: i64,
    created_at: String,
    uploader_name: Option<String>,
}

#[derive(sqlx::FromRow)]
struct SeriesRow {
    id: i64,
    title: String,
    slug: String,
    description: Option<String>,
    created_at: String,
    post_count: i64,
}

#[derive(sqlx::FromRow)]
struct TagRow {
    id: i64,
    name: String,
    slug: String,
    post_count: i64,
}

#[derive(sqlx::FromRow)]
struct CategoryRow {
    id: i64,
    name: String,
    slug: String,
    description: Option<String>,
    post_count: i64,
}

// ─── QueryRoot ────────────────────────────────────────────────────────────────

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

        if let Some(ref s) = search {
            let escaped = s.replace('\'', "''");
            where_parts.push(format!(
                "(LOWER(u.username) LIKE '%' || LOWER('{}') || '%' OR LOWER(um.display_name) LIKE '%' || LOWER('{}') || '%')",
                escaped, escaped
            ));
        }

        if let Some(ref r) = role {
            let escaped = r.replace('\'', "''");
            where_parts.push(format!("u.role = '{}'", escaped));
        }

        let where_clause = where_parts.join(" AND ");

        let count_sql = format!(
            "SELECT COUNT(*) FROM users u JOIN user_meta um ON um.user_id = u.id WHERE {}",
            where_clause
        );
        let total: i64 = sqlx::query_scalar(&count_sql)
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

        let rows = sqlx::query_as::<_, UserRow>(&data_sql)
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

        if let Some(ref s) = search {
            let escaped = s.replace('\'', "''");
            where_parts.push(format!(
                "(LOWER(p.title) LIKE '%' || LOWER('{}') || '%' OR LOWER(p.slug) LIKE '%' || LOWER('{}') || '%')",
                escaped, escaped
            ));
        }

        if let Some(ref st) = status {
            let escaped = st.replace('\'', "''");
            where_parts.push(format!("p.status = '{}'", escaped));
        }

        let where_clause = where_parts.join(" AND ");

        let count_sql = format!("SELECT COUNT(*) FROM posts p WHERE {}", where_clause);
        let total: i64 = sqlx::query_scalar(&count_sql)
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

        let rows = sqlx::query_as::<_, GqlPostRow>(&data_sql)
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

        if let Some(pid) = post_id {
            where_parts.push(format!("c.post_id = {}", pid));
        }

        if !include_deleted.unwrap_or(false) {
            where_parts.push("c.is_deleted = 0".to_string());
        }

        let where_clause = where_parts.join(" AND ");

        let count_sql = format!(
            "SELECT COUNT(*) FROM comments c JOIN posts p ON p.id = c.post_id WHERE {}",
            where_clause
        );
        let total: i64 = sqlx::query_scalar(&count_sql)
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

        let rows = sqlx::query_as::<_, CommentRow>(&data_sql)
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

        if let Some(ref s) = search {
            let escaped = s.replace('\'', "''");
            where_parts.push(format!(
                "(LOWER(m.short_name) LIKE '%' || LOWER('{}') || '%' OR LOWER(m.file_name) LIKE '%' || LOWER('{}') || '%')",
                escaped, escaped
            ));
        }

        let where_clause = where_parts.join(" AND ");

        let count_sql = format!("SELECT COUNT(*) FROM media m WHERE {}", where_clause);
        let total: i64 = sqlx::query_scalar(&count_sql)
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

        let rows = sqlx::query_as::<_, MediaRow>(&data_sql)
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
                   COUNT(sp.post_id) AS post_count
            FROM series s
            LEFT JOIN series_post sp ON sp.series_id = s.id
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
            SELECT t.id, t.name, t.slug,
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
}

// ─── Schema builder ───────────────────────────────────────────────────────────

pub type BlogSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

pub fn build_schema(pool: SqlitePool) -> BlogSchema {
    Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(pool)
        .finish()
}
