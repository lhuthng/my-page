// Comment creation (authenticated and anonymous) and the view/like counters.
use sqlx::Row;

use crate::application::{
    commands::post::{
        PostNewAnynymouseCommentCommand, PostNewCommentCommand, PushNewLikeCommand,
        PushNewViewCommand,
    },
    services::post::PostService,
};
use crate::domain::errors::post::PostError;

use super::PostServiceImpl;

impl PostService for PostServiceImpl {
    async fn post_new_comment(&self, cmd: PostNewCommentCommand) -> Result<i64, PostError> {
        let content = crate::helper::string::validate_text(&cmd.content, "Comment", 2000)
            .map_err(PostError::Validation)?;
        let cmd = PostNewCommentCommand { content, ..cmd };

        let mut tx = self.pool.begin().await?;

        if let Some(parent_id) = cmd.parent_id {
            let parent_post_id: Option<i64> = sqlx::query_scalar(
                r#"
                SELECT post_id
                FROM comments
                WHERE id = ?
                "#,
            )
            .bind(parent_id)
            .fetch_optional(&mut *tx)
            .await?;

            if parent_post_id != Some(cmd.post_id) {
                return Err(PostError::PostNotFound);
            }
        }

        let id = sqlx::query_scalar(
            r#"
            INSERT INTO comments (post_id, user_id, parent_id, content, guest_identity)
            VALUES (?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(cmd.post_id)
        .bind(cmd.user_id)
        .bind(cmd.parent_id)
        .bind(&cmd.content)
        .bind(&cmd.guest_identity)
        .fetch_one(&mut *tx)
        .await?;

        if let Some(parent_id) = cmd.parent_id {
            let recipient_user_id = sqlx::query_scalar::<_, Option<i64>>(
                r#"
                SELECT user_id
                FROM comments
                WHERE id = ?
                "#,
            )
            .bind(parent_id)
            .fetch_optional(&mut *tx)
            .await?
            .flatten();

            if let Some(recipient_user_id) = recipient_user_id
                && recipient_user_id != cmd.user_id
            {
                sqlx::query(
                    r#"
                    INSERT INTO notifications (
                        recipient_user_id,
                        actor_user_id,
                        post_id,
                        comment_id,
                        type
                    )
                    VALUES (?, ?, ?, ?, 'reply')
                    "#,
                )
                .bind(recipient_user_id)
                .bind(cmd.user_id)
                .bind(cmd.post_id)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            }
        }

        // Notify any @mentioned registered users
        let mentioned: HashSet<String> = MENTION_RE
            .captures_iter(&cmd.content)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .collect();
        if !mentioned.is_empty() {
            let placeholders = mentioned.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
            let sql = format!("SELECT id FROM users WHERE username IN ({})", placeholders);
            let mut q = sqlx::query_scalar::<_, i64>(&sql);
            for username in &mentioned {
                q = q.bind(username);
            }
            let mentioned_ids: Vec<i64> = q.fetch_all(&mut *tx).await?;
            for mentioned_id in mentioned_ids {
                if mentioned_id != cmd.user_id {
                    sqlx::query(
                        r#"
                        INSERT INTO notifications (recipient_user_id, actor_user_id, post_id, comment_id, type)
                        VALUES (?, ?, ?, ?, 'mention')
                        "#,
                    )
                    .bind(mentioned_id)
                    .bind(cmd.user_id)
                    .bind(cmd.post_id)
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
                }
            }
        }

        tx.commit().await?;
        Ok(id)
    }
    async fn post_new_anonymous_comment(
        &self,
        cmd: PostNewAnynymouseCommentCommand,
    ) -> Result<i64, PostError> {
        let content = crate::helper::string::validate_text(&cmd.content, "Comment", 2000)
            .map_err(PostError::Validation)?;
        let cmd = PostNewAnynymouseCommentCommand { content, ..cmd };
        let mut tx = self.pool.begin().await?;

        if let Some(parent_id) = cmd.parent_id {
            let parent_post_id: Option<i64> = sqlx::query_scalar(
                r#"
                SELECT post_id
                FROM comments
                WHERE id = ?
                "#,
            )
            .bind(parent_id)
            .fetch_optional(&mut *tx)
            .await?;

            if parent_post_id != Some(cmd.post_id) {
                return Err(PostError::PostNotFound);
            }
        }

        let id = sqlx::query_scalar(
            r#"
            INSERT INTO comments (post_id, parent_id, content, guest_identity)
            VALUES (?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(cmd.post_id)
        .bind(cmd.parent_id)
        .bind(&cmd.content)
        .bind(&cmd.guest_identity)
        .fetch_one(&mut *tx)
        .await?;

        // Anonymous commenter CAN write @mentions; notify matched registered users
        // (actor_user_id is NULL since there is no authenticated user)
        let mentioned: HashSet<String> = MENTION_RE
            .captures_iter(&cmd.content)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .collect();
        if !mentioned.is_empty() {
            let placeholders = mentioned.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
            let sql = format!("SELECT id FROM users WHERE username IN ({})", placeholders);
            let mut q = sqlx::query_scalar::<_, i64>(&sql);
            for username in &mentioned {
                q = q.bind(username);
            }
            let mentioned_ids: Vec<i64> = q.fetch_all(&mut *tx).await?;
            for mentioned_id in mentioned_ids {
                sqlx::query(
                    r#"
                    INSERT INTO notifications (recipient_user_id, actor_user_id, post_id, comment_id, type)
                    VALUES (?, NULL, ?, ?, 'mention')
                    "#,
                )
                .bind(mentioned_id)
                .bind(cmd.post_id)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(id)
    }
    async fn push_new_view(&self, cmd: PushNewViewCommand) -> Result<(), PostError> {
        sqlx::query(
            r#"
            INSERT INTO post_stats (post_id, views, updated_at)
            VALUES (?, 1, CURRENT_TIMESTAMP)
            ON CONFLICT(post_id) DO UPDATE
              SET views = post_stats.views + 1,
                  updated_at = CURRENT_TIMESTAMP
        "#,
        )
        .bind(cmd.post_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn push_new_like(&self, cmd: PushNewLikeCommand) -> Result<(), PostError> {
        sqlx::query(
            r#"
            INSERT INTO post_stats (post_id, likes, updated_at)
            VALUES (?, 1, CURRENT_TIMESTAMP)
            ON CONFLICT(post_id) DO UPDATE
              SET likes = post_stats.likes + 1,
                  updated_at = CURRENT_TIMESTAMP
        "#,
        )
        .bind(cmd.post_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

}
