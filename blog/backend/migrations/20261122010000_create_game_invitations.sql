-- Invitation codes for 'invited' games. A code is a random 4x4 base32 string
-- the author hands out; the database keeps only its SHA-256 hash, so a leaked
-- table leaks no playable codes. max_uses is the capacity: 1 = single-use,
-- N = N redemptions, 0 = unlimited. Revoking is soft (revoked_at) and kills
-- every guest token that points at the code, because token verification
-- re-reads this row.
--
-- A redemption by a registered user also lands in game_entitlements: those
-- grants are identity-bound (they work on every device, no cookie needed).
-- Guests get no row anywhere - their proof is a signed token in a cookie, so
-- guest access stays stateless on the server.

CREATE TABLE IF NOT EXISTS game_invitations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL,
    code_hash TEXT NOT NULL UNIQUE,
    code_hint TEXT NOT NULL DEFAULT '',
    max_uses INTEGER NOT NULL DEFAULT 1 CHECK (max_uses >= 0),
    uses_count INTEGER NOT NULL DEFAULT 0 CHECK (uses_count >= 0),
    note TEXT NOT NULL DEFAULT '',
    created_by INTEGER NOT NULL,
    revoked_at TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_game_invitations_game ON game_invitations(game_id);

CREATE TABLE IF NOT EXISTS game_entitlements (
    game_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    invitation_id INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (game_id, user_id),
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (invitation_id) REFERENCES game_invitations(id) ON DELETE CASCADE
);
