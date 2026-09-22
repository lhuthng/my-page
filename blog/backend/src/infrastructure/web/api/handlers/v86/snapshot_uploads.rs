// Starting and streaming a snapshot upload: freshness validation against the
// game's current disks and machine shape, then a chunked multipart relay.
use std::sync::Arc;

use axum::{
    Extension, Json,
    body::Bytes,
    extract::{Path as AxumPath, State},
};
use chrono::{Duration, Utc};
use sqlx::Row;
use uuid::Uuid;

use crate::domain::{entities::secret::Claims, errors::project::ProjectError};
use crate::infrastructure::web::{api::support::ownership::require_owner, server::AppState};

use super::constants::{
    V86_MEMORY_SIZE, V86_SNAPSHOT_MAX_BYTES, V86_STATE_VERSION, V86_TOPOLOGY_VERSION,
};
use super::dto::{ChunkUploadResponse, StartSnapshotUploadRequest, StartUploadResponse};
use super::manifest::{parse_system_specs, resolve_system_machine};
use super::shared::{storage_error, user_id};
use super::upload_session::{append_upload_chunk, transient_storage_key};

/// Storage key for a snapshot blob. Content-addressed, so identical states
/// dedupe and the object can be cached immutably forever.
pub(super) fn snapshot_storage_key(sha256: &str) -> String {
    format!("v86/snapshots/{sha256}/state.zst")
}

pub(super) fn validate_sha256_hex(value: &str) -> Result<(), ProjectError> {
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(ProjectError::InvalidDemo(
            "Expected a hex-encoded sha256 digest.".to_string(),
        ));
    }
    Ok(())
}

