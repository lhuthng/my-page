-- Media table
CREATE TABLE IF NOT EXISTS media (
    id INTEGER PRIMARY KEY,
    hash TEXT NOT NULL UNIQUE,
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
CREATE UNIQUE INDEX IF NOT EXISTS idx_media_hash ON media(hash);
CREATE UNIQUE INDEX IF NOT EXISTS idx_media_short_name ON media(short_name);

-- Media Aliases table
CREATE TABLE IF NOT EXISTS media_aliases (
    id INTEGER PRIMARY KEY,
    media_id INTEGER,
    alias TEXT NOT NULL,
    FOREIGN KEY (media_id) REFERENCES media(id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_media_aliases_alias ON media_aliases(alias);
