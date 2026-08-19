-- Track received game disk parts in a child table so parallel part uploads
-- are recorded atomically (a plain INSERT, unlike the previous
-- read-modify-write of the received_disk_parts JSON column which races under
-- parallel uploads). Mirrors v86_system_upload_parts so the game upload path
-- can be parallelized the same way the system image upload already is.

CREATE TABLE project_v86_received_disk_parts (
    upload_id TEXT NOT NULL
        REFERENCES project_v86_upload_sessions(id) ON DELETE CASCADE,
    part_index INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (upload_id, part_index)
);

CREATE INDEX idx_project_v86_received_disk_parts_session
    ON project_v86_received_disk_parts(upload_id);