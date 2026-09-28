-- Chapter play counts for audiobooks.
--
-- Measurement model: a play is counted when the public player reports that a
-- listener has actually listened to a chapter, and the counter moves at most
-- once per listener per track per UTC day. The dedup table is the source of
-- truth for that rule -- the counter on the track row is just the materialized
-- sum, incremented only when a dedup row was newly inserted.
--
-- `listener` is a salted, truncated SHA-256 of the best-effort client address;
-- raw IPs are never stored.

ALTER TABLE audiobook_tracks ADD COLUMN play_count INTEGER NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS audiobook_track_plays (
    track_id INTEGER NOT NULL REFERENCES audiobook_tracks(id) ON DELETE CASCADE,
    listener TEXT NOT NULL,
    day TEXT NOT NULL,

    created_at TEXT DEFAULT CURRENT_TIMESTAMP,

    PRIMARY KEY (track_id, listener, day)
);

CREATE INDEX IF NOT EXISTS idx_audiobook_track_plays_listener
    ON audiobook_track_plays (listener, day);
