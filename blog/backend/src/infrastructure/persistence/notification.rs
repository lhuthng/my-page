// Notification persistence: in-app rows written with a comment, plus the read
// queries behind the bell, the notifications page, and email preferences.
use std::collections::BTreeMap;

use sqlx::{FromRow, Sqlite, SqlitePool, Transaction};

use crate::domain::entities::notification::{CommentRecipient, NotificationKind};
use crate::infrastructure::persistence::post::links::mentioned_usernames;

/// Resolve everyone who should hear about a new comment, write one in-app row
/// per recipient, and hand the list back so the caller can send email from the
/// same resolution. Runs inside the comment transaction.
pub async fn record_comment_notifications(
    tx: &mut Transaction<'_, Sqlite>,
    post_id: i64,
    comment_id: i64,
    actor_user_id: Option<i64>,
    parent_id: Option<i64>,
    content: &str,
) -> Result<Vec<CommentRecipient>, sqlx::Error> {
    let post_author_id = sqlx::query_scalar::<_, Option<i64>>(
        r#"
        SELECT user_id
        FROM posts
        WHERE id = ?
        "#,
    )
    .bind(post_id)
    .fetch_optional(&mut **tx)
    .await?
    .flatten();

    let parent_author_id = match parent_id {
        Some(parent_id) => sqlx::query_scalar::<_, Option<i64>>(
            r#"
            SELECT user_id
            FROM comments
            WHERE id = ?
            "#,
        )
        .bind(parent_id)
        .fetch_optional(&mut **tx)
        .await?
        .flatten(),
        None => None,
    };

    let usernames = mentioned_usernames(content);
    let mentioned_ids = if usernames.is_empty() {
        Vec::new()
    } else {
        let placeholders = usernames.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!("SELECT id FROM users WHERE username IN ({})", placeholders);
        let mut query = sqlx::query_scalar::<_, i64>(&sql);
        for username in &usernames {
            query = query.bind(username);
        }
        query.fetch_all(&mut **tx).await?
    };

    let recipients = merge_recipients(
        actor_user_id,
        post_author_id,
        parent_author_id,
        mentioned_ids,
    );

    for recipient in &recipients {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO notifications (
                recipient_user_id,
                actor_user_id,
                post_id,
                comment_id,
                type
            )
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(recipient.user_id)
        .bind(actor_user_id)
        .bind(post_id)
        .bind(comment_id)
        .bind(recipient.kind.as_str())
        .execute(&mut **tx)
        .await?;
    }

    Ok(recipients)
}

fn merge_recipients(
    actor_user_id: Option<i64>,
    post_author_id: Option<i64>,
    parent_author_id: Option<i64>,
    mentioned_ids: Vec<i64>,
) -> Vec<CommentRecipient> {
    let mut by_user: BTreeMap<i64, NotificationKind> = BTreeMap::new();

    let mut add = |user_id: i64, kind: NotificationKind| {
        if Some(user_id) == actor_user_id {
            return;
        }
        by_user
            .entry(user_id)
            .and_modify(|current| {
                if kind.priority() < current.priority() {
                    *current = kind;
                }
            })
            .or_insert(kind);
    };

    if let Some(author_id) = post_author_id {
        add(author_id, NotificationKind::PostComment);
    }
    if let Some(author_id) = parent_author_id {
        add(author_id, NotificationKind::Reply);
    }
    for user_id in mentioned_ids {
        add(user_id, NotificationKind::Mention);
    }

    by_user
        .into_iter()
        .map(|(user_id, kind)| CommentRecipient { user_id, kind })
        .collect()
}

#[derive(Debug, FromRow)]
pub struct NotificationRow {
    pub id: i64,
    pub kind: String,
    pub created_at: String,
    pub read_at: Option<String>,
    pub actor_username: Option<String>,
    pub actor_display_name: Option<String>,
    pub actor_avatar_url: Option<String>,
    pub post_title: String,
    pub post_slug: String,
    pub post_kind: String,
    pub comment_id: i64,
    pub comment_content: String,
}

/// One page of a recipient's notifications, newest first. Fetches one extra row
/// so the caller can report `has_more` without a count query.
pub async fn list_notifications(
    pool: &SqlitePool,
    recipient_user_id: i64,
    before: Option<i64>,
    limit: i64,
) -> Result<Vec<NotificationRow>, sqlx::Error> {
    sqlx::query_as::<_, NotificationRow>(
        r#"
        SELECT n.id,
               n.type AS kind,
               n.created_at,
               n.read_at,
               a.username AS actor_username,
               COALESCE(am.display_name, a.username) AS actor_display_name,
               'media/i/' || m.short_name AS actor_avatar_url,
               p.title AS post_title,
               p.slug AS post_slug,
               p.content_kind AS post_kind,
               c.id AS comment_id,
               c.content AS comment_content
        FROM notifications n
        JOIN posts p ON p.id = n.post_id
        JOIN comments c ON c.id = n.comment_id
        LEFT JOIN users a ON a.id = n.actor_user_id
        LEFT JOIN user_meta am ON am.user_id = a.id
        LEFT JOIN media m ON m.id = am.avatar_image_id
        WHERE n.recipient_user_id = ?
          AND (? IS NULL OR n.id < ?)
        ORDER BY n.id DESC
        LIMIT ?
        "#,
    )
    .bind(recipient_user_id)
    .bind(before)
    .bind(before)
    .bind(limit + 1)
    .fetch_all(pool)
    .await
}

