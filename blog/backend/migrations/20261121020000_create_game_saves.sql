-- Unified per-user save files for the payload launchers ('shimkit',
-- 'wasmshell'). A save is one guest-written file, keyed by its name inside the
-- game's own save set (a shim-kit game's profile.dat, Vice City's GTAVCsf1.b),
-- so a game's full save set is several rows; the browser always keeps its own
-- copy, and this table is the account's copy in the cloud.
--
-- Guests never write here: their saves stay in the browser, exactly like the
-- v86 floppies they run beside. game_v86_saves keeps its own shape (one floppy
-- per game per user); this table is the general one the other launchers use.

CREATE TABLE IF NOT EXISTS game_saves (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    save_key TEXT NOT NULL,
    storage_key TEXT NOT NULL UNIQUE,
    size_bytes INTEGER NOT NULL CHECK (size_bytes > 0),
    sha256 TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(game_id, user_id, save_key)
);

CREATE INDEX idx_game_saves_user ON game_saves(user_id);
