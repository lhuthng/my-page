-- Migration: Add cover_image_id to user_meta

ALTER TABLE user_meta
ADD COLUMN avatar_image_id INTEGER REFERENCES media(id) ON DELETE SET NULL;
