-- Games: add the payload launcher kinds. 'shimkit' is a win32-shim-kit binary
-- rehost payload (the kit's x86 interpreter runs the game's own exe); 'wasmshell'
-- is a source port's own static web shell (its wasm, its page, its asset
-- streaming). Both upload through the same chunked zip pipeline and serve as
-- unpacked file trees; they differ only in which player plays them.
-- SQLite cannot alter a CHECK constraint, so the table is rebuilt in place.

PRAGMA foreign_keys=off;

CREATE TABLE games_new (
    id INTEGER PRIMARY KEY,
    post_id INTEGER NOT NULL UNIQUE,

    -- The launcher that plays this game.
    launcher_type TEXT NOT NULL DEFAULT 'html5'
        CHECK (launcher_type IN ('html5', 'webgl', 'jsdos', 'v86', 'embed', 'download', 'video', 'shimkit', 'wasmshell')),
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
    id, post_id, launcher_type, demo_width, demo_height, demo_url,
    instruction, cheatcode, story, created_at, updated_at
)
SELECT
    id, post_id, launcher_type, demo_width, demo_height, demo_url,
    instruction, cheatcode, story, created_at, updated_at
FROM games;

DROP TABLE games;
ALTER TABLE games_new RENAME TO games;

CREATE INDEX IF NOT EXISTS idx_games_post_id ON games(post_id);

PRAGMA foreign_keys=on;
