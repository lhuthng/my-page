CREATE TABLE IF NOT EXISTS newsletter_subscribers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email TEXT NOT NULL UNIQUE COLLATE NOCASE,
    status TEXT NOT NULL DEFAULT 'pending',
    confirm_token_hash TEXT,
    confirm_token_expires_at TEXT,
    confirm_sent_at TEXT,
    unsubscribe_token_hash TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    confirmed_at TEXT,
    unsubscribed_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_newsletter_subscribers_status ON newsletter_subscribers (status);

CREATE TABLE IF NOT EXISTS newsletter_campaigns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    post_id INTEGER REFERENCES posts (id) ON DELETE SET NULL,
    subject TEXT NOT NULL,
    body_text TEXT NOT NULL,
    body_html TEXT NOT NULL,
    sent_by_user_id INTEGER REFERENCES users (id) ON DELETE SET NULL,
    recipient_count INTEGER NOT NULL DEFAULT 0,
    success_count INTEGER NOT NULL DEFAULT 0,
    failure_count INTEGER NOT NULL DEFAULT 0,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_newsletter_campaigns_post_id_unique
ON newsletter_campaigns (post_id) WHERE post_id IS NOT NULL;
