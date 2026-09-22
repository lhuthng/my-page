// Completing a snapshot upload: verify the bytes (zstd magic + checksum),
// re-check disk pinning, promote to the content-addressed key, and record.
use std::sync::Arc;

use axum::{
    Extension,
    extract::{Path as AxumPath, State},
    http::StatusCode,
};
use sha2::{Digest, Sha256};
use sqlx::Row;

use crate::domain::{entities::secret::Claims, errors::project::ProjectError};
use crate::infrastructure::web::{api::support::ownership::require_owner, server::AppState};

use super::constants::{V86_TOPOLOGY_VERSION, ZSTD_MAGIC};
use super::shared::{ensure_upload_not_expired, storage_error, user_id};
use super::snapshot_uploads::{fail_snapshot_session, snapshot_storage_key};
use super::upload_session::parse_part_etags;

/// Finalises a snapshot upload. Unlike the disk pipeline there is no chunking,
/// no zstd pass and no background build: v86 forces `initial_state` to load
/// synchronously as a single blob, and `restore_state` unpacks the zstd frame
/// itself, so the bytes are promoted to their content-addressed key verbatim.
pub async fn complete_snapshot_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(upload_id): AxumPath<String>,
) -> Result<StatusCode, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let row = sqlx::query(
        r#"SELECT game_id, variant_index, iso_sha256, system_version_id,
                  game_disk_sha256, raw_size_bytes, sha256,
                  state_version, memory_size, vga_memory_size, expected_size_bytes,
                  received_size_bytes, temp_storage_key, r2_upload_id, r2_part_etags,
                  status, expires_at
           FROM game_v86_snapshot_upload_sessions WHERE id = ? AND uploader_id = ?"#,
    )
    .bind(&upload_id)
    .bind(uploader_id)
    .fetch_optional(&state.project_service.pool)
    .await?
    .ok_or(ProjectError::ProjectNotFound)?;
    ensure_upload_not_expired(row.get::<String, _>("expires_at").as_str())?;
    if row.get::<String, _>("status") != "active"
        || row.get::<i64, _>("expected_size_bytes") != row.get::<i64, _>("received_size_bytes")
    {
        return Err(ProjectError::InvalidDemo(
            "The snapshot upload is incomplete.".to_string(),
        ));
    }
    let game_id: i64 = row.get("game_id");
    require_owner(&state.game_service.pool, "games", game_id, uploader_id).await?;

    let storage = &state.storage;
    let temp_key: String = row.get("temp_storage_key");
    let multipart_id = row
        .get::<Option<String>, _>("r2_upload_id")
        .ok_or_else(|| {
            ProjectError::InternalError("Upload session is missing its multipart id.".to_string())
        })?;
    let etags = parse_part_etags(row.get::<Option<String>, _>("r2_part_etags").as_deref());
    storage
        .complete_multipart(&temp_key, &multipart_id, etags)
        .await
        .map_err(storage_error)?;

    // Verify what actually landed rather than trusting the client: the blob
    // must be a zstd frame (v86 sniffs this magic to decide whether to
    // decompress) and must hash to the digest the key is derived from.
    let bytes = storage.get_object(&temp_key).await.map_err(storage_error)?;
    let declared_sha: String = row.get("sha256");
    let actual_sha = hex::encode(Sha256::digest(&bytes));
    let invalid = if bytes.len() < 4 || bytes[..4] != ZSTD_MAGIC {
        Some("The snapshot is not a zstd-compressed v86 state.")
    } else if actual_sha != declared_sha {
        Some("The uploaded snapshot does not match its declared checksum.")
    } else {
        None
    };
    if let Some(message) = invalid {
        let _ = storage.delete_object(&temp_key).await;
        fail_snapshot_session(&state, &upload_id, message).await;
        return Err(ProjectError::InvalidDemo(message.to_string()));
    }

    // Re-check the disk pinning: the game disk may have been replaced while
    // the (slow) compress + upload was in flight.
    let game =
        sqlx::query("SELECT system_version_id, disk_sha256 FROM game_v86_games WHERE game_id = ?")
            .bind(game_id)
            .fetch_optional(&state.project_service.pool)
            .await?
            .ok_or(ProjectError::ProjectNotFound)?;
    let game_disk_sha: String = row.get("game_disk_sha256");
    let current_disk_sha: Option<String> = game.get("disk_sha256");
    if game.get::<i64, _>("system_version_id") != row.get::<i64, _>("system_version_id")
        || current_disk_sha.as_deref() != Some(game_disk_sha.as_str())
    {
        let _ = storage.delete_object(&temp_key).await;
        let message = "The game's disks changed while this snapshot was uploading.";
        fail_snapshot_session(&state, &upload_id, message).await;
        return Err(ProjectError::Conflict(message.to_string()));
    }

    // Same for the variant's disc, which a manifest edit can rebuild.
    let variant_index: i32 = row.get("variant_index");
    let iso_sha: Option<String> = row.get("iso_sha256");
    if variant_index > 0 {
        let current_iso_sha: Option<String> = sqlx::query_scalar(
            "SELECT iso_sha256 FROM game_v86_variants WHERE game_id = ? AND variant_index = ?",
        )
        .bind(game_id)
        .bind(variant_index)
        .fetch_optional(&state.project_service.pool)
        .await?;
        if current_iso_sha.is_none() || current_iso_sha != iso_sha {
            let _ = storage.delete_object(&temp_key).await;
            let message = "That variant's disc changed while this snapshot was uploading.";
            fail_snapshot_session(&state, &upload_id, message).await;
            return Err(ProjectError::Conflict(message.to_string()));
        }
    }

    let storage_key = snapshot_storage_key(&declared_sha);
    storage
        .put_object_bytes(&storage_key, bytes)
        .await
        .map_err(storage_error)?;
    let _ = storage.delete_object(&temp_key).await;

    let previous_key: Option<String> = sqlx::query_scalar(
        "SELECT storage_key FROM game_v86_snapshots WHERE game_id = ? AND variant_index = ?",
    )
    .bind(game_id)
    .bind(variant_index)
    .fetch_optional(&state.project_service.pool)
    .await?
    .flatten();

    sqlx::query(
        r#"INSERT INTO game_v86_snapshots
           (game_id, variant_index, iso_sha256, system_version_id, game_disk_sha256,
            storage_key, size_bytes,
            raw_size_bytes, sha256, state_version, topology_version,
            memory_size, vga_memory_size, created_by)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
           ON CONFLICT(game_id, variant_index) DO UPDATE SET
             iso_sha256 = excluded.iso_sha256,
             topology_version = excluded.topology_version,
             system_version_id = excluded.system_version_id,
             game_disk_sha256 = excluded.game_disk_sha256,
             storage_key = excluded.storage_key,
             size_bytes = excluded.size_bytes,
             raw_size_bytes = excluded.raw_size_bytes,
             sha256 = excluded.sha256,
             state_version = excluded.state_version,
             memory_size = excluded.memory_size,
             vga_memory_size = excluded.vga_memory_size,
             created_by = excluded.created_by,
             updated_at = CURRENT_TIMESTAMP"#,
    )
    .bind(game_id)
    .bind(variant_index)
    .bind(iso_sha.clone().unwrap_or_default())
    .bind(row.get::<i64, _>("system_version_id"))
    .bind(&game_disk_sha)
    .bind(&storage_key)
    .bind(row.get::<i64, _>("expected_size_bytes"))
    .bind(row.get::<i64, _>("raw_size_bytes"))
    .bind(&declared_sha)
    .bind(row.get::<i64, _>("state_version"))
    .bind(V86_TOPOLOGY_VERSION)
    .bind(row.get::<i64, _>("memory_size"))
    .bind(row.get::<i64, _>("vga_memory_size"))
    .bind(uploader_id)
    .execute(&state.project_service.pool)
    .await?;

    sqlx::query(
        "UPDATE game_v86_snapshot_upload_sessions SET status = 'consumed', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(&upload_id)
    .execute(&state.project_service.pool)
    .await?;

    // Content-addressed keys mean a recapture that produced identical bytes
    // reuses the same object, and two variants could in principle land on the
    // same one. Only drop the old object when it changed and nothing else
    // still points at it.
    if let Some(previous) = previous_key
        && previous != storage_key
    {
        let still_referenced: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM game_v86_snapshots WHERE storage_key = ?")
                .bind(&previous)
                .fetch_one(&state.project_service.pool)
                .await
                .unwrap_or(1);
        if still_referenced == 0 {
            let _ = storage.delete_object(&previous).await;
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn abort_snapshot_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(upload_id): AxumPath<String>,
) -> Result<StatusCode, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let row = sqlx::query(
        "SELECT temp_storage_key, r2_upload_id, status FROM game_v86_snapshot_upload_sessions WHERE id = ? AND uploader_id = ?",
    )
    .bind(&upload_id)
    .bind(uploader_id)
    .fetch_optional(&state.project_service.pool)
    .await?
    .ok_or(ProjectError::ProjectNotFound)?;
    if row.get::<String, _>("status") == "active"
        && let Some(multipart_id) = row.get::<Option<String>, _>("r2_upload_id")
    {
        let _ = state
            .storage
            .abort_multipart(&row.get::<String, _>("temp_storage_key"), &multipart_id)
            .await;
    }
    sqlx::query(
        "UPDATE game_v86_snapshot_upload_sessions SET status = 'aborted', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(&upload_id)
    .execute(&state.project_service.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
