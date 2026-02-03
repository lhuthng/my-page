-- 1) Create post_stats table
CREATE TABLE IF NOT EXISTS post_stats (
  post_id INTEGER PRIMARY KEY,
  views INTEGER NOT NULL DEFAULT 0,
  likes INTEGER NOT NULL DEFAULT 0,
  comments_count INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE
);

-- 2) Backfill comments_count from existing comments (only non-deleted ones)
-- Inserts a row per post with the computed comments_count (0 if none).
INSERT INTO post_stats (post_id, comments_count, updated_at)
SELECT p.id, COALESCE(c.cnt, 0), CURRENT_TIMESTAMP
FROM posts p
LEFT JOIN (
  SELECT post_id, COUNT(*) AS cnt
  FROM comments
  WHERE is_deleted = 0
  GROUP BY post_id
) c ON p.id = c.post_id
ON CONFLICT(post_id) DO UPDATE
  SET comments_count = excluded.comments_count,
      updated_at = excluded.updated_at;

-- 3) Triggers to keep comments_count in sync.
-- Note: comments table uses soft-delete via is_deleted.

-- After insert: increment count when the new comment is not deleted
CREATE TRIGGER IF NOT EXISTS comments_after_insert_post_stats
AFTER INSERT ON comments
WHEN NEW.is_deleted = 0
BEGIN
  INSERT INTO post_stats (post_id, comments_count, updated_at)
    VALUES (NEW.post_id, 1, CURRENT_TIMESTAMP)
  ON CONFLICT(post_id) DO UPDATE
    SET comments_count = post_stats.comments_count + 1,
        updated_at = CURRENT_TIMESTAMP;
END;

-- After delete (hard delete): decrement if the deleted comment was not marked deleted
CREATE TRIGGER IF NOT EXISTS comments_after_delete_post_stats
AFTER DELETE ON comments
WHEN OLD.is_deleted = 0
BEGIN
  UPDATE post_stats
    SET comments_count = CASE WHEN comments_count > 0 THEN comments_count - 1 ELSE 0 END,
        updated_at = CURRENT_TIMESTAMP
    WHERE post_id = OLD.post_id;
END;

-- After update: handle soft-delete toggle (0 -> 1) => decrement
CREATE TRIGGER IF NOT EXISTS comments_after_update_soft_delete
AFTER UPDATE ON comments
WHEN OLD.is_deleted = 0 AND NEW.is_deleted = 1
BEGIN
  UPDATE post_stats
    SET comments_count = CASE WHEN comments_count > 0 THEN comments_count - 1 ELSE 0 END,
        updated_at = CURRENT_TIMESTAMP
    WHERE post_id = OLD.post_id;
END;

-- After update: handle undelete toggle (1 -> 0) => increment
CREATE TRIGGER IF NOT EXISTS comments_after_update_undelete
AFTER UPDATE ON comments
WHEN OLD.is_deleted = 1 AND NEW.is_deleted = 0
BEGIN
  INSERT INTO post_stats (post_id, comments_count, updated_at)
    VALUES (NEW.post_id, 1, CURRENT_TIMESTAMP)
  ON CONFLICT(post_id) DO UPDATE
    SET comments_count = post_stats.comments_count + 1,
        updated_at = CURRENT_TIMESTAMP;
END;

-- After update: comment moved to different post (and both versions are not deleted)
CREATE TRIGGER IF NOT EXISTS comments_after_update_move_post
AFTER UPDATE ON comments
WHEN OLD.post_id != NEW.post_id AND OLD.is_deleted = 0 AND NEW.is_deleted = 0
BEGIN
  -- decrement count on old post
  UPDATE post_stats
    SET comments_count = CASE WHEN comments_count > 0 THEN comments_count - 1 ELSE 0 END,
        updated_at = CURRENT_TIMESTAMP
    WHERE post_id = OLD.post_id;

  -- increment (or create) row for new post
  INSERT INTO post_stats (post_id, comments_count, updated_at)
    VALUES (NEW.post_id, 1, CURRENT_TIMESTAMP)
  ON CONFLICT(post_id) DO UPDATE
    SET comments_count = post_stats.comments_count + 1,
        updated_at = CURRENT_TIMESTAMP;
END;
