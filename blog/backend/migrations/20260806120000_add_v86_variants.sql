-- v86 game variants: a single project can expose several launch variants,
-- each backed by its own autorun CD (E:) that points the shared game disk (D:)
-- at a different executable. The disk lives on project_v86_games (unchanged);
-- the per-variant ISOs live here.

CREATE TABLE project_v86_variants (
    project_id INTEGER NOT NULL,
    variant_index INTEGER NOT NULL CHECK (variant_index > 0),
    name TEXT NOT NULL,
    exe TEXT NOT NULL,
    args TEXT NOT NULL DEFAULT '',
    iso_storage_key TEXT NOT NULL,
    iso_size_bytes INTEGER NOT NULL CHECK (iso_size_bytes > 0),
    iso_sha256 TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (project_id, variant_index),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX idx_project_v86_variants_iso ON project_v86_variants(iso_sha256);

-- Staged variants: carried by an upload session until it is attached to a
-- project, mirroring the staged_iso_* columns on the session row (which keep
-- variant 1 for backward compatibility).
CREATE TABLE project_v86_staged_variants (
    upload_id TEXT NOT NULL,
    variant_index INTEGER NOT NULL CHECK (variant_index > 0),
    name TEXT NOT NULL,
    exe TEXT NOT NULL,
    args TEXT NOT NULL DEFAULT '',
    iso_storage_key TEXT NOT NULL,
    iso_size_bytes INTEGER NOT NULL CHECK (iso_size_bytes > 0),
    iso_sha256 TEXT NOT NULL,
    PRIMARY KEY (upload_id, variant_index),
    FOREIGN KEY (upload_id) REFERENCES project_v86_upload_sessions(id) ON DELETE CASCADE
);

-- Backfill: existing projects get a single default variant so the runtime can
-- always source variants from project_v86_variants. Index 1 mirrors the ISO on
-- project_v86_games.
INSERT INTO project_v86_variants (project_id, variant_index, name, exe, args, iso_storage_key, iso_size_bytes, iso_sha256)
SELECT g.project_id, 1, '', '', '', g.iso_storage_key, g.iso_size_bytes, g.iso_sha256
FROM project_v86_games g
WHERE NOT EXISTS (SELECT 1 FROM project_v86_variants v WHERE v.project_id = g.project_id);
