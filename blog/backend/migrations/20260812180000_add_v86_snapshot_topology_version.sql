-- Track which machine layout a snapshot was captured on.
--
-- A restored state assumes the exact set of emulated devices it was captured
-- with. v86 guards most per-device restores, so a removed device does not
-- throw outright -- it silently leaves the guest holding drivers for hardware
-- that is no longer there, which is harder to diagnose than a clean failure.
--
-- Bumping V86_TOPOLOGY_VERSION in the backend therefore retires every older
-- snapshot the same way the disk and memory gates do: it stops being served
-- and the player cold-boots instead.
--
-- 1 = base + game disk, cdrom, floppy, ne2k NIC
-- 2 = same without the NIC, which was configured with no relay and only cost
--     the guest time initialising hardware it could never use

ALTER TABLE project_v86_snapshots
    ADD COLUMN topology_version INTEGER NOT NULL DEFAULT 1;
