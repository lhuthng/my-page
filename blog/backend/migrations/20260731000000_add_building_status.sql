PRAGMA foreign_keys=off;

CREATE TABLE v86_system_upload_sessions_new (
    id TEXT PRIMARY KEY,
    uploader_id INTEGER NOT NULL,
    system_id INTEGER,
    name TEXT NOT NULL,
    platform_key TEXT NOT NULL CHECK (platform_key = 'windows95'),
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
SELECT * FROM v86_system_upload_sessions;

DROP TABLE v86_system_upload_sessions;
ALTER TABLE v86_system_upload_sessions_new RENAME TO v86_system_upload_sessions;

CREATE INDEX idx_v86_system_upload_expiry
    ON v86_system_upload_sessions(status, expires_at);

PRAGMA foreign_keys=on;
