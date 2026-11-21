-- Payload bundles: one uploaded zip per game, unpacked on completion into the
-- payload store and served file-by-file under /games/s/<slug>/payload/. Both
-- payload launcher kinds ('shimkit', 'wasmshell') share this table; the kind
-- comes from the bundle's own manifest (bundle.json, format 1, see the
-- win32-shim-kit repo's docs/web-bundle-contract.md) and is never accepted from
-- the uploader.
--
-- The storage_key names the unpacked bundle directory, content-addressed by the
-- zip digest: re-uploading the same bundle is free, a new bundle unpacks beside
-- the old one and the row flips over atomically.

CREATE TABLE IF NOT EXISTS game_payload_bundles (
    game_id INTEGER PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('shimkit', 'wasmshell')),
    storage_key TEXT NOT NULL UNIQUE,
    zip_sha256 TEXT NOT NULL,
    zip_size_bytes INTEGER NOT NULL CHECK (zip_size_bytes > 0),
    -- The bundle's own manifest, kept verbatim: the player reads slug, title,
    -- entry, variants and saves out of it rather than out of a wire type that
    -- could drift from the contract.
    manifest_json TEXT NOT NULL,
    artifact_revision INTEGER NOT NULL DEFAULT 1 CHECK (artifact_revision > 0),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE
);

-- The upload session: same shape as the js-dos one, sized for the bigger
-- bundles a source port ships. Chunks are appended to one temp file on the VM
-- disk; completion validates the archive, unpacks it and only then flips the
-- game's launcher type.
CREATE TABLE IF NOT EXISTS game_payload_upload_sessions (
    id TEXT PRIMARY KEY,
    game_id INTEGER NOT NULL,
    uploader_id INTEGER NOT NULL,
    original_file_name TEXT NOT NULL,
    expected_size_bytes INTEGER NOT NULL CHECK (expected_size_bytes > 0 AND expected_size_bytes <= 2147483648),
    received_size_bytes INTEGER NOT NULL DEFAULT 0 CHECK (received_size_bytes >= 0),
    next_chunk_index INTEGER NOT NULL DEFAULT 0 CHECK (next_chunk_index >= 0),
    chunk_size_bytes INTEGER NOT NULL DEFAULT 8388608 CHECK (chunk_size_bytes > 0),
    temp_storage_key TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'completed', 'aborted', 'expired')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT NOT NULL,
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
    FOREIGN KEY (uploader_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_game_payload_upload_sessions_game
    ON game_payload_upload_sessions(game_id);
CREATE INDEX idx_game_payload_upload_sessions_expiry
    ON game_payload_upload_sessions(status, expires_at);
