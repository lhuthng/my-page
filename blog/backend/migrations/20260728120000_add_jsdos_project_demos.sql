PRAGMA foreign_keys=off;

CREATE TABLE projects_new (
    id INTEGER PRIMARY KEY,
    post_id INTEGER NOT NULL UNIQUE,

    demo_type TEXT NOT NULL DEFAULT 'html5'
        CHECK (demo_type IN ('none', 'html5', 'embed', 'webgl', 'download', 'video', 'jsdos')),
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

CREATE TABLE project_jsdos_bundles (
    project_id INTEGER PRIMARY KEY,
    storage_key TEXT NOT NULL UNIQUE,
    original_file_name TEXT NOT NULL,
    size_bytes INTEGER NOT NULL CHECK (size_bytes > 0),
    sha256 TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE TABLE project_jsdos_upload_sessions (
    id TEXT PRIMARY KEY,
    project_id INTEGER NOT NULL,
    uploader_id INTEGER NOT NULL,
    original_file_name TEXT NOT NULL,
    expected_size_bytes INTEGER NOT NULL CHECK (expected_size_bytes > 0 AND expected_size_bytes <= 524288000),
    received_size_bytes INTEGER NOT NULL DEFAULT 0 CHECK (received_size_bytes >= 0),
    next_chunk_index INTEGER NOT NULL DEFAULT 0 CHECK (next_chunk_index >= 0),
    chunk_size_bytes INTEGER NOT NULL DEFAULT 8388608 CHECK (chunk_size_bytes > 0),
    temp_storage_key TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'completed', 'aborted', 'expired')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (uploader_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_jsdos_upload_sessions_project
    ON project_jsdos_upload_sessions(project_id);
CREATE INDEX idx_jsdos_upload_sessions_expiry
    ON project_jsdos_upload_sessions(status, expires_at);

PRAGMA foreign_keys=on;
