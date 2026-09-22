// Reading a post's comment thread.

use crate::application::commands::post::GetCommentsCommand;
use crate::domain::entities::post::{Comment, CommentPage};
use crate::domain::errors::post::PostError;

use super::PostServiceImpl;

impl PostServiceImpl {
    pub(super) async fn get_comments(
        &self,
        cmd: GetCommentsCommand,
    ) -> Result<CommentPage, PostError> {
        if let Some(parent_id) = cmd.parent_id {
            let parent_post_id: Option<i64> = sqlx::query_scalar(
                r#"
                SELECT post_id
                FROM comments
                WHERE id = ?
                "#,
            )
            .bind(parent_id)
            .fetch_optional(&self.pool)
            .await?;

            if parent_post_id != Some(cmd.post_id) {
                return Err(PostError::PostNotFound);
            }

            let sequel = format!(
                r#"
                SELECT
                    comments.id,
                    comments.parent_id,
                    content,
                    comments.created_at,
                    users.username,
                    user_meta.display_name,
                    media.url,
                    users.role,
                    comments.guest_identity,
                    (
                        SELECT COUNT(*)
                        FROM comments AS replies
                        WHERE replies.parent_id = comments.id
                          AND replies.is_deleted = 0
                    ) AS direct_reply_count
                FROM comments
                LEFT JOIN users ON users.id = comments.user_id
                LEFT JOIN user_meta ON user_meta.user_id = comments.user_id
                LEFT JOIN media ON media.id = user_meta.avatar_image_id
                WHERE comments.post_id = ?
                  AND comments.parent_id = ?
                  AND comments.is_deleted = 0
                  {}
                ORDER BY comments.id DESC
                LIMIT ?
                "#,
                cmd.before.map(|_| "AND comments.id < ?").unwrap_or("")
            );

            let mut query = sqlx::query_as::<
                _,
                (
                    i64,
                    Option<i64>,
                    String,
                    String,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                    i64,
                ),
            >(&sequel)
            .bind(cmd.post_id)
            .bind(parent_id);

            if let Some(before) = &cmd.before {
                query = query.bind(before)
            }

            query = query.bind(cmd.limit + 1);

            let mut comment_rows = query.fetch_all(&self.pool).await?;
            let has_more = comment_rows.len() as i64 > cmd.limit;
            if has_more {
                comment_rows.truncate(cmd.limit as usize);
            }

            let comments = comment_rows
                .into_iter()
                .map(
                    |(
                        id,
                        parent_id,
                        content,
                        created_at,
                        username,
                        display_name,
                        avatar_url,
                        user_role,
                        guest_identity,
                        direct_reply_count,
                    )| {
                        let (username, display_name, avatar_url, user_role) =
                            if guest_identity.is_some() {
                                (None, None, None, None)
                            } else {
                                (username, display_name, avatar_url, user_role)
                            };
                        Comment {
                            id,
                            parent_id,
                            direct_reply_count: Some(direct_reply_count),
                            content,
                            created_at,
                            username,
                            display_name,
                            avatar_url,
                            user_role,
                            guest_identity,
                        }
                    },
                )
                .collect();

            return Ok(CommentPage { comments, has_more });
        }

        let sequel = format!(
            r#"
            SELECT
                comments.id,
                comments.parent_id,
                comments.content,
                comments.created_at,
                users.username,
                user_meta.display_name,
                media.url,
                users.role,
                comments.guest_identity,
                (
                    SELECT COUNT(*)
                    FROM comments AS replies
                    WHERE replies.parent_id = comments.id
                      AND replies.is_deleted = 0
                ) AS direct_reply_count
            FROM comments
            LEFT JOIN users ON users.id = comments.user_id
            LEFT JOIN user_meta ON user_meta.user_id = comments.user_id
            LEFT JOIN media ON media.id = user_meta.avatar_image_id
            WHERE comments.post_id = ?
              AND comments.parent_id IS NULL
              AND comments.is_deleted = 0
              {}
            ORDER BY comments.id DESC
            LIMIT ?
            "#,
            cmd.before.map(|_| "AND comments.id < ?").unwrap_or("")
        );

        let mut query = sqlx::query_as::<
            _,
            (
                i64,
                Option<i64>,
                String,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                i64,
            ),
        >(&sequel)
        .bind(cmd.post_id);

        if let Some(before) = &cmd.before {
            query = query.bind(before)
        }

        query = query.bind(cmd.limit + 1);

        let mut comment_rows = query.fetch_all(&self.pool).await?;
        let has_more = comment_rows.len() as i64 > cmd.limit;
        if has_more {
            comment_rows.truncate(cmd.limit as usize);
        }

        let comments = comment_rows
            .into_iter()
            .map(
                |(
                    id,
                    parent_id,
                    content,
                    created_at,
                    username,
                    display_name,
                    avatar_url,
                    user_role,
                    guest_identity,
                    direct_reply_count,
                )| {
                    let (username, display_name, avatar_url, user_role) =
                        if guest_identity.is_some() {
                            (None, None, None, None)
                        } else {
                            (username, display_name, avatar_url, user_role)
                        };
                    Comment {
                        id,
                        parent_id,
                        direct_reply_count: Some(direct_reply_count),
                        content,
                        created_at,
                        username,
                        display_name,
                        avatar_url,
                        user_role,
                        guest_identity,
                    }
                },
            )
            .collect();

        Ok(CommentPage { comments, has_more })
    }
}
