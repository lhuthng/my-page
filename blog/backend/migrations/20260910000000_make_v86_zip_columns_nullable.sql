-- The ZIP bookkeeping columns (original_file_name, zip_storage_key,
-- zip_size_bytes, zip_sha256) date from the server-side build era. Since
-- 20260819000000 the browser builds the artifacts and no zip ever exists, and
-- nothing reads these columns back — but they were still NOT NULL, so every
-- fresh v86 upload failed its INSERT. Nothing needs them; make them nullable
-- for new rows. SQLite requires a table rebuild for this.
CREATE TABLE game_v86_games_new (
    game_id INTEGER PRIMARY KEY,
    system_version_id INTEGER NOT NULL,
    manifest_text TEXT NOT NULL,
    manifest_sha256 TEXT NOT NULL,
    launcher_config_sha256 TEXT NOT NULL DEFAULT '',
    game_config_sha256 TEXT NOT NULL DEFAULT '',
    original_file_name TEXT,
    zip_storage_key TEXT UNIQUE,
    zip_size_bytes INTEGER CHECK (zip_size_bytes IS NULL OR zip_size_bytes > 0),
    zip_sha256 TEXT,
    iso_storage_key TEXT NOT NULL UNIQUE,
    iso_size_bytes INTEGER NOT NULL CHECK (iso_size_bytes > 0),
    iso_sha256 TEXT NOT NULL,
    disk_storage_key TEXT,
    disk_sha256 TEXT,
    disk_size_bytes INTEGER,
    chunk_size_bytes INTEGER NOT NULL CHECK (chunk_size_bytes > 0),
    chunk_count INTEGER NOT NULL CHECK (chunk_count > 0),
    artifact_revision INTEGER NOT NULL DEFAULT 1 CHECK (artifact_revision > 0),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
    FOREIGN KEY (system_version_id) REFERENCES v86_system_versions(id) ON DELETE RESTRICT
);

INSERT INTO game_v86_games_new (
    game_id, system_version_id, manifest_text, manifest_sha256,
    launcher_config_sha256, game_config_sha256,
    original_file_name, zip_storage_key, zip_size_bytes, zip_sha256,
    iso_storage_key, iso_size_bytes, iso_sha256,
    disk_storage_key, disk_sha256, disk_size_bytes,
    chunk_size_bytes, chunk_count, artifact_revision,
    created_at, updated_at
)
SELECT
    game_id, system_version_id, manifest_text, manifest_sha256,
    launcher_config_sha256, game_config_sha256,
    original_file_name, zip_storage_key, zip_size_bytes, zip_sha256,
    iso_storage_key, iso_size_bytes, iso_sha256,
    disk_storage_key, disk_sha256, disk_size_bytes,
    chunk_size_bytes, chunk_count, artifact_revision,
    created_at, updated_at
FROM game_v86_games;

DROP TABLE game_v86_games;
ALTER TABLE game_v86_games_new RENAME TO game_v86_games;

CREATE INDEX idx_game_v86_games_system_version
    ON game_v86_games(system_version_id);
