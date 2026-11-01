-- In-app notifications become read/write, deduplicated, and preference-aware.

-- A read timestamp replaces the unused flag. The old index must go first,
-- because SQLite refuses to drop a column an index still references.
DROP INDEX IF EXISTS idx_notifications_is_read;
ALTER TABLE notifications DROP COLUMN is_read;
ALTER TABLE notifications ADD COLUMN read_at TEXT;

-- Historically a user could receive both a reply row and a mention row for the
-- same comment. Keep the most specific one before the constraint lands.
DELETE FROM notifications
WHERE id NOT IN (
    SELECT id FROM (
        SELECT id,
               ROW_NUMBER() OVER (
                   PARTITION BY recipient_user_id, comment_id
                   ORDER BY CASE type
                                WHEN 'reply' THEN 0
                                WHEN 'mention' THEN 1
                                ELSE 2
                            END,
                            id
               ) AS rn
        FROM notifications
    )
    WHERE rn = 1
);

-- One notification per recipient per comment. Writers rely on INSERT OR IGNORE.
CREATE UNIQUE INDEX IF NOT EXISTS idx_notifications_recipient_comment
    ON notifications(recipient_user_id, comment_id);

-- Badge query: count unread rows for one recipient.
CREATE INDEX IF NOT EXISTS idx_notifications_unread
    ON notifications(recipient_user_id, id DESC)
    WHERE read_at IS NULL;

-- Existing rows predate the in-app reader. Marking them read avoids a badge
-- full of months-old events on the first deploy.
UPDATE notifications SET read_at = created_at WHERE read_at IS NULL;

-- Email notification preferences. New accounts default to off and choose at
-- signup; existing accounts keep receiving the mail they already receive.
ALTER TABLE users ADD COLUMN notify_reply_email INTEGER NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN notify_mention_email INTEGER NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN notify_post_comment_email INTEGER NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN notification_prompt_dismissed_at TEXT;

UPDATE users
SET notify_reply_email = 1,
    notify_mention_email = 1,
    notify_post_comment_email = 1;
