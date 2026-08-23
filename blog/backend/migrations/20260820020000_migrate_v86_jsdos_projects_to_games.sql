-- ---------------------------------------------------------------------------
-- Data migration: convert existing v86/jsdos projects into standalone games,
-- then set the project's demo_type='game' + delegate_game_id to the new game.
-- Preserves existing URLs (the project keeps its slug) and moves the
-- artifacts (v86 game, variants, snapshots, saves, js-dos bundle).
-- ---------------------------------------------------------------------------

-- 1. Create a games row per existing v86/jsdos project.
--    The new game reuses the SAME posts row (content_kind is already a post);
--    we can't have two post rows for one project, so the game's post is the
--    project's post. That keeps slug, title, cover, tags, comments, stats.
INSERT INTO games (post_id, launcher_type, demo_width, demo_height, demo_url, instruction, cheatcode, story, created_at, updated_at)
SELECT
    p.post_id,
    CASE p.demo_type WHEN 'jsdos' THEN 'jsdos' WHEN 'v86' THEN 'v86' ELSE 'html5' END,
    p.demo_width,
    p.demo_height,
    NULLIF(p.demo_url, ''),
    '',  -- instruction: empty for now
    '',  -- cheatcode: empty for now
    posts.excerpt, -- seed story with the project excerpt as a starting point
    COALESCE(p.created_at, CURRENT_TIMESTAMP),
    COALESCE(p.updated_at, CURRENT_TIMESTAMP)
FROM projects p
JOIN posts ON posts.id = p.post_id
WHERE p.demo_type IN ('jsdos', 'v86');

-- 2. Move js-dos bundles.
INSERT OR IGNORE INTO game_jsdos_bundles (game_id, storage_key, original_file_name, size_bytes, sha256, created_at, updated_at)
SELECT g.id, b.storage_key, b.original_file_name, b.size_bytes, b.sha256, b.created_at, b.updated_at
FROM project_jsdos_bundles b
JOIN projects p ON p.id = b.project_id
JOIN games g ON g.post_id = p.post_id
WHERE p.demo_type = 'jsdos';

-- 3. Move v86 games. The legacy table lost its ZIP bookkeeping columns in
-- 20260819000000 (the client now builds the artifacts), so the ZIP fields are
-- filled with inert placeholders: nothing in the code reads them back.
INSERT OR IGNORE INTO game_v86_games (
    game_id, system_version_id, manifest_text, manifest_sha256,
    launcher_config_sha256, game_config_sha256,
    original_file_name, zip_storage_key, zip_size_bytes, zip_sha256,
    iso_storage_key, iso_size_bytes, iso_sha256,
    disk_storage_key, disk_sha256, disk_size_bytes,
    chunk_size_bytes, chunk_count, artifact_revision,
    created_at, updated_at
)
SELECT
    g.id, pg.system_version_id, pg.manifest_text, pg.manifest_sha256,
    pg.manifest_sha256, pg.manifest_sha256,
    posts.slug || '.zip',
    'v86/games/' || pg.iso_sha256 || '/game.zip',
    1,
    pg.iso_sha256,
    pg.iso_storage_key, pg.iso_size_bytes, pg.iso_sha256,
    pg.disk_storage_key, pg.disk_sha256, pg.disk_size_bytes,
    pg.chunk_size_bytes, pg.chunk_count, pg.artifact_revision,
    pg.created_at, pg.updated_at
FROM project_v86_games pg
JOIN projects p ON p.id = pg.project_id
JOIN games g ON g.post_id = p.post_id
JOIN posts ON posts.id = p.post_id
WHERE p.demo_type = 'v86';

-- 4. Move v86 variants.
INSERT OR IGNORE INTO game_v86_variants (game_id, variant_index, name, exe, args, iso_storage_key, iso_size_bytes, iso_sha256, created_at)
SELECT g.id, v.variant_index, v.name, v.exe, v.args, v.iso_storage_key, v.iso_size_bytes, v.iso_sha256, v.created_at
FROM project_v86_variants v
JOIN projects p ON p.id = v.project_id
JOIN games g ON g.post_id = p.post_id
WHERE p.demo_type = 'v86';

