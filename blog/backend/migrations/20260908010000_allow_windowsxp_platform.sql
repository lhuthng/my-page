-- Accept the windowsxp platform key alongside windows9x.
-- SQLite cannot ALTER a CHECK constraint, so the tables that carry
-- `platform_key = 'windows9x'` are rebuilt following the 20260819010000
-- precedent, carrying the memory_size_mb columns added by the previous
-- migration and adding range CHECKs for them while the tables are open.
-- The session schema mirrors the live layout (client-shifted upload:
-- staged_* columns, no received/next_chunk tracking).

PRAGMA foreign_keys=off;

CREATE TABLE v86_systems_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL COLLATE NOCASE UNIQUE,
    platform_key TEXT NOT NULL CHECK (platform_key IN ('windows9x', 'windowsxp')),
    memory_size_mb INTEGER NOT NULL DEFAULT 64 CHECK (memory_size_mb BETWEEN 32 AND 1024),
    is_active INTEGER NOT NULL DEFAULT 0 CHECK (is_active IN (0, 1)),
    is_default INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1)),
    current_version INTEGER NOT NULL DEFAULT 0 CHECK (current_version >= 0),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
INSERT INTO v86_systems_new (id, name, platform_key, memory_size_mb, is_active, is_default, current_version, created_at, updated_at)
  SELECT id, name, platform_key, memory_size_mb, is_active, is_default, current_version, created_at, updated_at
  FROM v86_systems;
DROP TABLE v86_systems;
ALTER TABLE v86_systems_new RENAME TO v86_systems;
CREATE UNIQUE INDEX idx_v86_systems_default
    ON v86_systems(is_default) WHERE is_default = 1;

CREATE TABLE v86_system_upload_sessions_new (
    id TEXT PRIMARY KEY,
    uploader_id INTEGER NOT NULL,
    system_id INTEGER,
    name TEXT NOT NULL,
    platform_key TEXT NOT NULL CHECK (platform_key IN ('windows9x', 'windowsxp')),
    memory_size_mb INTEGER CHECK (memory_size_mb IS NULL OR memory_size_mb BETWEEN 32 AND 1024),
    expected_current_version INTEGER NOT NULL DEFAULT 0,
    original_file_name TEXT NOT NULL,
    expected_size_bytes INTEGER NOT NULL CHECK (expected_size_bytes > 0),
    staged_storage_key TEXT,
    staged_sha256 TEXT,
    staged_chunk_count INTEGER,
    reuse INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'building', 'consumed', 'failed', 'aborted', 'expired')),
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT NOT NULL,
    FOREIGN KEY (uploader_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (system_id) REFERENCES v86_systems(id) ON DELETE CASCADE
);
INSERT INTO v86_system_upload_sessions_new
    (id, uploader_id, system_id, name, platform_key, memory_size_mb, expected_current_version,
     original_file_name, expected_size_bytes, staged_storage_key, staged_sha256,
     staged_chunk_count, reuse, status, error_message, created_at, updated_at, expires_at)
  SELECT id, uploader_id, system_id, name, platform_key, memory_size_mb, expected_current_version,
     original_file_name, expected_size_bytes, staged_storage_key, staged_sha256,
     staged_chunk_count, reuse, status, error_message, created_at, updated_at, expires_at
  FROM v86_system_upload_sessions;
DROP TABLE v86_system_upload_sessions;
ALTER TABLE v86_system_upload_sessions_new RENAME TO v86_system_upload_sessions;

CREATE INDEX idx_v86_system_upload_expiry
    ON v86_system_upload_sessions(status, expires_at);

PRAGMA foreign_keys=on;