pub async fn count_unread(pool: &SqlitePool, recipient_user_id: i64) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM notifications
        WHERE recipient_user_id = ? AND read_at IS NULL
        "#,
    )
    .bind(recipient_user_id)
    .fetch_one(pool)
    .await
}

/// Returns false when the row does not exist or belongs to someone else, so the
/// handler can answer 404 without leaking ids.
pub async fn mark_read(
    pool: &SqlitePool,
    recipient_user_id: i64,
    notification_id: i64,
) -> Result<bool, sqlx::Error> {
    let owner: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT recipient_user_id
        FROM notifications
        WHERE id = ?
        "#,
    )
    .bind(notification_id)
    .fetch_optional(pool)
    .await?;

    if owner != Some(recipient_user_id) {
        return Ok(false);
    }

    sqlx::query(
        r#"
        UPDATE notifications
        SET read_at = CURRENT_TIMESTAMP
        WHERE id = ? AND read_at IS NULL
        "#,
    )
    .bind(notification_id)
    .execute(pool)
    .await?;

    Ok(true)
}

pub async fn mark_all_read(pool: &SqlitePool, recipient_user_id: i64) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE notifications
        SET read_at = CURRENT_TIMESTAMP
        WHERE recipient_user_id = ? AND read_at IS NULL
        "#,
    )
    .bind(recipient_user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

#[derive(Debug, FromRow)]
pub struct NotificationPreferencesRow {
    pub reply_email: i64,
    pub mention_email: i64,
    pub post_comment_email: i64,
    pub prompt_dismissed: i64,
    pub show_prompt: i64,
}

pub async fn get_preferences(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Option<NotificationPreferencesRow>, sqlx::Error> {
    sqlx::query_as::<_, NotificationPreferencesRow>(
        r#"
        SELECT notify_reply_email AS reply_email,
               notify_mention_email AS mention_email,
               notify_post_comment_email AS post_comment_email,
               notification_prompt_dismissed_at IS NOT NULL AS prompt_dismissed,
               (
                   notification_prompt_dismissed_at IS NULL
                   AND notify_reply_email = 0
                   AND notify_mention_email = 0
                   AND notify_post_comment_email = 0
                   AND created_at >= datetime('now', '-7 days')
               ) AS show_prompt
        FROM users
        WHERE id = ?
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn update_preferences(
    pool: &SqlitePool,
    user_id: i64,
    reply_email: Option<bool>,
    mention_email: Option<bool>,
    post_comment_email: Option<bool>,
    dismiss_prompt: bool,
) -> Result<Option<NotificationPreferencesRow>, sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE users
        SET notify_reply_email = COALESCE(?, notify_reply_email),
            notify_mention_email = COALESCE(?, notify_mention_email),
            notify_post_comment_email = COALESCE(?, notify_post_comment_email),
            notification_prompt_dismissed_at = CASE
                WHEN ? = 1 THEN CURRENT_TIMESTAMP
                ELSE notification_prompt_dismissed_at
            END
        WHERE id = ?
        "#,
    )
    .bind(reply_email.map(i64::from))
    .bind(mention_email.map(i64::from))
    .bind(post_comment_email.map(i64::from))
    .bind(i64::from(dismiss_prompt))
    .bind(user_id)
    .execute(pool)
    .await?;

    get_preferences(pool, user_id).await
}

#[cfg(test)]
mod tests {
    use super::merge_recipients;
    use crate::domain::entities::notification::NotificationKind;

    #[test]
    fn drops_the_actor_and_collapses_duplicates_by_priority() {
        // 1 is the post author and parent author and mentioned: reply wins.
        // 2 is the actor and must disappear even though mentioned twice.
        // 3 is only mentioned.
        let recipients = merge_recipients(Some(2), Some(1), Some(1), vec![1, 2, 3, 3]);

        assert_eq!(recipients.len(), 2);
        assert_eq!(recipients[0].user_id, 1);
        assert_eq!(recipients[0].kind, NotificationKind::Reply);
        assert_eq!(recipients[1].user_id, 3);
        assert_eq!(recipients[1].kind, NotificationKind::Mention);
    }

    #[test]
    fn mention_beats_a_broad_post_comment_and_parent_wins_over_both() {
        let only_post = merge_recipients(None, Some(1), None, vec![]);
        assert_eq!(only_post[0].kind, NotificationKind::PostComment);

        let mentioned = merge_recipients(None, Some(1), None, vec![1]);
        assert_eq!(mentioned[0].kind, NotificationKind::Mention);

        let reply = merge_recipients(None, Some(1), Some(1), vec![1]);
        assert_eq!(reply[0].kind, NotificationKind::Reply);
    }

    #[test]
    fn anonymous_actor_never_suppresses_a_recipient() {
        let recipients = merge_recipients(None, Some(1), Some(2), vec![3]);
        assert_eq!(recipients.len(), 3);
    }
}
