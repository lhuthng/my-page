use std::collections::HashMap;

use sqlx::{SqlitePool, prelude::FromRow};

use crate::{
    application::{
        commands::user::{
            ChangeDetailsCommand, GetLatestCommentsCommand, GetPostsCommand, GetUserCommand,
            MeCommand, SearchUserCommand,
        },
        services::user::UserService,
    },
    domain::{
        entities::{
            post::PostSnapshot,
            user::{LatestComment, Me, User, UserSummary},
        },
        errors::user::UserError,
    },
    infrastructure::persistence::post::PostRow,
};

pub struct UserServiceImpl {
    pub pool: SqlitePool,
}

impl UserServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow, Debug)]
struct MeRow {
    username: String,
    display_name: String,
    role: String,
    avatar_url: Option<String>,
}

#[derive(FromRow, Debug)]
struct UserRow {
    username: String,
    display_name: String,
    bio: String,
    role: String,
    avatar_url: Option<String>,
}

#[derive(FromRow, Debug)]
struct UserSearchRow {
    username: String,
    display_name: String,
    role: String,
    avatar_url: Option<String>,
    #[allow(dead_code)]
    score: i32,
}

#[derive(Debug, FromRow)]
struct LatestCommentRow {
    id: i64,
    parent_id: Option<i64>,
    content: String,
    created_at: String,
    post_title: String,
    post_slug: String,
    avatar_url: Option<String>,
    display_name: Option<String>,
    username: Option<String>,
}

#[derive(Debug, FromRow)]
pub struct TagRow {
    pub post_id: i64,
    pub tag_name: String,
    pub tag_slug: String,
}

#[async_trait::async_trait]
impl UserService for UserServiceImpl {
    async fn me(&self, cmd: MeCommand) -> Result<Me, UserError> {
        let me_row = sqlx::query_as::<_, MeRow>(
            r#"
			SELECT username, display_name, role, 'media/i/' || media.short_name AS avatar_url
			FROM user_meta JOIN users ON users.id = user_id
			LEFT JOIN media on user_meta.avatar_image_id = media.id
			WHERE user_id = ?
			"#,
        )
        .bind(cmd.user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(Me {
            username: me_row.username,
            display_name: me_row.display_name,
            role: me_row.role,
            avatar_url: me_row.avatar_url,
        })
    }
    async fn change_details(&self, cmd: ChangeDetailsCommand) -> Result<(), UserError> {
        use crate::helper::string::*;
        let cmd = ChangeDetailsCommand {
            display_name: cmd
                .display_name
                .map(|v| validate_text(&v, "Display name", 60).map_err(UserError::InvalidData))
                .transpose()?,
            bio: validate_optional_long_text(cmd.bio.as_deref(), "Bio", 500)
                .map_err(UserError::InvalidData)?,

            ..cmd
        };

        let mut tx = self.pool.begin().await?;

        if cmd.bio.is_none() && cmd.display_name.is_none() {
            return Ok(());
        }

        let mut values: Vec<&str> = vec![];

        if cmd.display_name.is_some() {
            values.push("display_name = ?");
        }
        if cmd.bio.is_some() {
            values.push("bio = ?");
        }

        let sequel = format!(
            r#"
            UPDATE user_meta
            SET {}
            WHERE user_id = ?
            "#,
            values.join(", ")
        );

        let mut query = sqlx::query(&sequel);

        if let Some(username) = cmd.display_name {
            query = query.bind(username);
        }

        if let Some(bio) = cmd.bio {
            query = query.bind(bio);
        }

        query.bind(cmd.user_id).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }
    async fn search(&self, cmd: SearchUserCommand) -> Result<Vec<UserSummary>, UserError> {
        let rows = sqlx::query_as::<_, UserSearchRow>(
            r#"
            SELECT DISTINCT
                u.username,
                u.role,
                um.display_name,
                'media/i/' || m.short_name AS avatar_url,
                CASE
                    WHEN LOWER(um.display_name) = LOWER(?1) THEN 3
                    WHEN LOWER(um.display_name) LIKE LOWER(?1) || '%' THEN 2
                    WHEN LOWER(um.display_name) LIKE '%' || LOWER(?1) || '%' THEN 1
                    WHEN LOWER(u.username) LIKE '%' || LOWER(?1) || '%' THEN 1
                    ELSE 0
                END AS score
            FROM users AS u
            JOIN user_meta AS um ON um.user_id = u.id
            LEFT JOIN media AS m ON m.id = um.avatar_image_id
            WHERE
                LOWER(um.display_name) LIKE '%' || LOWER(?1) || '%'
                OR LOWER(u.username) LIKE '%' || LOWER(?1) || '%'
            ORDER BY score DESC, u.created_at DESC
            LIMIT ?2 OFFSET ?3;
            "#,
        )
        .bind(&cmd.term)
        .bind(cmd.size)
        .bind(cmd.offset)
        .fetch_all(&self.pool)
        .await?;

        let summaries = rows
            .into_iter()
            .map(
                |UserSearchRow {
                     username,
                     display_name,
                     role,
                     avatar_url,
                     score: _,
                 }| UserSummary {
                    username,
                    display_name,
                    role,
                    avatar_url,
                },
            )
            .collect::<Vec<_>>();
        Ok(summaries)
    }
    async fn get_user(&self, cmd: GetUserCommand) -> Result<User, UserError> {
        let user_row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT username, display_name, bio, role, 'media/i/' || media.short_name AS avatar_url
            FROM users
            JOIN user_meta ON users.id = user_id
            LEFT JOIN media ON media.id = avatar_image_id
            WHERE username = ?
            "#,
        )
        .bind(&cmd.username)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| UserError::NotFound)?;

