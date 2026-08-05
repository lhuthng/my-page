-- Per-user cloud saves for v86 projects. Saves are stored as a single
-- zstd-compressed floppy image on R2, keyed by user + project.
CREATE TABLE v86_saves (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    storage_key TEXT NOT NULL UNIQUE,
    size_bytes INTEGER NOT NULL CHECK (size_bytes > 0),
    sha256 TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(project_id, user_id)
);

-- The v86 game moved from a read-only ISO onto a writable FAT disk (D:).
-- Keep iso_* for the tiny read-only "autorun" CD, add disk_* for the game disk.
ALTER TABLE project_v86_games ADD COLUMN disk_storage_key TEXT;
ALTER TABLE project_v86_games ADD COLUMN disk_sha256 TEXT;
ALTER TABLE project_v86_games ADD COLUMN disk_size_bytes INTEGER;

ALTER TABLE project_v86_upload_sessions ADD COLUMN staged_disk_storage_key TEXT;
ALTER TABLE project_v86_upload_sessions ADD COLUMN staged_disk_sha256 TEXT;
ALTER TABLE project_v86_upload_sessions ADD COLUMN staged_disk_size_bytes INTEGER;
