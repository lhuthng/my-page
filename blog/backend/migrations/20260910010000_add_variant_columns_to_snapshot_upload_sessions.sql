-- The games split (20260820010000) recreated the snapshot upload-session
-- table from the pre-variant schema and dropped variant_index + iso_sha256,
-- which 20260812120000 had already added to the project-era table. The
-- capture code inserts both, so every snapshot capture 500s. Restore them.
ALTER TABLE game_v86_snapshot_upload_sessions
    ADD COLUMN variant_index INTEGER NOT NULL DEFAULT 0;
ALTER TABLE game_v86_snapshot_upload_sessions ADD COLUMN iso_sha256 TEXT NOT NULL DEFAULT '';
