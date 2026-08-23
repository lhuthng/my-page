-- ---------------------------------------------------------------------------
-- Cleanup: drop the legacy project-scoped v86/jsdos tables. Their data was
-- migrated to the game-scoped tables (20260820020000) and the project
-- endpoints that served them are gone: projects now delegate their demo to a
-- game instead of carrying a launcher.
--
-- Kept on purpose:
--   * project_v86_upload_sessions / project_v86_staged_variants — despite the
--     legacy name these are the shared v86 staging pipeline used by games.
-- ---------------------------------------------------------------------------

DROP TABLE IF EXISTS project_v86_snapshots;
DROP TABLE IF EXISTS project_v86_variants;
DROP TABLE IF EXISTS project_v86_games;
DROP TABLE IF EXISTS project_jsdos_bundles;
DROP TABLE IF EXISTS project_jsdos_upload_sessions;
DROP TABLE IF EXISTS v86_snapshot_upload_sessions;
DROP TABLE IF EXISTS v86_saves;
