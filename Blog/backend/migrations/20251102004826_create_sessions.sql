-- Create simple sessions table
CREATE TABLE IF NOT EXISTS sessions (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL,
    token_hash TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT NOT NULL,
    revoked_at TEXT,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);
