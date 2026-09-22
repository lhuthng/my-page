// Compile-time limits and wire-format constants for the v86 feature.
pub(super) const MANIFEST_MAX_BYTES: usize = 64 * 1024;

/// Capacity of the 1.44 MB FAT12 floppy used as the save transport box.
pub(super) const V86_SAVE_FLOPPY_BYTES: usize = 1474560;
/// Hard cap for a floppy save upload so the body limit stays bounded.
pub(super) const V86_SAVE_MAX_UPLOAD_BYTES: usize = 2 * 1024 * 1024;
/// Per-user cooldown between cloud saves (matches the client).
pub(super) const V86_SAVE_RATE_LIMIT_MS: u64 = 30_000;

/// RAM / VGA the player hands to v86. A restored `initial_state` only makes
/// sense against the same sizes it was captured with, so snapshots record
/// these and `runtime_descriptor` refuses to serve a snapshot that disagrees.
pub(super) const V86_MEMORY_SIZE: u64 = 64 * 1024 * 1024;
pub(super) const V86_VGA_MEMORY_SIZE: u64 = 8 * 1024 * 1024;

/// v86's `save_state()` container format version (`libv86.js` throws
/// "Version mismatch" on anything else). Bumping the vendored v86 build
/// invalidates every stored snapshot, which degrades to a normal cold boot.
pub(super) const V86_STATE_VERSION: i64 = 6;

/// Which set of emulated devices the player builds. A restored state assumes
/// the layout it was captured on, so changing the device list here retires
/// every older snapshot rather than restoring one into a machine it does not
/// match. Keep in sync with V86_TOPOLOGY_VERSION in V86Player.svelte.js.
pub(super) const V86_TOPOLOGY_VERSION: i64 = 2;

/// Zstandard frame magic, little-endian. Snapshots are compressed in the
/// browser and stored verbatim: v86's `restore_state` sniffs this magic and
/// decompresses internally, so nothing on the server ever unpacks them.
pub(super) const ZSTD_MAGIC: [u8; 4] = [0x28, 0xb5, 0x2f, 0xfd];

/// Hard cap for a snapshot upload. A 64 MB machine dumps ~72 MB raw, which
/// compresses to roughly 10-25 MB; 192 MB leaves generous headroom.
pub(super) const V86_SNAPSHOT_MAX_BYTES: u64 = 192 * 1024 * 1024;

pub(super) const SAVE_FILE_MAX_LEN: usize = 260;
pub(super) const SAVE_FILE_MAX_COUNT: usize = 64;
