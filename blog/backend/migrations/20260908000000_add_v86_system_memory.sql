-- Per-system RAM for v86 machines. Windows 9x runs fine at the old global
-- 64 MB, but XP wants 256-512 MB. Memory is part of the snapshot topology:
-- snapshots captured under one size stop matching (stale → cold boot) when
-- a system's value changes, which is the intended safety behavior.
ALTER TABLE v86_systems ADD COLUMN memory_size_mb INTEGER NOT NULL DEFAULT 64;

-- The upload session carries the chosen size so complete_system_upload can
-- create the system row without the client re-sending it.
ALTER TABLE v86_system_upload_sessions ADD COLUMN memory_size_mb INTEGER;
