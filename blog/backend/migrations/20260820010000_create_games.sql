-- Games: a standalone playable entity backed by a `posts` row
-- (`content_kind = 'game'`). A game holds the launcher (html5/jsdos/v86)
-- plus "many bodies" (instruction / cheatcode / story) and related games.
-- Projects may delegate to a game (demo_type = 'game') but never carry a
-- launcher themselves anymore.

-- ---------------------------------------------------------------------------
-- 1. Games table
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS games (
    id INTEGER PRIMARY KEY,
    post_id INTEGER NOT NULL UNIQUE,

    -- The launcher that plays this game.
    launcher_type TEXT NOT NULL DEFAULT 'html5'
        CHECK (launcher_type IN ('html5', 'webgl', 'jsdos', 'v86', 'embed', 'download', 'video')),
    demo_width TEXT,
    demo_height TEXT,
    demo_url TEXT,

    -- "Many bodies" instead of a single post body.
    instruction TEXT NOT NULL DEFAULT '',
    cheatcode TEXT NOT NULL DEFAULT '',
    story TEXT NOT NULL DEFAULT '',

    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_games_post_id ON games(post_id);

-- ---------------------------------------------------------------------------
-- 2. js-dos bundles (moved from project_jsdos_bundles)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS game_jsdos_bundles (
    game_id INTEGER PRIMARY KEY,
    storage_key TEXT NOT NULL UNIQUE,
    original_file_name TEXT NOT NULL,
    size_bytes INTEGER NOT NULL CHECK (size_bytes > 0),
    sha256 TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS game_jsdos_upload_sessions (
    id TEXT PRIMARY KEY,
    game_id INTEGER NOT NULL,
    uploader_id INTEGER NOT NULL,
    original_file_name TEXT NOT NULL,
    expected_size_bytes INTEGER NOT NULL CHECK (expected_size_bytes > 0 AND expected_size_bytes <= 524288000),
    received_size_bytes INTEGER NOT NULL DEFAULT 0 CHECK (received_size_bytes >= 0),
    next_chunk_index INTEGER NOT NULL DEFAULT 0 CHECK (next_chunk_index >= 0),
    chunk_size_bytes INTEGER NOT NULL DEFAULT 8388608 CHECK (chunk_size_bytes > 0),
    temp_storage_key TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'completed', 'aborted', 'expired')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT NOT NULL,
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
    FOREIGN KEY (uploader_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_game_jsdos_upload_sessions_game
    ON game_jsdos_upload_sessions(game_id);
CREATE INDEX idx_game_jsdos_upload_sessions_expiry
    ON game_jsdos_upload_sessions(status, expires_at);

-- ---------------------------------------------------------------------------
-- 3. v86 games (moved from project_v86_games)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS game_v86_games (
    game_id INTEGER PRIMARY KEY,
    system_version_id INTEGER NOT NULL,
    manifest_text TEXT NOT NULL,
    manifest_sha256 TEXT NOT NULL,
    -- The manifest is split into two hashes so launcher-only tweaks
    -- (mouse speed, revert_mouse_y, delay_ms) do NOT invalidate snapshots.
    launcher_config_sha256 TEXT NOT NULL DEFAULT '',
    game_config_sha256 TEXT NOT NULL DEFAULT '',
    original_file_name TEXT NOT NULL,
    zip_storage_key TEXT NOT NULL UNIQUE,
    zip_size_bytes INTEGER NOT NULL CHECK (zip_size_bytes > 0),
    zip_sha256 TEXT NOT NULL,
    iso_storage_key TEXT NOT NULL UNIQUE,
    iso_size_bytes INTEGER NOT NULL CHECK (iso_size_bytes > 0),
    iso_sha256 TEXT NOT NULL,
    disk_storage_key TEXT,
    disk_sha256 TEXT,
    disk_size_bytes INTEGER,
    chunk_size_bytes INTEGER NOT NULL CHECK (chunk_size_bytes > 0),
    chunk_count INTEGER NOT NULL CHECK (chunk_count > 0),
    artifact_revision INTEGER NOT NULL DEFAULT 1 CHECK (artifact_revision > 0),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
    FOREIGN KEY (system_version_id) REFERENCES v86_system_versions(id) ON DELETE RESTRICT
);

CREATE INDEX idx_game_v86_games_system_version
    ON game_v86_games(system_version_id);

CREATE TABLE IF NOT EXISTS game_v86_variants (
    game_id INTEGER NOT NULL,
    variant_index INTEGER NOT NULL CHECK (variant_index > 0),
    name TEXT NOT NULL,
    exe TEXT NOT NULL,
    args TEXT NOT NULL DEFAULT '',
    iso_storage_key TEXT NOT NULL,
    iso_size_bytes INTEGER NOT NULL CHECK (iso_size_bytes > 0),
    iso_sha256 TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (game_id, variant_index),
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE
);

CREATE INDEX idx_game_v86_variants_iso ON game_v86_variants(iso_sha256);

-- ---------------------------------------------------------------------------
-- 4. v86 snapshots (moved from project_v86_snapshots)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS game_v86_snapshots (
    game_id INTEGER NOT NULL,
    variant_index INTEGER NOT NULL DEFAULT 0,
    system_version_id INTEGER NOT NULL,
    game_disk_sha256 TEXT NOT NULL,
    iso_sha256 TEXT NOT NULL DEFAULT '',
    storage_key TEXT NOT NULL,
    size_bytes INTEGER NOT NULL CHECK (size_bytes > 0),
    raw_size_bytes INTEGER NOT NULL CHECK (raw_size_bytes > 0),
    sha256 TEXT NOT NULL,
    state_version INTEGER NOT NULL CHECK (state_version > 0),
    topology_version INTEGER NOT NULL DEFAULT 0,
    memory_size INTEGER NOT NULL CHECK (memory_size > 0),
    vga_memory_size INTEGER NOT NULL CHECK (vga_memory_size > 0),
    created_by INTEGER,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (game_id, variant_index),
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
    FOREIGN KEY (system_version_id) REFERENCES v86_system_versions(id) ON DELETE RESTRICT,
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS game_v86_snapshot_upload_sessions (
    id TEXT PRIMARY KEY,
    uploader_id INTEGER NOT NULL,
    game_id INTEGER NOT NULL,
    system_version_id INTEGER NOT NULL,
    game_disk_sha256 TEXT NOT NULL,
    iso_sha256 TEXT NOT NULL DEFAULT '',
    raw_size_bytes INTEGER NOT NULL CHECK (raw_size_bytes > 0),
    sha256 TEXT NOT NULL,
    state_version INTEGER NOT NULL CHECK (state_version > 0),
    topology_version INTEGER NOT NULL DEFAULT 0,
    memory_size INTEGER NOT NULL CHECK (memory_size > 0),
    vga_memory_size INTEGER NOT NULL CHECK (vga_memory_size > 0),
    expected_size_bytes INTEGER NOT NULL CHECK (expected_size_bytes > 0),
    received_size_bytes INTEGER NOT NULL DEFAULT 0,
    next_chunk_index INTEGER NOT NULL DEFAULT 0,
    upload_chunk_size_bytes INTEGER NOT NULL DEFAULT 8388608,
    temp_storage_key TEXT NOT NULL UNIQUE,
    r2_upload_id TEXT,
    r2_part_etags TEXT,
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'ready', 'consumed', 'failed', 'aborted', 'expired')),
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT NOT NULL,
    FOREIGN KEY (uploader_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
    FOREIGN KEY (system_version_id) REFERENCES v86_system_versions(id) ON DELETE CASCADE
);

CREATE INDEX idx_game_v86_snapshot_sessions_uploader
    ON game_v86_snapshot_upload_sessions (uploader_id, status);

-- ---------------------------------------------------------------------------
-- 5. v86 saves (moved from v86_saves). Saves are durable: they are never
-- auto-deleted on disk change. They become "stale" (not served) and a mod
-- must explicitly approve their removal.
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS game_v86_saves (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    storage_key TEXT NOT NULL UNIQUE,
    size_bytes INTEGER NOT NULL CHECK (size_bytes > 0),
    sha256 TEXT NOT NULL,
    -- The disk this save was captured against. When it no longer matches the
    -- game's current disk, the save is stale and not served.
    game_disk_sha256 TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(game_id, user_id)
);

-- ---------------------------------------------------------------------------
-- 6. Related games
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS related_games (
    game_id INTEGER NOT NULL,
    related_game_id INTEGER NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (game_id, related_game_id),
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
    FOREIGN KEY (related_game_id) REFERENCES games(id) ON DELETE CASCADE
);

CREATE INDEX idx_related_games_game ON related_games(game_id);

-- ---------------------------------------------------------------------------
-- 7. Projects: delegation to a game
-- ---------------------------------------------------------------------------
PRAGMA foreign_keys=off;

CREATE TABLE projects_new (
    id INTEGER PRIMARY KEY,
    post_id INTEGER NOT NULL UNIQUE,

    -- 'jsdos'/'v86' are transitional: real databases still carry legacy rows
    -- with those values, which 20260820020000 converts to 'game' before
    -- re-tightening this CHECK.
    demo_type TEXT NOT NULL DEFAULT 'html5'
        CHECK (demo_type IN ('none', 'html5', 'embed', 'webgl', 'download', 'video', 'game', 'jsdos', 'v86')),
    demo_entry_path TEXT NOT NULL DEFAULT 'index.html',
    demo_width TEXT,
    demo_height TEXT,
    demo_config TEXT,
    demo_url TEXT,

    -- A project delegates to a game ONLY when demo_type = 'game'.
    delegate_game_id INTEGER,
    inherit_thumbnail INTEGER NOT NULL DEFAULT 1 CHECK (inherit_thumbnail IN (0, 1)),
    inherit_tags INTEGER NOT NULL DEFAULT 1 CHECK (inherit_tags IN (0, 1)),

    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE,
    FOREIGN KEY (delegate_game_id) REFERENCES games(id) ON DELETE SET NULL
);

INSERT INTO projects_new (
    id, post_id, demo_type, demo_entry_path, demo_width, demo_height,
    demo_config, demo_url, created_at, updated_at
)
SELECT
    id, post_id, demo_type, demo_entry_path, demo_width, demo_height,
    demo_config, demo_url, created_at, updated_at
FROM projects;

DROP TABLE projects;
ALTER TABLE projects_new RENAME TO projects;

CREATE INDEX IF NOT EXISTS idx_projects_post_id ON projects(post_id);
CREATE INDEX IF NOT EXISTS idx_projects_delegate_game ON projects(delegate_game_id);

PRAGMA foreign_keys=on;