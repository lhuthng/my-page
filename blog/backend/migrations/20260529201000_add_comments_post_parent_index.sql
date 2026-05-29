CREATE INDEX IF NOT EXISTS idx_comments_post_parent
ON comments(post_id, parent_id);
