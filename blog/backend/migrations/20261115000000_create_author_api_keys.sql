-- Author API keys: a secret that unlocks exactly one audiobook for scripting.
--
-- Scope is the point of this table. `sync_keys` grants a pull of the whole
-- database and every media file, so it cannot be reused here: an author key is
-- bound to one book, and the guard compares that id with the one in the URL.
-- Only the SHA-256 of the secret is stored; the secret itself is returned once
-- at creation and never again, like the sync and password-reset tokens.
--
-- `user_id` is the book's creator at issue time. The guard re-checks it against
-- `audiobooks.user_id` on every request, so a key stops working if the book is
-- ever reassigned. Both foreign keys cascade: deleting a book deletes its keys.
--
-- `expires_at` is nullable and NULL means "until revoked". The API is opt-in at
-- the deployment level too (AUTHOR_API_ENABLED); this table only describes
-- keys, not whether the feature is switched on.
CREATE TABLE IF NOT EXISTS author_api_keys (
    id INTEGER PRIMARY KEY,
    audiobook_id INTEGER NOT NULL REFERENCES audiobooks(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    label TEXT NOT NULL DEFAULT '',
    hint TEXT NOT NULL DEFAULT '',
    scopes TEXT NOT NULL DEFAULT 'read,write'
        CHECK (scopes IN ('read', 'read,write')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT,
    revoked_at TEXT,
    last_used_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_author_api_keys_audiobook
    ON author_api_keys (audiobook_id);
