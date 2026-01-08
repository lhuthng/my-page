-- Add migration script here
DROP TABLE IF EXISTS series;

CREATE TABLE series (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,

    user_id INTEGER NOT NULL REFERENCES users(id),
    cover_image_id INTEGER REFERENCES media(id)
);
