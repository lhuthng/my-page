ALTER TABLE posts RENAME COLUMN cover_image_id TO cover_media_id;
ALTER TABLE posts ADD COLUMN og_image_seconds INTEGER NOT NULL DEFAULT 0;
