-- Track R2 multipart uploads for v86 upload sessions so chunks can be
-- relayed straight to R2 instead of buffered on the VM disk.
ALTER TABLE v86_system_upload_sessions ADD COLUMN r2_upload_id TEXT;
ALTER TABLE v86_system_upload_sessions ADD COLUMN r2_part_etags TEXT;

ALTER TABLE project_v86_upload_sessions ADD COLUMN r2_upload_id TEXT;
ALTER TABLE project_v86_upload_sessions ADD COLUMN r2_part_etags TEXT;
