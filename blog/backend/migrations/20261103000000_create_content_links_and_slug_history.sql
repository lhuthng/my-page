-- Link registry: one row per distinct reference a post body makes to another
-- entity. Target title/slug are snapshots so a reference can still be rendered
-- (and a deleted target can be shown as a tombstone) after the target moves.
-- There is deliberately no FK on target_id: links must survive a target delete.
CREATE TABLE IF NOT EXISTS content_links (
    source_post_id INTEGER NOT NULL,
    field TEXT NOT NULL DEFAULT 'body',
    ref_key TEXT NOT NULL,
    target_kind TEXT NOT NULL CHECK (target_kind IN ('media', 'content', 'series')),
    target_id INTEGER,
    target_title TEXT,
    target_slug TEXT,
    PRIMARY KEY (source_post_id, field, ref_key),
    FOREIGN KEY (source_post_id) REFERENCES posts(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_content_links_target
ON content_links(target_kind, target_id);

-- Old slugs retained after a rename, keyed by entity so the 404 path can look
-- up where a moved post/project/game/series went. A slug that is live again
-- wins over this table because lookups check the live row first.
CREATE TABLE IF NOT EXISTS slug_history (
    entity_kind TEXT NOT NULL CHECK (entity_kind IN ('post', 'project', 'game', 'series')),
    entity_id INTEGER NOT NULL,
    slug TEXT NOT NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (entity_kind, slug)
);

CREATE INDEX IF NOT EXISTS idx_slug_history_entity
ON slug_history(entity_kind, entity_id);
