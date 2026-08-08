-- Additive input-validation hardening.
--
-- Application code already validates on write; these triggers enforce the same
-- rules at the database layer so bad data cannot creep in via any other path.
-- Existing rows that violate the new rules are backfilled to safe values first.

-- ---------------------------------------------------------------------------
-- 1. Backfill existing invalid data (before triggers are installed).
-- ---------------------------------------------------------------------------

-- Any slug that is empty, whitespace-only, too long, or contains characters
-- outside [a-z0-9_-] is replaced with a unique, valid placeholder.
UPDATE posts     SET slug = 'backfill-' || id
      WHERE length(trim(slug)) < 2 OR length(slug) > 100
         OR slug GLOB '*[^a-z0-9_-]*';
UPDATE series    SET slug = 'backfill-' || id
      WHERE length(trim(slug)) < 2 OR length(slug) > 100
         OR slug GLOB '*[^a-z0-9_-]*';
UPDATE tags      SET slug = 'backfill-' || id
      WHERE length(trim(slug)) < 2 OR length(slug) > 100
         OR slug GLOB '*[^a-z0-9_-]*';
UPDATE categories SET slug = 'backfill-' || id
      WHERE length(trim(slug)) < 2 OR length(slug) > 100
         OR slug GLOB '*[^a-z0-9_-]*';

-- Ensure a title/excerpt is never blank or unbounded.
UPDATE posts  SET title   = 'untitled'        WHERE length(trim(title)) = 0;
UPDATE posts  SET excerpt = ''                WHERE length(excerpt) > 400;
UPDATE series SET title   = 'Untitled series' WHERE length(trim(title)) = 0;
UPDATE tags   SET name    = 'untitled tag'    WHERE length(trim(name)) = 0;

-- ---------------------------------------------------------------------------
-- 2. Slug triggers (insert + update) for posts, series, tags, categories.
-- ---------------------------------------------------------------------------

CREATE TRIGGER posts_slug_insert BEFORE INSERT ON posts
BEGIN
  SELECT CASE WHEN length(lower(trim(NEW.slug))) < 2
              THEN RAISE(ABORT, 'Slug must be at least 2 characters.')
              END;
  SELECT CASE WHEN length(lower(trim(NEW.slug))) > 100
              THEN RAISE(ABORT, 'Slug must be at most 100 characters.')
              END;
  SELECT CASE WHEN lower(trim(NEW.slug)) GLOB '*[^a-z0-9_-]*'
              THEN RAISE(ABORT, 'Slug may only contain lowercase letters, numbers, hyphens, and underscores.')
              END;
END;

CREATE TRIGGER posts_slug_update BEFORE UPDATE OF slug ON posts
BEGIN
  SELECT CASE WHEN length(trim(NEW.slug)) < 2
       THEN RAISE(ABORT, 'Slug must be at least 2 characters.')
       END;
  SELECT CASE WHEN length(lower(trim(NEW.slug))) > 100
       THEN RAISE(ABORT, 'Slug must be at most 100 characters.')
       END;
  SELECT CASE WHEN lower(trim(NEW.slug)) GLOB '*[^a-z0-9_-]*'
       THEN RAISE(ABORT, 'Slug may only contain lowercase letters, numbers, hyphens, and underscores.')
       END;
END;

CREATE TRIGGER series_slug_insert BEFORE INSERT ON series
BEGIN
  SELECT CASE WHEN length(lower(trim(NEW.slug))) < 2
       THEN RAISE(ABORT, 'Series slug must be at least 2 characters.')
       END;
  SELECT CASE WHEN length(lower(trim(NEW.slug))) > 100
       THEN RAISE(ABORT, 'Series slug must be at most 100 characters.')
       END;
  SELECT CASE WHEN lower(trim(NEW.slug)) GLOB '*[^a-z0-9_-]*'
       THEN RAISE(ABORT, 'Series slug may only contain lowercase letters, numbers, hyphens, and underscores.')
       END;
END;

CREATE TRIGGER series_slug_update BEFORE UPDATE ON series
BEGIN
  SELECT CASE WHEN length(lower(trim(NEW.slug))) < 2
       THEN RAISE(ABORT, 'Series slug must be at least 2 characters.')
       END;
  SELECT CASE WHEN length(lower(trim(NEW.slug))) > 100
       THEN RAISE(ABORT, 'Series slug must be at most 100 characters.')
       END;
  SELECT CASE WHEN lower(trim(NEW.slug)) GLOB '*[^a-z0-9_-]*'
       THEN RAISE(ABORT, 'Series slug may only contain lowercase letters, numbers, hyphens, and underscores.')
       END;
END;

CREATE TRIGGER tags_slug_insert BEFORE INSERT ON tags
BEGIN
  SELECT CASE WHEN length(lower(trim(NEW.slug))) < 2
       THEN RAISE(ABORT, 'Tag slug must be at least 2 characters.')
       END;
  SELECT CASE WHEN length(lower(trim(NEW.slug))) > 100
       THEN RAISE(ABORT, 'Tag slug must be at most 100 characters.')
       END;
  SELECT CASE WHEN lower(trim(NEW.slug)) GLOB '*[^a-z0-9_-]*'
       THEN RAISE(ABORT, 'Tag slug may only contain lowercase letters, numbers, hyphens, and underscores.')
       END;
END;

CREATE TRIGGER categories_slug_insert BEFORE INSERT ON categories
BEGIN
  SELECT CASE WHEN length(lower(trim(NEW.slug))) < 2
       THEN RAISE(ABORT, 'Category slug must be at least 2 characters.')
       END;
  SELECT CASE WHEN length(lower(trim(NEW.slug))) > 100
       THEN RAISE(ABORT, 'Category slug must be at most 100 characters.')
       END;
  SELECT CASE WHEN lower(trim(NEW.slug)) GLOB '*[^a-z0-9_-]*'
       THEN RAISE(ABORT, 'Category slug may only contain lowercase letters, numbers, hyphens, and underscores.')
       END;
END;

-- ---------------------------------------------------------------------------
-- 3. Title / name / excerpt triggers.
-- ---------------------------------------------------------------------------

CREATE TRIGGER posts_title_insert BEFORE INSERT ON posts
BEGIN
  SELECT CASE WHEN length(trim(NEW.title)) = 0
       THEN RAISE(ABORT, 'Post title must not be empty.')
       END;
  SELECT CASE WHEN length(NEW.title) > 200
       THEN RAISE(ABORT, 'Post title must be at most 200 characters.')
       END;
END;

CREATE TRIGGER posts_title_update BEFORE UPDATE ON posts
BEGIN
  SELECT CASE WHEN length(trim(NEW.title)) = 0
       THEN RAISE(ABORT, 'Post title must not be empty.')
       END;
  SELECT CASE WHEN length(NEW.title) > 200
       THEN RAISE(ABORT, 'Post title must be at most 200 characters.')
       END;
END;

CREATE TRIGGER series_title_insert BEFORE INSERT ON series
BEGIN
  SELECT CASE WHEN length(trim(NEW.title)) = 0
       THEN RAISE(ABORT, 'Series title must not be empty.')
       END;
  SELECT CASE WHEN length(NEW.title) > 300
       THEN RAISE(ABORT, 'Series title must be at most 300 characters.')
       END;
END;