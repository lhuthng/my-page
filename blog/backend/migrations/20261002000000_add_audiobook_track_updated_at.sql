-- Per-chapter last-modified time.
--
-- The book already carries `audiobooks.updated_at`, but that answers a question
-- an author of a long book never asks. "When did this book change?" is answered
-- by any of its 24 chapters; "which chapters did I actually touch?" is the one
-- that helps, and it needs a timestamp per row.
--
-- SQLite refuses a non-constant DEFAULT in ALTER TABLE ADD COLUMN, so
-- `DEFAULT CURRENT_TIMESTAMP` is not available here. The column is added bare,
-- backfilled from `created_at` (which is what a never-edited chapter's
-- modified time genuinely is), and written explicitly from here on by the one
-- INSERT and by the content-changing UPDATEs in `persistence/audiobook/tracks.rs`.
-- Reads use COALESCE(updated_at, created_at) so a row that somehow escaped both
-- still reports a time rather than nothing.
ALTER TABLE audiobook_tracks ADD COLUMN updated_at TEXT;

UPDATE audiobook_tracks SET updated_at = created_at;

-- Reorder is deliberately not a modification.
--
-- Nothing below writes `updated_at`, and that is the point: renumbering a
-- chapter moves it in the book without changing the chapter. If a drag bumped
-- every shifted row, one reorder would mark the whole list as freshly edited
-- and the column would stop distinguishing anything. `play_count` is likewise
-- a listener counter rather than an edit, so a popular chapter must not read as
-- a recently-edited one.
