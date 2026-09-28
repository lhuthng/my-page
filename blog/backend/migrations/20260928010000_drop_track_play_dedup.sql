-- The chapter counter measures listening time now: the player reports once per
-- ten seconds of real playback and every report is counted, with no cap. The
-- per-listener-per-day dedup that used to gate the write has nothing left to
-- do, and an unreferenced table that still holds client identities is a
-- liability rather than a record.
--
-- `audiobook_tracks.play_count` keeps its meaning as the materialized total and
-- is deliberately left alone: the figures already in it are the old deduped
-- ones (a handful), and zeroing them would throw away the only history there
-- is. They are negligible next to what accumulates from here.
--
-- Nothing else reads the table: the beacon's listener hash now lives only in
-- the handler's in-memory rate-limit window.

DROP TABLE IF EXISTS audiobook_track_plays;