        Ok(User {
            username: user_row.username,
            display_name: user_row.display_name,
            avatar_url: user_row.avatar_url,
            bio: user_row.bio,
            role: user_row.role,
        })
    }
    async fn get_posts(&self, cmd: GetPostsCommand) -> Result<Vec<PostSnapshot>, UserError> {
        let filtered = match cmd.user_id {
            None => true,
            Some(user_id) => {
                let exist: bool = sqlx::query_scalar(
                    r#"
                SELECT EXISTS (
                    SELECT 1
                    FROM users
                    WHERE id = ? AND username = ?
                )
                "#,
                )
                .bind(user_id)
                .bind(&cmd.username)
                .fetch_one(&self.pool)
                .await?;

                if !exist {
                    return Err(UserError::Unauthorized);
                }

                false
            }
        };

        let sql = format!(
            r#"
            SELECT p.id AS post_id, title, slug, excerpt, username AS author_slug, display_name AS author_name, status, 'media/i/' || m.short_name AS url, m.file_type AS cover_media_type, views, likes, comments_count, reading_time_minutes
            FROM posts p
                JOIN users u ON u.id = p.user_id
                JOIN user_meta um ON u.id = um.user_id
                JOIN post_stats ps ON p.id = ps.post_id
                LEFT JOIN media m ON m.id = p.cover_media_id
            WHERE u.username = ? AND p.content_kind = 'post' {}
            ORDER BY p.updated_at DESC
            LIMIT ? OFFSET ?
            "#,
            if filtered {
                "AND p.status = 'published' "
            } else {
                ""
            }
        );

        let post_rows = sqlx::query_as::<_, PostRow>(&sql)
            .bind(&cmd.username)
            .bind(cmd.limit)
            .bind(cmd.offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| UserError::InternalError(e.to_string()))?;

        if post_rows.is_empty() {
            return Ok(vec![]);
        }

        let placeholder = post_rows
            .iter()
            .map(|_| "?".to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let sequel = format!(
            r#"
            SELECT post_id, tag_id, tags.name AS tag_name, tags.slug AS tag_slug
            FROM post_tags
            JOIN tags ON tags.id = post_tags.tag_id
            WHERE post_tags.post_id IN ({})
            "#,
            placeholder
        );

        let mut query = sqlx::query_as::<_, TagRow>(&sequel);

        let mut posts_map: HashMap<i64, usize> = HashMap::new();

        let mut posts = vec![];

        for post_row in post_rows {
            posts_map.insert(post_row.post_id, posts.len());
            query = query.bind(post_row.post_id);
            posts.push(post_row.into_snapshot(vec![], vec![]));
        }

        let tag_rows = query.fetch_all(&self.pool).await?;

        for tag_row in tag_rows {
            if let Some(index) = posts_map.get_mut(&tag_row.post_id)
                && let Some(post) = posts.get_mut(*index)
            {
                post.tag_names.push(tag_row.tag_name);
                post.tag_slugs.push(tag_row.tag_slug);
            }
        }

        Ok(posts)
    }

    async fn get_latest_comments(
        &self,
        cmd: GetLatestCommentsCommand,
    ) -> Result<Vec<LatestComment>, UserError> {
        let rows = sqlx::query_as::<_, LatestCommentRow>(
            r#"
            SELECT c.id,
                   c.parent_id,
                   c.content,
                   c.created_at,
                   p.title AS post_title,
                   p.slug AS post_slug,
                   'media/i/' || m.short_name AS avatar_url,
                   um.display_name,
                   u.username
            FROM comments c
            JOIN posts p ON p.id = c.post_id
            LEFT JOIN users u ON u.id = c.user_id
            LEFT JOIN user_meta um ON um.user_id = c.user_id
            LEFT JOIN media m ON m.id = um.avatar_image_id
            WHERE u.username = ?
            ORDER BY c.created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(&cmd.username)
        .bind(cmd.limit)
        .bind(cmd.offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| LatestComment {
                id: row.id,
                parent_id: row.parent_id,
                content: row.content,
                created_at: row.created_at,
                post_title: row.post_title,
                post_slug: row.post_slug,
                avatar_url: row.avatar_url,
                display_name: row.display_name,
                username: row.username,
            })
            .collect())
    }
}
