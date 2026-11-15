-- Per-book switch for the author API.
--
-- The API is exposed book by book, never deployment-wide: an author turns it on
-- for one audiobook and that book alone becomes reachable with an `abk_` key.
-- Default 0, so a book is not reachable until someone deliberately switches it
-- on. Turning it off leaves the key rows in place but the guard refuses them,
-- so re-enabling restores the same keys instead of minting new ones.
--
-- This is a separate migration rather than an edit to
-- `20261115000000_create_author_api_keys.sql` because sqlx checksums applied
-- migrations: editing one that has already run makes the migrator refuse to
-- start.
ALTER TABLE audiobooks
    ADD COLUMN author_api_enabled INTEGER NOT NULL DEFAULT 0;
