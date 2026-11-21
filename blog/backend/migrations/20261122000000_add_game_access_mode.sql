-- Games: who may play. 'open' is today's behavior - anyone with the URL plays.
-- 'invited' gates every serving route behind a consumed invitation: registered
-- redeemers get an account-bound grant, guests get a signed cookie token, and
-- the author always plays their own game.
-- SQLite cannot alter a CHECK constraint, so the table is rebuilt in place.

PRAGMA foreign_keys=off;

CREATE TABLE games_new (
    id INTEGER PRIMARY KEY,
    post_id INTEGER NOT NULL UNIQUE,

    -- The launcher that plays this game.
    launcher_type TEXT NOT NULL DEFAULT 'html5'
        CHECK (launcher_type IN ('html5', 'webgl', 'jsdos', 'v86', 'embed', 'download', 'video', 'shimkit', 'wasmshell')),

    -- Who may play: 'open' or 'invited' (a consumed invitation code required).
    access_mode TEXT NOT NULL DEFAULT 'open'
        CHECK (access_mode IN ('open', 'invited')),

    demo_width TEXT,
    demo_height TEXT,
    demo_url TEXT,

    -- "Many bodies" instead of a single post body.
    instruction TEXT NOT NULL DEFAULT '',
    cheatcode TEXT NOT NULL DEFAULT '',
    story TEXT NOT NULL DEFAULT '',

    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE
);

INSERT INTO games_new (
    id, post_id, launcher_type, access_mode, demo_width, demo_height, demo_url,
    instruction, cheatcode, story, created_at, updated_at
)
SELECT
    id, post_id, launcher_type, 'open', demo_width, demo_height, demo_url,
    instruction, cheatcode, story, created_at, updated_at
FROM games;

DROP TABLE games;
ALTER TABLE games_new RENAME TO games;

CREATE INDEX IF NOT EXISTS idx_games_post_id ON games(post_id);

PRAGMA foreign_keys=on;
