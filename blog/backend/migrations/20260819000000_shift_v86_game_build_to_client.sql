-- Shift v86 game building to the browser: the client unzips the game, builds
-- the FAT disk and the launcher CDs, and uploads the finished artifacts. The
-- server becomes a pure receiver: it no longer stores the game ZIP (or its
-- name/size), so the ZIP columns are dropped here and content-addressed dedup
-- moves from zip_sha256 to disk_sha256 / iso_sha256 lookups.

-- 1) project_v86_games: drop the ZIP bookkeeping, index the dedup keys.
CREATE TABLE project_v86_games_new (
    project_id INTEGER PRIMARY KEY,
    system_version_id INTEGER NOT NULL,
    manifest_text TEXT NOT NULL,
    manifest_sha256 TEXT NOT NULL,
    iso_storage_key TEXT NOT NULL,
    iso_size_bytes INTEGER NOT NULL CHECK (iso_size_bytes > 0),
    iso_sha256 TEXT NOT NULL,
    chunk_size_bytes INTEGER NOT NULL CHECK (chunk_size_bytes > 0),
    chunk_count INTEGER NOT NULL CHECK (chunk_count > 0),
    artifact_revision INTEGER NOT NULL DEFAULT 1 CHECK (artifact_revision > 0),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    disk_storage_key TEXT,
    disk_sha256 TEXT,
    disk_size_bytes INTEGER,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (system_version_id) REFERENCES v86_system_versions(id) ON DELETE RESTRICT
);
INSERT INTO project_v86_games_new (project_id, system_version_id, manifest_text, manifest_sha256,
    iso_storage_key, iso_size_bytes, iso_sha256, chunk_size_bytes, chunk_count, artifact_revision,
    created_at, updated_at, disk_storage_key, disk_sha256, disk_size_bytes)
  SELECT project_id, system_version_id, manifest_text, manifest_sha256,
    iso_storage_key, iso_size_bytes, iso_sha256, chunk_size_bytes, chunk_count, artifact_revision,
    created_at, updated_at, disk_storage_key, disk_sha256, disk_size_bytes
  FROM project_v86_games;
DROP TABLE project_v86_games;
ALTER TABLE project_v86_games_new RENAME TO project_v86_games;
CREATE INDEX idx_project_v86_games_system_version ON project_v86_games(system_version_id);
CREATE INDEX idx_project_v86_games_disk ON project_v86_games(disk_sha256);
CREATE INDEX idx_project_v86_games_iso ON project_v86_games(iso_sha256);

-- 2) project_v86_upload_sessions: drop the ZIP multipart bookkeeping; the
-- session now only records the client's build plans and upload progress.
CREATE TABLE project_v86_upload_sessions_new (
    id TEXT PRIMARY KEY,
    uploader_id INTEGER NOT NULL,
    source_project_id INTEGER,
    system_version_id INTEGER NOT NULL,
    expected_artifact_revision INTEGER NOT NULL DEFAULT 0,
    manifest_text TEXT NOT NULL,
    manifest_sha256 TEXT NOT NULL,
    staged_disk_storage_key TEXT,
    staged_disk_sha256 TEXT,
    staged_disk_size_bytes INTEGER,
    staged_disk_chunk_count INTEGER,
    disk_reuse INTEGER NOT NULL DEFAULT 0,
    received_disk_parts TEXT,
    staged_iso_storage_key TEXT,
    staged_iso_sha256 TEXT,
    staged_iso_size_bytes INTEGER,
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'building', 'ready', 'consumed', 'failed', 'aborted', 'expired')),
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT NOT NULL,
    FOREIGN KEY (uploader_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (source_project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (system_version_id) REFERENCES v86_system_versions(id) ON DELETE RESTRICT
);
INSERT INTO project_v86_upload_sessions_new (id, uploader_id, source_project_id,
    system_version_id, expected_artifact_revision, manifest_text, manifest_sha256,
    staged_disk_storage_key, staged_disk_sha256, staged_disk_size_bytes, staged_disk_chunk_count,
    disk_reuse, received_disk_parts,
    staged_iso_storage_key, staged_iso_sha256, staged_iso_size_bytes,
    status, error_message, created_at, updated_at, expires_at)
  SELECT id, uploader_id, source_project_id,
    system_version_id, expected_artifact_revision, manifest_text, manifest_sha256,
    staged_disk_storage_key, staged_disk_sha256, staged_disk_size_bytes, staged_iso_chunk_count,
    0, NULL,
    staged_iso_storage_key, staged_iso_sha256, staged_iso_size_bytes,
    status, error_message, created_at, updated_at, expires_at
  FROM project_v86_upload_sessions;
DROP TABLE project_v86_upload_sessions;
ALTER TABLE project_v86_upload_sessions_new RENAME TO project_v86_upload_sessions;
CREATE INDEX idx_v86_game_upload_expiry ON project_v86_upload_sessions(status, expires_at);

-- 3) project_v86_staged_variants: track whether a variant CD was reused from
-- an existing project (no upload) and whether its bytes were received.
ALTER TABLE project_v86_staged_variants ADD COLUMN reuse INTEGER NOT NULL DEFAULT 0;
ALTER TABLE project_v86_staged_variants ADD COLUMN received INTEGER NOT NULL DEFAULT 0;