-- 5. Move v86 snapshots.
INSERT OR IGNORE INTO game_v86_snapshots (
    game_id, variant_index, system_version_id, game_disk_sha256, iso_sha256,
    storage_key, size_bytes, raw_size_bytes, sha256, state_version,
    topology_version, memory_size, vga_memory_size, created_by, created_at, updated_at
)
SELECT
    g.id, COALESCE(s.variant_index, 0), s.system_version_id, s.game_disk_sha256,
    COALESCE(s.iso_sha256, ''),
    s.storage_key, s.size_bytes, s.raw_size_bytes, s.sha256, s.state_version,
    COALESCE(s.topology_version, 0), s.memory_size, s.vga_memory_size,
    s.created_by, s.created_at, s.updated_at
FROM project_v86_snapshots s
JOIN projects p ON p.id = s.project_id
JOIN games g ON g.post_id = p.post_id
WHERE p.demo_type = 'v86';

-- 6. Move v86 saves (durable: keep them). The legacy table has no disk
-- pinning, so the migrated saves start unpinned ('') and are served until a
-- mod explicitly retires them.
INSERT OR IGNORE INTO game_v86_saves (game_id, user_id, storage_key, size_bytes, sha256, game_disk_sha256, created_at, updated_at)
SELECT
    g.id, s.user_id, s.storage_key, s.size_bytes, s.sha256,
    '',
    s.created_at, s.updated_at
FROM v86_saves s
JOIN projects p ON p.id = s.project_id
JOIN games g ON g.post_id = p.post_id
WHERE p.demo_type = 'v86';

-- 7. Point the original projects at their new game and switch demo_type.
UPDATE projects
SET demo_type = 'game',
    delegate_game_id = (
        SELECT g.id FROM games g WHERE g.post_id = projects.post_id
    ),
    demo_url = NULL,
    updated_at = CURRENT_TIMESTAMP
WHERE demo_type IN ('jsdos', 'v86');

-- 8. Re-tighten the demo_type CHECK now that every legacy jsdos/v86 row has
-- been converted to 'game' (20260820010000 had to admit the legacy values so
-- this conversion could run at all).
PRAGMA foreign_keys=off;

CREATE TABLE projects_strict (
    id INTEGER PRIMARY KEY,
    post_id INTEGER NOT NULL UNIQUE,

    demo_type TEXT NOT NULL DEFAULT 'html5'
        CHECK (demo_type IN ('none', 'html5', 'embed', 'webgl', 'download', 'video', 'game')),
    demo_entry_path TEXT NOT NULL DEFAULT 'index.html',
    demo_width TEXT,
    demo_height TEXT,
    demo_config TEXT,
    demo_url TEXT,

    delegate_game_id INTEGER,
    inherit_thumbnail INTEGER NOT NULL DEFAULT 1 CHECK (inherit_thumbnail IN (0, 1)),
    inherit_tags INTEGER NOT NULL DEFAULT 1 CHECK (inherit_tags IN (0, 1)),

    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE,
    FOREIGN KEY (delegate_game_id) REFERENCES games(id) ON DELETE SET NULL
);

INSERT INTO projects_strict (
    id, post_id, demo_type, demo_entry_path, demo_width, demo_height,
    demo_config, demo_url, delegate_game_id, inherit_thumbnail, inherit_tags,
    created_at, updated_at
)
SELECT
    id, post_id, demo_type, demo_entry_path, demo_width, demo_height,
    demo_config, demo_url, delegate_game_id, inherit_thumbnail, inherit_tags,
    created_at, updated_at
FROM projects;

DROP TABLE projects;
ALTER TABLE projects_strict RENAME TO projects;

CREATE INDEX IF NOT EXISTS idx_projects_post_id ON projects(post_id);
CREATE INDEX IF NOT EXISTS idx_projects_delegate_game ON projects(delegate_game_id);

PRAGMA foreign_keys=on;

-- ---------------------------------------------------------------------------
-- The legacy project-scoped v86/jsdos tables are deliberately NOT dropped here;
-- they are kept so the old endpoints keep working for any project that was not
-- converted (e.g. non-migrated rows). A later cleanup migration can drop them
-- once the migration is verified.
-- ---------------------------------------------------------------------------
