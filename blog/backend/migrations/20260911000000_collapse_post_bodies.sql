-- Collapse the two body columns into one.
--
-- `posts.draft` began as "the draft body" but became the primary editing
-- surface, while `posts.content` is only a snapshot written at publish time
-- (`content := draft`). The name is misleading — `draft` is *the* body, and
-- `content` is the published copy.
--
-- In practice the two have never diverged: every row holds byte-identical
-- values, so the staging column is pure duplication. It is also the only
-- staged field in the schema — title, slug, excerpt and tags are all written
-- straight to the live row — so removing it makes the model consistent:
-- save = live, publish = visibility.
--
-- Ordered so nothing is lost: backfill first, verify second, drop last.

-- 1. Anything never published carries its body in `draft` only (create inserts
--    `draft` and leaves `content` NULL). Move it across.
UPDATE posts
SET content = draft
WHERE content IS NULL;

-- 2. Refuse to continue if a row still has two different bodies — that would
--    be an unreleased edit, and dropping the column would silently discard it.
--    Inserting a row violates the CHECK, aborting the whole migration (and
--    rolling it back) with a readable constraint name.
CREATE TABLE migration_guard_divergent_bodies (
    msg TEXT NOT NULL
        CONSTRAINT divergent_post_bodies_would_be_lost
        CHECK (msg = 'no divergent bodies')
);

INSERT INTO migration_guard_divergent_bodies (msg)
SELECT 'divergent'
FROM posts
WHERE COALESCE(content, '') <> COALESCE(draft, '')
LIMIT 1;

DROP TABLE migration_guard_divergent_bodies;

-- 3. Drop the staging column. SQLite 3.35+; the bundled build is 3.46.
ALTER TABLE posts DROP COLUMN draft;
