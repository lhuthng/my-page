CREATE INDEX IF NOT EXISTS idx_posts_listing_created
ON posts(content_kind, status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_posts_listing_updated
ON posts(content_kind, status, updated_at DESC);
