// Comment creation (authenticated and anonymous) and the view/like counters.
use crate::application::commands::post::{
    PostNewAnynymouseCommentCommand, PostNewCommentCommand, PushNewLikeCommand, PushNewViewCommand,
};
use crate::domain::entities::notification::CommentCreated;
use crate::domain::errors::post::PostError;
use crate::infrastructure::persistence::notification::record_comment_notifications;

use super::PostServiceImpl;

impl PostServiceImpl {
    pub(super) async fn post_new_comment(
        &self,
        cmd: PostNewCommentCommand,
    ) -> Result<CommentCreated, PostError> {
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

        let recipients = record_comment_notifications(
            &mut tx,
            cmd.post_id,
            id,
            Some(cmd.user_id),
            cmd.parent_id,
            &cmd.content,
        )
        .await?;

        tx.commit().await?;
        Ok(CommentCreated {
            comment_id: id,
            recipients,
        })
    }
    pub(super) async fn post_new_anonymous_comment(
        &self,
        cmd: PostNewAnynymouseCommentCommand,
    ) -> Result<CommentCreated, PostError> {
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

        // Anonymous commenter CAN write @mentions; notify matched registered
        // users with a NULL actor (there is no authenticated user).
        let recipients = record_comment_notifications(
            &mut tx,
            cmd.post_id,
            id,
            None,
            cmd.parent_id,
            &cmd.content,
        )
        .await?;

        tx.commit().await?;
        Ok(CommentCreated {
            comment_id: id,
            recipients,
        })
    }
    pub(super) async fn push_new_view(&self, cmd: PushNewViewCommand) -> Result<(), PostError> {
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

    pub(super) async fn push_new_like(&self, cmd: PushNewLikeCommand) -> Result<(), PostError> {
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
