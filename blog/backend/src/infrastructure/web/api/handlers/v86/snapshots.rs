// Snapshot status and deletion for the admin snapshot studio.
use std::{fs, path::Path, sync::Arc};

use axum::{
    Extension, Json,
    extract::{Path as AxumPath, State},
    http::StatusCode,
};
use sqlx::Row;

use crate::domain::{entities::secret::Claims, errors::project::ProjectError};
use crate::infrastructure::web::{api::support::ownership::require_owner, server::AppState};

use super::constants::{V86_STATE_VERSION, V86_TOPOLOGY_VERSION};
use super::dto::SnapshotStatusResponse;
use super::manifest::{parse_system_specs, resolve_system_machine};
use super::shared::user_id;

#[allow(dead_code)]
fn dir_size(root: &Path) -> u64 {
    let mut total = 0_u64;
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_dir() {
                    total = total.saturating_add(dir_size(&entry.path()));
                } else {
                    total = total.saturating_add(meta.len());
                }
            }
        }
    }
    total
}

pub async fn get_game_snapshot(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(game_id): AxumPath<i64>,
) -> Result<Json<Vec<SnapshotStatusResponse>>, ProjectError> {
    require_owner(
        &state.game_service.pool,
        "games",
        game_id,
        user_id(&claims)?,
    )
    .await?;
    // One row per snapshot; freshness against the system's resolved machine
    // shape is computed in Rust (specs JSON can't be compared in plain SQL).
    let rows = sqlx::query(
        r#"SELECT s.variant_index, s.size_bytes, s.raw_size_bytes, s.created_at,
                  s.state_version, s.topology_version, s.memory_size, s.vga_memory_size,
                  (s.system_version_id = g.system_version_id
                   AND s.game_disk_sha256 = g.disk_sha256
                   AND (s.variant_index = 0 OR s.iso_sha256 = v.iso_sha256)) AS disks_fresh,
                  sys.platform_key, sys.memory_size_mb, sys.specs
           FROM game_v86_snapshots s
           JOIN game_v86_games g ON g.game_id = s.game_id
           JOIN v86_system_versions gv ON gv.id = g.system_version_id
           JOIN v86_systems sys ON sys.id = gv.system_id
           LEFT JOIN game_v86_variants v
             ON v.game_id = s.game_id AND v.variant_index = s.variant_index
           WHERE s.game_id = ?
           ORDER BY s.variant_index"#,
    )
    .bind(game_id)
    .fetch_all(&state.project_service.pool)
    .await?;
    Ok(Json(
        rows.iter()
            .map(|row| {
                let specs = parse_system_specs(
                    row.try_get::<Option<String>, _>("specs")
                        .ok()
                        .flatten()
                        .as_deref(),
                );
                let (expected_vga, _) =
                    resolve_system_machine(&row.get::<String, _>("platform_key"), &specs);
                let fresh = row.get::<i64, _>("disks_fresh") != 0
                    && row.get::<i64, _>("state_version") == V86_STATE_VERSION
                    && row.get::<i64, _>("topology_version") == V86_TOPOLOGY_VERSION
                    && row.get::<i64, _>("memory_size")
                        == row.get::<i64, _>("memory_size_mb").max(1) * 1048576
                    && row.get::<i64, _>("vga_memory_size") == expected_vga as i64;
                SnapshotStatusResponse {
                    variant_index: row.get("variant_index"),
                    exists: true,
                    stale: !fresh,
                    size_bytes: Some(row.get::<i64, _>("size_bytes") as u64),
                    raw_size_bytes: Some(row.get::<i64, _>("raw_size_bytes") as u64),
                    created_at: Some(row.get("created_at")),
                }
            })
            .collect(),
    ))
}

pub async fn delete_game_snapshot(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath((game_id, variant_index)): AxumPath<(i64, i32)>,
) -> Result<StatusCode, ProjectError> {
    require_owner(
        &state.game_service.pool,
        "games",
        game_id,
        user_id(&claims)?,
    )
    .await?;
    let storage_key: Option<String> = sqlx::query_scalar(
        "SELECT storage_key FROM game_v86_snapshots WHERE game_id = ? AND variant_index = ?",
    )
    .bind(game_id)
    .bind(variant_index)
    .fetch_optional(&state.project_service.pool)
    .await?
    .flatten();
    sqlx::query("DELETE FROM game_v86_snapshots WHERE game_id = ? AND variant_index = ?")
        .bind(game_id)
        .bind(variant_index)
        .execute(&state.project_service.pool)
        .await?;
    // Blobs are content-addressed, so another variant may share this object.
    if let Some(key) = storage_key {
        let still_referenced: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM game_v86_snapshots WHERE storage_key = ?")
                .bind(&key)
                .fetch_one(&state.project_service.pool)
                .await
                .unwrap_or(1);
        if still_referenced == 0 {
            let _ = state.storage.delete_object(&key).await;
        }
    }
    Ok(StatusCode::NO_CONTENT)
}
