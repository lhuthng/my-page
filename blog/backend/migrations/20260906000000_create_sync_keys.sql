-- Sync keys authorize a one-directional data pull (prod -> dev) through the
-- /sync endpoints. Only the SHA-256 hash of the key is stored; the secret is
-- shown once at creation, like the email-verification and password-reset
-- tokens. mode is constrained to 'pull' on purpose: pushing data back to
-- production needs its own, more cautious flow.
CREATE TABLE IF NOT EXISTS sync_keys (
    id INTEGER PRIMARY KEY,
    token_hash TEXT NOT NULL UNIQUE,
    label TEXT NOT NULL DEFAULT '',
    mode TEXT NOT NULL DEFAULT 'pull' CHECK (mode IN ('pull')),
    created_by INTEGER,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT NOT NULL,
    revoked_at TEXT,
    last_used_at TEXT,
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_sync_keys_expires_at ON sync_keys (expires_at);
