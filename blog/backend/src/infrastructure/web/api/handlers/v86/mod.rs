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
pub use systems::{list_active_systems, list_public_systems, list_systems, update_system};

// ---------------------------------------------------------------------------
// Route table
use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware,
    routing::{delete, get, patch, post, put},
};

use crate::infrastructure::web::{api::middlewares, server::AppState};

pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // moderator-protected: game package and snapshot pipelines
    Router::new()
        .route("/systems/active", get(list_active_systems))
        .route("/launcher", get(get_game_launcher))
        .route("/games/upload", post(start_game_upload))
        .route(
            "/games/upload/{upload_id}/disk/{part_index}",
            put(upload_game_disk_part),
        )
        .route(
            "/games/upload/{upload_id}/iso/{variant_index}",
            put(upload_game_variant_iso),
        )
        .route(
            "/games/upload/{upload_id}/complete",
            post(complete_game_upload),
        )
        .route("/games/upload/{upload_id}", get(get_game_upload_status))
        .route("/games/upload/{upload_id}", delete(abort_game_upload))
        .route("/snapshots/upload", post(start_snapshot_upload))
        .route(
            "/snapshots/upload/{upload_id}/chunk/{chunk_index}",
            put(append_snapshot_chunk),
        )
        .route(
            "/snapshots/upload/{upload_id}/complete",
            post(complete_snapshot_upload),
        )
        .route(
            "/snapshots/upload/{upload_id}",
            delete(abort_snapshot_upload),
        )
        .route("/games/id/{game_id}/snapshot", get(get_game_snapshot))
        .route(
            "/games/id/{game_id}/snapshot/{variant_index}",
            delete(delete_game_snapshot),
        )
        .route(
            "/games/id/{game_id}/capture-runtime",
            get(get_game_capture_runtime),
        )
        .layer(middleware::from_fn(middlewares::auth::mod_check))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middlewares::auth::user_guard,
        ))
        .layer(DefaultBodyLimit::max(9 * 1024 * 1024))
        // admin-protected: system image management
        .merge(
            Router::new()
                .route("/systems", get(list_systems))
                .route("/systems/status", get(get_server_status))
                .route("/systems/upload", post(start_system_upload))
                .route(
                    "/systems/upload/{upload_id}/part/{part_index}",
                    put(upload_system_part),
                )
                .route(
                    "/systems/upload/{upload_id}/complete",
                    post(complete_system_upload),
                )
                .route("/systems/upload/{upload_id}", get(get_system_upload_status))
                .route("/systems/upload/{upload_id}", delete(abort_system_upload))
                .route("/systems/{system_id}", patch(update_system))
                .route("/systems/{system_id}", delete(delete_system))
                .route(
                    "/systems/{system_id}/versions/{version_id}",
                    delete(delete_system_version),
                )
                .layer(middleware::from_fn(middlewares::auth::admin_check))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                ))
                .layer(DefaultBodyLimit::max(9 * 1024 * 1024)),
        )
        // public: published artifacts
        .route("/systems/public", get(list_public_systems))
        .route("/assets/systems/{sha256}/{part}", get(get_system_chunk))
        .route("/snapshots/{sha256}/{part}", get(get_snapshot_blob))
}