pub async fn start_snapshot_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<StartSnapshotUploadRequest>,
) -> Result<Json<StartUploadResponse>, ProjectError> {
    let uploader_id = user_id(&claims)?;
    require_owner(
        &state.game_service.pool,
        "games",
        request.game_id,
        uploader_id,
    )
    .await?;
    validate_sha256_hex(&request.sha256)?;
    validate_sha256_hex(&request.game_disk_sha256)?;

    if request.size_bytes == 0 || request.size_bytes > V86_SNAPSHOT_MAX_BYTES {
        return Err(ProjectError::InvalidDemo(
            "The snapshot exceeds the configured limit.".to_string(),
        ));
    }
    // Restoring a state into a machine shaped differently from the one it was
    // captured on corrupts the guest, so reject the mismatch at the door
    // rather than storing something runtime_descriptor would silently drop.
    if request.state_version != V86_STATE_VERSION {
        return Err(ProjectError::InvalidDemo(format!(
            "This snapshot targets v86 state version {}, but the server serves version {V86_STATE_VERSION}.",
            request.state_version
        )));
    }
    // The snapshot embeds dirty blocks from these exact disks; if the game
    // has been re-uploaded since capture started, the state is already void.
    let game = sqlx::query(
        r#"SELECT g.system_version_id, g.disk_sha256, sys.memory_size_mb, sys.platform_key, sys.specs
           FROM game_v86_games g
           JOIN v86_system_versions v ON v.id = g.system_version_id
           JOIN v86_systems sys ON sys.id = v.system_id
           WHERE g.game_id = ?"#,
    )
    .bind(request.game_id)
    .fetch_optional(&state.project_service.pool)
    .await?
    .ok_or(ProjectError::ProjectNotFound)?;
    let current_disk_sha: Option<String> = game.get("disk_sha256");
    if game.get::<i64, _>("system_version_id") != request.system_version_id
        || current_disk_sha.as_deref() != Some(request.game_disk_sha256.as_str())
    {
        return Err(ProjectError::Conflict(
            "The game's disks changed while this snapshot was being captured. Recapture it."
                .to_string(),
        ));
    }
    // Memory is per-system; the capturing player reports what it booted with.
    let system_memory_bytes = game
        .try_get::<i64, _>("memory_size_mb")
        .ok()
        .map(|mb| mb.max(1) as u64 * 1024 * 1024)
        .unwrap_or(V86_MEMORY_SIZE);
    let system_specs = parse_system_specs(
        game.try_get::<Option<String>, _>("specs")
            .ok()
            .flatten()
            .as_deref(),
    );
    let (expected_vga, _) =
        resolve_system_machine(&game.get::<String, _>("platform_key"), &system_specs);
    if request.memory_size != system_memory_bytes || request.vga_memory_size != expected_vga {
        return Err(ProjectError::InvalidDemo(
            "The snapshot was captured with a different memory size than the player uses."
                .to_string(),
        ));
    }
    if request.topology_version != V86_TOPOLOGY_VERSION {
        return Err(ProjectError::InvalidDemo(
            "This snapshot was captured on a different machine layout. Reload the studio and recapture."
                .to_string(),
        ));
    }

    // A variant snapshot holds a machine with that variant's disc mounted, so
    // it is only replayable against the identical disc. Index 0 is the
    // game-wide capture and must have no disc at all.
    if request.variant_index < 0 {
        return Err(ProjectError::InvalidDemo(
            "The variant index cannot be negative.".to_string(),
        ));
    }
    if request.variant_index == 0 {
        if request.iso_sha256.is_some() {
            return Err(ProjectError::InvalidDemo(
                "A game-wide snapshot is captured with no disc, so it cannot record one."
                    .to_string(),
            ));
        }
    } else {
        let iso_sha = request.iso_sha256.as_deref().ok_or_else(|| {
            ProjectError::InvalidDemo(
                "A variant snapshot must record the disc it was captured with.".to_string(),
            )
        })?;
        validate_sha256_hex(iso_sha)?;
        let current_iso_sha: Option<String> = sqlx::query_scalar(
            "SELECT iso_sha256 FROM game_v86_variants WHERE game_id = ? AND variant_index = ?",
        )
        .bind(request.game_id)
        .bind(request.variant_index)
        .fetch_optional(&state.project_service.pool)
        .await?;
        match current_iso_sha {
            None => {
                return Err(ProjectError::InvalidDemo(
                    "That launch variant does not exist for this game.".to_string(),
                ));
            }
            Some(current) if current != iso_sha => {
                return Err(ProjectError::Conflict(
                    "That variant's disc was rebuilt while this snapshot was being captured. Recapture it."
                        .to_string(),
                ));
            }
            Some(_) => {}
        }
    }

    let upload_id = Uuid::new_v4().to_string();
    let transient_key = transient_storage_key("snapshots", &upload_id, "zst");
    let multipart = state
        .storage
        .create_multipart(&transient_key)
        .await
        .map_err(storage_error)?;
    let multipart_id = multipart.upload_id;
    let expires_at =
        Utc::now() + Duration::hours(state.project_demo_config.upload_session_ttl_hours as i64);
    sqlx::query(
        r#"INSERT INTO game_v86_snapshot_upload_sessions
           (id, uploader_id, game_id, variant_index, iso_sha256,
            system_version_id, game_disk_sha256,
            raw_size_bytes, sha256, state_version, memory_size, vga_memory_size,
            expected_size_bytes, upload_chunk_size_bytes, temp_storage_key,
            r2_upload_id, expires_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&upload_id)
    .bind(uploader_id)
    .bind(request.game_id)
    .bind(request.variant_index)
    .bind(request.iso_sha256.clone().unwrap_or_default())
    .bind(request.system_version_id)
    .bind(&request.game_disk_sha256)
    .bind(request.raw_size_bytes as i64)
    .bind(&request.sha256)
    .bind(request.state_version)
    .bind(request.memory_size as i64)
    .bind(request.vga_memory_size as i64)
    .bind(request.size_bytes as i64)
    .bind(state.project_demo_config.v86_upload_chunk_size as i64)
    .bind(&transient_key)
    .bind(&multipart_id)
    .bind(expires_at.to_rfc3339())
    .execute(&state.project_service.pool)
    .await
    .inspect_err(|_error| {
        let storage = state.storage.clone();
        let multipart_id = multipart_id.clone();
        tokio::spawn(async move {
            let _ = storage.abort_multipart(&transient_key, &multipart_id).await;
        });
    })?;
    Ok(Json(StartUploadResponse {
        upload_id,
        chunk_size_bytes: state.project_demo_config.v86_upload_chunk_size,
        next_chunk_index: 0,
        expected_size_bytes: request.size_bytes,
        upload_required: true,
    }))
}

pub async fn append_snapshot_chunk(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath((upload_id, chunk_index)): AxumPath<(String, u64)>,
    bytes: Bytes,
) -> Result<Json<ChunkUploadResponse>, ProjectError> {
    Ok(Json(
        append_upload_chunk(
            &state,
            "game_v86_snapshot_upload_sessions",
            &upload_id,
            user_id(&claims)?,
            chunk_index,
            bytes,
        )
        .await?,
    ))
}

pub(super) async fn fail_snapshot_session(state: &AppState, upload_id: &str, message: &str) {
    sqlx::query(
        "UPDATE game_v86_snapshot_upload_sessions SET status = 'failed', error_message = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(message)
    .bind(upload_id)
    .execute(&state.project_service.pool)
    .await
    .ok();
}
