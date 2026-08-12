-- v86 initial_state snapshots.
--
-- A snapshot is a v86 save_state() blob (zstd-compressed client side) that lets
-- the player restore an already-booted machine instead of running the full
-- BIOS -> bootloader -> Windows boot sequence on every visit.
--
-- Snapshots are captured with hda (base system) + hdb (game disk) attached and
-- deliberately NO cdrom and NO floppy: v86 always instantiates those drive
-- objects, so the guest still enumerates both drive letters, and inserting
-- media later raises the media-change condition that triggers autorun. Media
-- attached at construction time would instead be masked by the IDE restore
-- path, which force-clears medium_changed.
--
-- Because the state embeds the guest's dirty disk blocks and RAM, it is only
-- valid against the exact disk images it was taken from. It is therefore
-- pinned to (system_version_id, game_disk_sha256) and additionally gated on
-- state_version / memory_size so a v86 upgrade or a memory resize degrades to
-- a normal cold boot instead of a failed restore.

CREATE TABLE project_v86_snapshots (
    project_id INTEGER PRIMARY KEY,
    system_version_id INTEGER NOT NULL,
    game_disk_sha256 TEXT NOT NULL,
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
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (system_version_id) REFERENCES v86_system_versions(id) ON DELETE RESTRICT,
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE SET NULL
);

-- Column names for the staging/transfer fields intentionally match
-- v86_system_upload_sessions so append_upload_chunk (which takes the table
-- name as a parameter) works against this table unchanged.
CREATE TABLE v86_snapshot_upload_sessions (
    id TEXT PRIMARY KEY,
    uploader_id INTEGER NOT NULL,
    project_id INTEGER NOT NULL,
    system_version_id INTEGER NOT NULL,
    game_disk_sha256 TEXT NOT NULL,
    raw_size_bytes INTEGER NOT NULL CHECK (raw_size_bytes > 0),
    sha256 TEXT NOT NULL,
    state_version INTEGER NOT NULL CHECK (state_version > 0),
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
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (system_version_id) REFERENCES v86_system_versions(id) ON DELETE CASCADE
);

CREATE INDEX idx_v86_snapshot_sessions_uploader
    ON v86_snapshot_upload_sessions (uploader_id, status);
