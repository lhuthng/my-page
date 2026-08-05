PRAGMA foreign_keys=off;

-- Recreate project_v86_games without UNIQUE on zip_storage_key and iso_storage_key.
-- Content-addressed keys (v86/games/zips/{sha}.zip, v86/games/{sha}) are designed to
-- be shared across projects. The refcount-based deletion in project.rs already handles
-- this correctly (deletes only when no project references the key).

CREATE TABLE project_v86_games_new (
    project_id INTEGER PRIMARY KEY,
    system_version_id INTEGER NOT NULL,
    manifest_text TEXT NOT NULL,
    manifest_sha256 TEXT NOT NULL,
    original_file_name TEXT NOT NULL,
    zip_storage_key TEXT NOT NULL,
    zip_size_bytes INTEGER NOT NULL CHECK (zip_size_bytes > 0),
    zip_sha256 TEXT NOT NULL,
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

INSERT INTO project_v86_games_new (
    project_id, system_version_id, manifest_text, manifest_sha256,
    original_file_name, zip_storage_key, zip_size_bytes, zip_sha256,
    iso_storage_key, iso_size_bytes, iso_sha256, chunk_size_bytes,
    chunk_count, artifact_revision, created_at, updated_at,
    disk_storage_key, disk_sha256, disk_size_bytes
)
SELECT
    project_id, system_version_id, manifest_text, manifest_sha256,
    original_file_name, zip_storage_key, zip_size_bytes, zip_sha256,
    iso_storage_key, iso_size_bytes, iso_sha256, chunk_size_bytes,
    chunk_count, artifact_revision, created_at, updated_at,
    disk_storage_key, disk_sha256, disk_size_bytes
FROM project_v86_games;

DROP TABLE project_v86_games;
ALTER TABLE project_v86_games_new RENAME TO project_v86_games;

CREATE INDEX idx_project_v86_games_system_version
    ON project_v86_games(system_version_id);

PRAGMA foreign_keys=on;