-- Soft-delete (trash) for posts/projects/games: flagged rows stay 7 days for rollback,
-- author sees own trash, admin sees all and can force-purge. Hard purge is
-- delayed so delegated games keep their FK until then and show a tombstone.

ALTER TABLE posts ADD COLUMN deleted_at TEXT;
ALTER TABLE posts ADD COLUMN deletion_reason TEXT CHECK (deletion_reason IN ('user_request','dmca','moderation','replaced','other'));
ALTER TABLE posts ADD COLUMN deletion_detail TEXT;
ALTER TABLE posts ADD COLUMN deleted_by INTEGER REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE posts ADD COLUMN scheduled_purge_at TEXT;
ALTER TABLE posts ADD COLUMN prev_status TEXT;

CREATE INDEX IF NOT EXISTS idx_posts_deleted_at ON posts(deleted_at);
CREATE INDEX IF NOT EXISTS idx_posts_scheduled_purge ON posts(scheduled_purge_at) WHERE deleted_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_posts_deleted_by ON posts(deleted_by);

-- Tombstone log so that after hard purge (DELETE FROM posts cascades games)
-- we can still show "Game unavailable — DMCA" on delegated projects.
CREATE TABLE IF NOT EXISTS game_deletion_log (
    id INTEGER PRIMARY KEY,
    game_id INTEGER NOT NULL,
    slug TEXT NOT NULL,
    title TEXT NOT NULL,
    reason TEXT NOT NULL CHECK (reason IN ('user_request','dmca','moderation','replaced','other')),
    detail TEXT,
    deleted_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    delegated_project_ids TEXT, -- JSON array of project ids that were delegating at delete time
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_game_deletion_log_game_id ON game_deletion_log(game_id);
CREATE INDEX IF NOT EXISTS idx_game_deletion_log_slug ON game_deletion_log(slug);
