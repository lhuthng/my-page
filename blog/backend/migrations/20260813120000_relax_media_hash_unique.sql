PRAGMA foreign_keys=off;

-- Recreate media without UNIQUE on hash. Files are content-addressed on disk
-- (path derived from sha256), so the same bytes may legitimately be registered
-- under multiple short_names. bulk_upload now reuses existing files and upserts
-- by short_name; a UNIQUE(hash) constraint would reject that second row.

CREATE TABLE media_new (
    id INTEGER PRIMARY KEY,
    hash TEXT NOT NULL,
    short_name TEXT NOT NULL UNIQUE,
    file_name TEXT NOT NULL,
    file_type TEXT NOT NULL,
    url TEXT NOT NULL,
    size INTEGER NOT NULL,
    description TEXT,
    uploader_id INTEGER,
    use_count INTEGER DEFAULT 0,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (uploader_id) REFERENCES users(id) ON DELETE SET NULL
);

INSERT INTO media_new (
    id, hash, short_name, file_name, file_type, url, size, description,
    uploader_id, use_count, created_at, updated_at
)
SELECT
    id, hash, short_name, file_name, file_type, url, size, description,
    uploader_id, use_count, created_at, updated_at
FROM media;

DROP TABLE media;
ALTER TABLE media_new RENAME TO media;

CREATE INDEX idx_media_hash ON media(hash);
CREATE UNIQUE INDEX idx_media_short_name ON media(short_name);

PRAGMA foreign_keys=on;
