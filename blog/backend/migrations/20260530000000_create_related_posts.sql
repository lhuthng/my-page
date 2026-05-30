CREATE TABLE IF NOT EXISTS related_posts (
    post_id INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    related_post_id INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    display_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (post_id, related_post_id),
    CHECK (post_id != related_post_id)
);

CREATE INDEX IF NOT EXISTS idx_related_posts_post_id ON related_posts(post_id);
