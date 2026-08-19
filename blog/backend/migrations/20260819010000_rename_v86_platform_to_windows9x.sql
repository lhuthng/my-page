-- Rename the v86 platform strategy from Windows 95 to the Windows 9x family.
-- SQLite cannot ALTER a CHECK constraint, so the two tables that carry
-- `platform_key = 'windows95'` are rebuilt with the windows9x value; existing
-- rows are updated to the new key.

PRAGMA foreign_keys=off;

-- 1) v86_systems: the platform_key CHECK changes and existing rows move to
--    the new family key. Child tables reference v86_systems by name, so the
--    rebuild preserves the table name and its unique default index.
CREATE TABLE v86_systems_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL COLLATE NOCASE UNIQUE,
    platform_key TEXT NOT NULL CHECK (platform_key = 'windows9x'),
    is_active INTEGER NOT NULL DEFAULT 0 CHECK (is_active IN (0, 1)),
    is_default INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1)),
    current_version INTEGER NOT NULL DEFAULT 0 CHECK (current_version >= 0),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
INSERT INTO v86_systems_new (id, name, platform_key, is_active, is_default, current_version, created_at, updated_at)
  SELECT id, name, 'windows9x', is_active, is_default, current_version, created_at, updated_at
  FROM v86_systems;
DROP TABLE v86_systems;
ALTER TABLE v86_systems_new RENAME TO v86_systems;
CREATE UNIQUE INDEX idx_v86_systems_default
    ON v86_systems(is_default) WHERE is_default = 1;

-- 2) v86_system_upload_sessions: transient sessions keep their shape but accept
--    the new key, and any in-flight rows are updated to match.
CREATE TABLE v86_system_upload_sessions_new (
    id TEXT PRIMARY KEY,
    uploader_id INTEGER NOT NULL,
    system_id INTEGER,
    name TEXT NOT NULL,
    platform_key TEXT NOT NULL CHECK (platform_key = 'windows9x'),
    expected_current_version INTEGER NOT NULL DEFAULT 0,
    original_file_name TEXT NOT NULL,
    expected_size_bytes INTEGER NOT NULL CHECK (expected_size_bytes > 0),
    received_size_bytes INTEGER NOT NULL DEFAULT 0,
    next_chunk_index INTEGER NOT NULL DEFAULT 0,
    upload_chunk_size_bytes INTEGER NOT NULL DEFAULT 8388608,
    temp_storage_key TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'building', 'ready', 'consumed', 'failed', 'aborted', 'expired')),
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT NOT NULL,
    FOREIGN KEY (uploader_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (system_id) REFERENCES v86_systems(id) ON DELETE CASCADE
);
INSERT INTO v86_system_upload_sessions_new
    (id, uploader_id, system_id, name, platform_key, expected_current_version,
     original_file_name, expected_size_bytes, received_size_bytes, next_chunk_index,
     upload_chunk_size_bytes, temp_storage_key, status, error_message, created_at, updated_at, expires_at)
  SELECT id, uploader_id, system_id, name, 'windows9x', expected_current_version,
     original_file_name, expected_size_bytes, received_size_bytes, next_chunk_index,
     upload_chunk_size_bytes, temp_storage_key, status, error_message, created_at, updated_at, expires_at
  FROM v86_system_upload_sessions;
DROP TABLE v86_system_upload_sessions;
ALTER TABLE v86_system_upload_sessions_new RENAME TO v86_system_upload_sessions;

CREATE INDEX idx_v86_system_upload_expiry
    ON v86_system_upload_sessions(status, expires_at);

PRAGMA foreign_keys=on;