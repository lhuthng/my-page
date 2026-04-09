use std::collections::HashMap;

use sqlx::{SqlitePool, prelude::FromRow};

use crate::{
    application::{
        commands::user::{
            ChangeDetailsCommand, GetPostsCommand, GetUserCommand, MeCommand, SearchUserCommand,
        },
        services::user::UserService,
    },
    domain::{
        entities::{
            post::{PostSnapshot, PostStats},
            user::{Me, User, UserSummary},
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
    score: i32,
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
        .bind(&cmd.user_id)
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
        let mut tx = self.pool.begin().await?;

        if cmd.bio == None && cmd.display_name == None {
            return Ok(());
        }

        let mut values: Vec<&str> = vec![];

        if let Some(_) = cmd.display_name {
            values.push("display_name = ?");
        }
        if let Some(_) = cmd.bio {
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

        query.bind(&cmd.user_id).execute(&mut *tx).await?;

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
            SELECT p.id AS post_id, title, slug, excerpt, username AS author_slug, display_name AS author_name, status, 'media/i/' || m.short_name AS url, views, likes, comments_count
            FROM posts p
                JOIN users u ON u.id = p.user_id
                JOIN user_meta um ON u.id = um.user_id
                JOIN post_stats ps ON p.id = ps.post_id
                LEFT JOIN media m ON m.id = p.cover_image_id
            WHERE u.username = ? {}
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
            .bind(&cmd.limit)
            .bind(&cmd.offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| UserError::InternalError(e.to_string()))?;

        if post_rows.len() == 0 {
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
            query = query.bind(post_row.post_id.clone());
            posts.push(post_row.into_snapshot(vec![], vec![]));
        }

        let tag_rows = query.fetch_all(&self.pool).await?;

        for tag_row in tag_rows {
            if let Some(index) = posts_map.get_mut(&tag_row.post_id) {
                if let Some(post) = posts.get_mut(*index) {
                    post.tag_names.push(tag_row.tag_name);
                    post.tag_slugs.push(tag_row.tag_slug);
                }
            }
        }

        Ok(posts)
    }
}
