PRAGMA foreign_keys=off;

CREATE TABLE projects_new (
    id INTEGER PRIMARY KEY,
    post_id INTEGER NOT NULL UNIQUE,
    demo_type TEXT NOT NULL DEFAULT 'html5'
        CHECK (demo_type IN ('none', 'html5', 'embed', 'webgl', 'download', 'video', 'jsdos', 'v86')),
    demo_entry_path TEXT NOT NULL DEFAULT 'index.html',
    demo_width TEXT,
    demo_height TEXT,
    demo_config TEXT,
    demo_url TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE
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

CREATE TABLE v86_systems (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL COLLATE NOCASE UNIQUE,
    platform_key TEXT NOT NULL CHECK (platform_key = 'windows95'),
    is_active INTEGER NOT NULL DEFAULT 0 CHECK (is_active IN (0, 1)),
    is_default INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1)),
    current_version INTEGER NOT NULL DEFAULT 0 CHECK (current_version >= 0),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX idx_v86_systems_default
    ON v86_systems(is_default) WHERE is_default = 1;

CREATE TABLE v86_system_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    system_id INTEGER NOT NULL,
    version_number INTEGER NOT NULL CHECK (version_number > 0),
    original_file_name TEXT NOT NULL,
    storage_key TEXT NOT NULL UNIQUE,
    size_bytes INTEGER NOT NULL CHECK (size_bytes > 0),
    sha256 TEXT NOT NULL,
    chunk_size_bytes INTEGER NOT NULL CHECK (chunk_size_bytes > 0),
    chunk_count INTEGER NOT NULL CHECK (chunk_count > 0),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (system_id) REFERENCES v86_systems(id) ON DELETE CASCADE,
    UNIQUE(system_id, version_number)
);

CREATE TABLE project_v86_games (
    project_id INTEGER PRIMARY KEY,
    system_version_id INTEGER NOT NULL,
    manifest_text TEXT NOT NULL,
    manifest_sha256 TEXT NOT NULL,
    original_file_name TEXT NOT NULL,
    zip_storage_key TEXT NOT NULL UNIQUE,
    zip_size_bytes INTEGER NOT NULL CHECK (zip_size_bytes > 0),
    zip_sha256 TEXT NOT NULL,
    iso_storage_key TEXT NOT NULL UNIQUE,
    iso_size_bytes INTEGER NOT NULL CHECK (iso_size_bytes > 0),
    iso_sha256 TEXT NOT NULL,
    chunk_size_bytes INTEGER NOT NULL CHECK (chunk_size_bytes > 0),
    chunk_count INTEGER NOT NULL CHECK (chunk_count > 0),
    artifact_revision INTEGER NOT NULL DEFAULT 1 CHECK (artifact_revision > 0),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (system_version_id) REFERENCES v86_system_versions(id) ON DELETE RESTRICT
);

CREATE INDEX idx_project_v86_games_system_version
    ON project_v86_games(system_version_id);

CREATE TABLE v86_system_upload_sessions (
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
        CHECK (status IN ('active', 'ready', 'consumed', 'failed', 'aborted', 'expired')),
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT NOT NULL,
    FOREIGN KEY (uploader_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (system_id) REFERENCES v86_systems(id) ON DELETE CASCADE
);

CREATE TABLE project_v86_upload_sessions (
    id TEXT PRIMARY KEY,
    uploader_id INTEGER NOT NULL,
    source_project_id INTEGER,
    system_version_id INTEGER NOT NULL,
    expected_artifact_revision INTEGER NOT NULL DEFAULT 0,
    manifest_text TEXT NOT NULL,
    manifest_sha256 TEXT NOT NULL,
    original_file_name TEXT NOT NULL,
    expected_size_bytes INTEGER NOT NULL CHECK (expected_size_bytes > 0),
    received_size_bytes INTEGER NOT NULL DEFAULT 0,
    next_chunk_index INTEGER NOT NULL DEFAULT 0,
    upload_chunk_size_bytes INTEGER NOT NULL DEFAULT 8388608,
    temp_storage_key TEXT NOT NULL UNIQUE,
    staged_zip_storage_key TEXT,
    staged_zip_sha256 TEXT,
    staged_iso_storage_key TEXT,
    staged_iso_sha256 TEXT,
    staged_iso_size_bytes INTEGER,
    staged_iso_chunk_count INTEGER,
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

CREATE INDEX idx_v86_system_upload_expiry
    ON v86_system_upload_sessions(status, expires_at);
CREATE INDEX idx_v86_game_upload_expiry
    ON project_v86_upload_sessions(status, expires_at);

PRAGMA foreign_keys=on;
