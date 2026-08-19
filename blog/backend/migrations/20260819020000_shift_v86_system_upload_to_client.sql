-- Shift v86 system (base image) upload to the client-side model: the client
-- hashes the IMG, splits it into 256 KiB zstd parts, and uploads them directly
-- to content-addressed keys. The server becomes a pure receiver + verifier.
-- This eliminates the R2 tmp assembled image and server-side
-- download/split/compress, and enables parallel part uploads.

-- 1) Rebuild v86_system_upload_sessions:
--    DROP: received_size_bytes, next_chunk_index, upload_chunk_size_bytes,
--          temp_storage_key, r2_upload_id, r2_part_etags (all multipart leftovers)
--    ADD:  staged_storage_key, staged_sha256, staged_chunk_count, reuse flag.
--    Sessions still active/building from the old protocol are marked failed.

CREATE TABLE v86_system_upload_sessions_new (
    id TEXT PRIMARY KEY,
    uploader_id INTEGER NOT NULL,
    system_id INTEGER,
    name TEXT NOT NULL,
    platform_key TEXT NOT NULL CHECK (platform_key = 'windows9x'),
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

-- Copy rows over; any in-flight old-protocol sessions become failed.
INSERT INTO v86_system_upload_sessions_new (
    id, uploader_id, system_id, name, platform_key, expected_current_version,
    original_file_name, expected_size_bytes, status, error_message,
    created_at, updated_at, expires_at
)
SELECT
    id, uploader_id, system_id, name, platform_key, expected_current_version,
    original_file_name, expected_size_bytes,
    CASE WHEN status IN ('active', 'building') THEN 'failed' ELSE status END,
    CASE WHEN status IN ('active', 'building')
        THEN 'Upload method changed; re-upload the image.'
        ELSE error_message END,
    created_at, updated_at, expires_at
FROM v86_system_upload_sessions;

DROP TABLE v86_system_upload_sessions;
ALTER TABLE v86_system_upload_sessions_new RENAME TO v86_system_upload_sessions;

CREATE INDEX idx_v86_system_upload_expiry
    ON v86_system_upload_sessions(status, expires_at);

-- 2) Track received parts in a child table so parallel part uploads are
--    recorded atomically (a plain INSERT, unlike a read-modify-write of a JSON
--    column which races under parallel uploads).
CREATE TABLE v86_system_upload_parts (
    upload_id TEXT NOT NULL
        REFERENCES v86_system_upload_sessions(id) ON DELETE CASCADE,
    part_index INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (upload_id, part_index)
);

CREATE INDEX idx_v86_system_upload_parts_session
    ON v86_system_upload_parts(upload_id);