// v86 emulator feature handlers, split by responsibility. This module is the
// public surface: route handlers plus the descriptor API the game/project
// handlers call. Internal helpers stay inside their sub-modules.
mod constants;
mod dto;
mod game_uploads;
mod games;
mod manifest;
mod runtime;
mod saves;
mod serving;
mod shared;
mod snapshot_finalize;
mod snapshot_uploads;
mod snapshots;
mod system_uploads;
mod system_versions;
mod systems;
mod upload_session;

#[cfg(test)]
mod tests;

pub use dto::{
    ActiveSystemsQuery, ChunkProgress, ChunkUploadResponse, DiskUploadSpec, GameBuildPlans,
    GameDiskPlan, GameVariantPlan, PublicSystemVersion, ServerStatusResponse,
    SnapshotStatusResponse, StartGameUploadRequest, StartGameUploadResponse,
    StartSnapshotUploadRequest, StartSystemUploadRequest, StartSystemUploadResponse,
    StartUploadResponse, UpdateSystemRequest, UploadStatusResponse, V86RuntimeDescriptor,
    V86SystemResponse, V86SystemSpecs, V86SystemVersionResponse, VariantDescriptor,
    VariantUploadSpec,
};
pub use game_uploads::{
    abort_game_upload, attach_ready_game_tx, complete_game_upload, get_game_upload_status,
};
pub use games::{start_game_upload, upload_game_disk_part, upload_game_variant_iso};
pub use runtime::{
    RuntimeLookup, get_game_capture_runtime, get_game_launcher, runtime_descriptor,
    runtime_descriptor_for,
};
pub use saves::{delete_game_save, get_game_save, put_game_save};
pub use serving::{
    get_game_chunk, get_game_disk_chunk, get_game_iso, get_snapshot_blob, get_system_chunk,
};
pub use snapshot_finalize::{abort_snapshot_upload, complete_snapshot_upload};
pub use snapshot_uploads::{append_snapshot_chunk, start_snapshot_upload};
pub use snapshots::{delete_game_snapshot, get_game_snapshot};
pub use system_uploads::{
    abort_system_upload, get_server_status, get_system_upload_status, start_system_upload,
    upload_system_part,
};
pub use system_versions::{complete_system_upload, delete_system, delete_system_version};
pub use systems::{
    list_active_systems, list_public_systems, list_systems, update_system,
};
