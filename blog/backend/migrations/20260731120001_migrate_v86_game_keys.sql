-- Rewrite legacy per-upload v86 game storage keys to the content-addressed
-- layout used by the R2 pipeline (v86/games/zips/{zip_sha}.zip and
-- v86/games/{iso_sha}) so reuse-source and deletion work for projects that
-- predate the R2 refactor.
UPDATE project_v86_games
   SET zip_storage_key = 'v86/games/zips/' || zip_sha256 || '.zip',
       iso_storage_key = 'v86/games/' || iso_sha256
 WHERE zip_storage_key NOT LIKE 'v86/games/zips/%';
