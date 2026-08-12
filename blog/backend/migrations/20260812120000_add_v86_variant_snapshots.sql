-- Per-variant v86 snapshots.
--
-- Two kinds of snapshot now coexist, and they differ in device topology, which
-- the player must reproduce exactly when restoring (v86 throws if the state
-- records an empty drive while a disc is attached, and vice versa):
--
--   variant_index = 0  project-wide, captured with NO cdrom. The player
--                      restores without a disc and inserts one afterwards.
--   variant_index > 0  captured with that variant's launcher CD already in,
--                      typically while the launcher idles in its delay window.
--                      The player passes that same ISO at construction.
--
-- Variant snapshots therefore also pin iso_sha256: rebuilding a variant's CD
-- (any manifest edit) changes the disc under a state that cached its contents.
--
-- Existing rows were all captured without a disc, so they migrate to
-- variant_index = 0 and keep working as the project-wide fallback.

CREATE TABLE project_v86_snapshots_new (
    project_id INTEGER NOT NULL,
    variant_index INTEGER NOT NULL DEFAULT 0 CHECK (variant_index >= 0),
    system_version_id INTEGER NOT NULL,
    game_disk_sha256 TEXT NOT NULL,
    -- Required when variant_index > 0, always NULL when variant_index = 0.
    iso_sha256 TEXT,
    storage_key TEXT NOT NULL,
    size_bytes INTEGER NOT NULL CHECK (size_bytes > 0),
    raw_size_bytes INTEGER NOT NULL CHECK (raw_size_bytes > 0),
    sha256 TEXT NOT NULL,
    state_version INTEGER NOT NULL CHECK (state_version > 0),
    memory_size INTEGER NOT NULL CHECK (memory_size > 0),
    vga_memory_size INTEGER NOT NULL CHECK (vga_memory_size > 0),
    created_by INTEGER,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (project_id, variant_index),
    CHECK ((variant_index = 0) = (iso_sha256 IS NULL)),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (system_version_id) REFERENCES v86_system_versions(id) ON DELETE RESTRICT,
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE SET NULL
);

INSERT INTO project_v86_snapshots_new (
    project_id, variant_index, system_version_id, game_disk_sha256, iso_sha256,
    storage_key, size_bytes, raw_size_bytes, sha256, state_version,
    memory_size, vga_memory_size, created_by, created_at, updated_at
)
SELECT
    project_id, 0, system_version_id, game_disk_sha256, NULL,
    storage_key, size_bytes, raw_size_bytes, sha256, state_version,
    memory_size, vga_memory_size, created_by, created_at, updated_at
FROM project_v86_snapshots;

DROP TABLE project_v86_snapshots;
ALTER TABLE project_v86_snapshots_new RENAME TO project_v86_snapshots;

-- Upload sessions carry the same two fields through to completion.
ALTER TABLE v86_snapshot_upload_sessions
    ADD COLUMN variant_index INTEGER NOT NULL DEFAULT 0;
ALTER TABLE v86_snapshot_upload_sessions ADD COLUMN iso_sha256 TEXT;
