-- Optional per-system machine specs (JSON), extending memory_size_mb:
--   { "vga_memory_size_mb": 16, "screen_width": 800, "screen_height": 600 }
-- All keys optional; missing keys fall back to platform defaults. VRAM is
-- part of the snapshot topology (changing it stales snapshots); resolution
-- only sizes the screen container — the guest picks its own video mode.
ALTER TABLE v86_systems ADD COLUMN specs TEXT;
