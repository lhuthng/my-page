CREATE TABLE IF NOT EXISTS notifications (
    id INTEGER PRIMARY KEY,

    recipient_user_id INTEGER NOT NULL,
    actor_user_id INTEGER,
    post_id INTEGER NOT NULL,
    comment_id INTEGER NOT NULL,

    type TEXT NOT NULL,
    is_read INTEGER DEFAULT 0,

    created_at TEXT DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (recipient_user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (actor_user_id) REFERENCES users(id) ON DELETE SET NULL,
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE,
    FOREIGN KEY (comment_id) REFERENCES comments(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_notifications_recipient_user_id ON notifications(recipient_user_id);
CREATE INDEX IF NOT EXISTS idx_notifications_is_read ON notifications(is_read);
CREATE INDEX IF NOT EXISTS idx_notifications_created_at ON notifications(created_at);