-- Add migration script here
CREATE TABLE IF NOT EXISTS series_post (
    series_id INTEGER,
    post_id INTEGER,
    number INTEGER,

    PRIMARY KEY (series_id, post_id),
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE,
    FOREIGN KEY (series_id) REFERENCES series(id) ON DELETE CASCADE
);
