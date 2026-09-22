// Wire types for the v86 feature: request bodies, query params, and
// response shapes. Handlers do not declare structs inline.
use serde::{Deserialize, Serialize};

/// Optional per-system machine specs (`v86_systems.specs`, JSON). Every key
/// is optional; anything missing falls back to the platform defaults. VRAM
/// participates in the snapshot topology, resolution does not (it only sizes
/// the screen container — the guest picks its own video mode).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct V86SystemSpecs {
    pub vga_memory_size_mb: Option<i64>,
    pub screen_width: Option<i64>,
    pub screen_height: Option<i64>,
}

#[derive(Serialize)]
pub struct V86SystemVersionResponse {
    pub id: i64,
    pub version_number: i64,
    pub original_file_name: String,
    pub size_bytes: i64,
    pub sha256: String,
    pub chunk_size_bytes: i64,
    pub chunk_count: i64,
}

#[derive(Serialize)]
pub struct V86SystemResponse {
    pub id: i64,
    pub name: String,
    pub platform_key: String,
    pub memory_size_mb: i64,
    pub specs: V86SystemSpecs,
    pub is_active: bool,
    pub is_default: bool,
    pub current_version: i64,
    pub pending_build: bool,
    pub project_count: i64,
    pub published_project_count: i64,
    pub versions: Vec<V86SystemVersionResponse>,
}

#[derive(Deserialize)]
pub struct StartSystemUploadRequest {
    pub system_id: Option<i64>,
    pub expected_current_version: Option<i64>,
    pub name: String,
    pub platform_key: String,
    pub file_name: String,
    pub size_bytes: u64,
    pub sha256: String,
    /// Guest RAM in MB (per-system). Defaults to the legacy 64 MB.
    #[serde(default)]
    pub memory_size_mb: Option<i64>,
}

#[derive(Serialize)]
pub struct StartSystemUploadResponse {
    pub upload_id: String,
    pub reuse: bool,
    pub chunk_size_bytes: u64,
    pub chunk_count: u64,
    pub storage_key: Option<String>,
}

/// The client's plan for the game disk (D:) it built locally. `None` means the
/// upload carries no ZIP (a manifest-only edit) and the source project's
/// stored disk is reused unchanged.
#[derive(Debug, Deserialize)]
pub struct GameDiskPlan {
    pub sha256: String,
    pub size_bytes: u64,
}

/// A client-built launcher CD (E:) plan for one variant. The SHA-256 is over
/// the finished ISO bytes; the server verifies it when the bytes arrive.
#[derive(Debug, Deserialize)]
pub struct GameVariantPlan {
    pub index: i32,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Deserialize)]
pub struct GameBuildPlans {
    pub disk: Option<GameDiskPlan>,
    pub variants: Vec<GameVariantPlan>,
}

#[derive(Deserialize)]
pub struct StartGameUploadRequest {
    pub source_project_id: Option<i64>,
    pub system_version_id: i64,
    pub expected_artifact_revision: i64,
    pub manifest: String,
    pub plans: GameBuildPlans,
}

/// Whether the client must upload each artifact. Reused artifacts already
/// exist content-addressed in R2 and are skipped.
#[derive(Serialize)]
pub struct DiskUploadSpec {
    pub sha256: String,
    pub size_bytes: u64,
    pub chunk_size_bytes: u64,
    pub chunk_count: u64,
    pub reuse: bool,
}

#[derive(Serialize)]
pub struct VariantUploadSpec {
    pub index: i32,
    pub sha256: String,
    pub size_bytes: u64,
    pub reuse: bool,
}

#[derive(Serialize)]
pub struct StartGameUploadResponse {
    pub upload_id: String,
    pub disk: Option<DiskUploadSpec>,
    pub variants: Vec<VariantUploadSpec>,
}

#[derive(Serialize)]
pub struct StartUploadResponse {
    pub upload_id: String,
    pub chunk_size_bytes: u64,
    pub next_chunk_index: u64,
    pub expected_size_bytes: u64,
    pub upload_required: bool,
}

#[derive(Serialize)]
pub struct ChunkUploadResponse {
    pub received_size_bytes: u64,
    pub next_chunk_index: u64,
}

#[derive(Deserialize)]
pub struct StartSnapshotUploadRequest {
    pub game_id: i64,
    /// 0 captures the project-wide machine with no disc in the drive; a
    /// variant index captures that variant's launcher CD already mounted.
    /// The player has to rebuild the same topology, so this decides whether
    /// the disc is passed at construction or inserted after the restore.
    #[serde(default)]
    pub variant_index: i32,
    /// Required when `variant_index` > 0: the disc that was in the drive.
    pub iso_sha256: Option<String>,
    pub system_version_id: i64,
    /// sha256 of the game disk the snapshot was captured against. Together
    /// with `system_version_id` this pins the state to the exact images whose
    /// dirty blocks are baked into it.
    pub game_disk_sha256: String,
    /// Size of the compressed blob about to be uploaded.
    pub size_bytes: u64,
    /// Size of the raw `save_state()` output, for display only.
    pub raw_size_bytes: u64,
    /// sha256 of the compressed blob, verified server-side on completion.
    pub sha256: String,
    pub state_version: i64,
    /// Which device layout the capturing player built. Rejected unless it
    /// matches what this server currently serves.
    #[serde(default)]
    pub topology_version: i64,
    pub memory_size: u64,
    pub vga_memory_size: u64,
}

