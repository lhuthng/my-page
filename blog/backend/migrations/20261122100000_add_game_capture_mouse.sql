-- Per-game input preference: when set, the player grants the frame the
-- `pointer-lock` permission, so games whose engine takes the cursor
-- exclusively while playing (camera control) can actually capture it.
-- A plain ADD COLUMN keeps the table's history intact; the constant default
-- means every existing game keeps today's behavior.
ALTER TABLE games ADD COLUMN capture_mouse INTEGER NOT NULL DEFAULT 0;
