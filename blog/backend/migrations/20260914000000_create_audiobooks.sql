-- Audiobooks: an ordered collection of audio tracks with book-level metadata.
--
-- Design notes:
--   * Tracks reference the existing `media` table instead of storing their own
--     file paths. Media is content-addressed and already served by the
--     `/media/i/{short_name}` handler with HTTP Range support and a streaming
--     body (ReaderStream), which is exactly what continuous audio playback
--     needs: the browser seeks and buffers without the whole file ever being
--     resident in memory on either the client or the server.
--   * Tags live in `audiobook_tags`, a table dedicated to audiobooks. They are
--     deliberately NOT the global `tags` table so audiobook taxonomy (narrator
--     genres, language, etc.) cannot leak into post/project tag listings and
--     vice versa.
--   * `translator` is a first-class column: it is required metadata for the
--     translated works this collection is built around, and is separate from
--     the owning author account (`user_id`).

CREATE TABLE IF NOT EXISTS audiobooks (
    id INTEGER PRIMARY KEY,

    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    title TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    description TEXT,
    translator TEXT,

    cover_image_id INTEGER REFERENCES media(id) ON DELETE SET NULL,

    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'published', 'archived')),

    published_at TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_audiobooks_user_id ON audiobooks (user_id);
CREATE INDEX IF NOT EXISTS idx_audiobooks_status_published
    ON audiobooks (status, published_at DESC);

-- One row per chapter/track. `number` is the playback order inside the
-- audiobook and is re-sequenced transactionally on insert/remove/reorder so it
-- is always contiguous starting at 1.
CREATE TABLE IF NOT EXISTS audiobook_tracks (
    id INTEGER PRIMARY KEY,

    audiobook_id INTEGER NOT NULL REFERENCES audiobooks(id) ON DELETE CASCADE,
    media_id INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,

    title TEXT NOT NULL,
    number INTEGER NOT NULL,

    -- Probed in the browser from the decoded audio element's metadata, then
    -- sent with the upload. Nullable because a probe can legitimately fail for
    -- exotic encodings; the player falls back to the element's own duration.
    duration_seconds INTEGER,

    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_audiobook_tracks_order
    ON audiobook_tracks (audiobook_id, number);
CREATE INDEX IF NOT EXISTS idx_audiobook_tracks_media ON audiobook_tracks (media_id);

-- Audiobook-only tag vocabulary. `slug` is unique so tag pages and lookups are
-- stable and case-insensitive duplicates collapse onto one row.
CREATE TABLE IF NOT EXISTS audiobook_tags (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS audiobook_tag_links (
    audiobook_id INTEGER NOT NULL REFERENCES audiobooks(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES audiobook_tags(id) ON DELETE CASCADE,

    PRIMARY KEY (audiobook_id, tag_id)
);

CREATE INDEX IF NOT EXISTS idx_audiobook_tag_links_tag ON audiobook_tag_links (tag_id);