#[derive(Serialize)]
pub struct SnapshotStatusResponse {
    pub variant_index: i32,
    pub exists: bool,
    /// True when a snapshot row exists but no longer matches the project's
    /// current disks / disc / state version / memory size, so it is not served.
    pub stale: bool,
    pub size_bytes: Option<u64>,
    pub raw_size_bytes: Option<u64>,
    pub created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSystemRequest {
    pub name: Option<String>,
    pub is_active: Option<bool>,
    pub is_default: Option<bool>,
    pub expected_current_version: Option<i64>,
    /// Guest RAM in MB. Changing it marks the system's snapshots stale.
    pub memory_size_mb: Option<i64>,
    /// Machine specs (VRAM, suggested screen size). VRAM changes mark
    /// snapshots stale; resolution is cosmetic.
    pub specs: Option<V86SystemSpecs>,
}

#[derive(Deserialize)]
pub struct ActiveSystemsQuery {
    pub include_version_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct V86RuntimeDescriptor {
    pub platform_key: String,
    pub system_name: String,
    pub system_version_id: i64,
    pub artifact_revision: i64,
    pub manifest_sha256: String,
    pub slug: String,
    pub memory_size: u64,
    pub vga_memory_size: u64,
    pub display_width: String,
    pub display_height: String,
    pub chunk_size_bytes: u64,
    pub base_size_bytes: u64,
    pub base_sha256: String,
    pub base_url: String,
    pub game_size_bytes: u64,
    pub game_sha256: String,
    pub game_url: String,
    pub iso_size_bytes: u64,
    pub iso_sha256: String,
    pub iso_url: String,
    pub variants: Vec<VariantDescriptor>,
    pub save_supported: bool,
    pub save_max_bytes: u64,
    /// A pre-booted `initial_state` blob, present only when one was captured
    /// against exactly this base disk, game disk, state version and memory
    /// size. `None` means the player cold-boots, which is always safe.
    pub snapshot_url: Option<String>,
    pub snapshot_size_bytes: Option<u64>,
    pub snapshot_sha256: Option<String>,
    /// Whether the emulated mouse's Y axis is inverted. v86's built-in mouse
    /// adapter negates movementY; this restores the browser's natural
    /// direction for guests whose drivers expect it.
    pub revert_mouse_y: bool,
    /// Per-project mouse speed multiplier applied on top of the visitor's own
    /// sensitivity slider. Defaults to 1.0.
    pub mouse_speed: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct VariantDescriptor {
    pub index: i32,
    pub name: String,
    pub exe: String,
    pub args: String,
    pub iso_url: String,
    pub iso_size_bytes: u64,
    pub iso_sha256: String,
    /// A snapshot captured with this variant's disc already mounted. When set
    /// the player restores it with that same disc attached at construction;
    /// when absent it falls back to the project-wide snapshot, then to a cold
    /// boot.
    pub snapshot_url: Option<String>,
    pub snapshot_size_bytes: Option<u64>,
    pub snapshot_sha256: Option<String>,
}

#[derive(Serialize)]
pub struct PublicSystemVersion {
    pub id: i64,
    pub version_number: i64,
    pub system_name: String,
    pub platform_key: String,
    /// Guest RAM in MB; the sandbox boots the machine with this much.
    pub memory_size_mb: i64,
    /// Optional machine specs (VRAM, suggested screen size) for the sandbox.
    pub specs: V86SystemSpecs,
    /// VRAM the sandbox should boot with, resolved from specs/platform.
    pub vga_memory_size_mb: i64,
    pub sha256: String,
    pub storage_key: String,
    /// The disk image URL the sandbox boots from, resolved exactly like the
    /// game runtime descriptor: the R2 public URL when configured (the browser
    /// fetches chunks straight from the CDN), otherwise a relative path the
    /// frontend proxies to `get_system_chunk`.
    pub base_url: String,
    pub size_bytes: i64,
    pub chunk_size_bytes: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChunkProgress {
    pub upload_id: String,
    pub kind: String,
    pub total_chunks: u64,
    pub completed_chunks: u64,
    pub message: String,
}

#[derive(Serialize)]
pub struct UploadStatusResponse {
    pub status: String,
    pub error_message: Option<String>,
    pub chunk_progress: Option<ChunkProgress>,
    pub active_uploads: Vec<ChunkProgress>,
}

#[derive(Serialize)]
pub struct ServerStatusResponse {
    pub ok: bool,
    pub active_uploads: Vec<ChunkProgress>,
}
