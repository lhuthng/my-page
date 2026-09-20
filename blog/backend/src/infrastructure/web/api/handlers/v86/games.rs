// Game build upload intake: validating the client's build plan against the
// manifest and storing disk parts and launcher CDs content-addressed.
use std::sync::Arc;

use axum::{
    Extension, Json,
    body::Bytes,
    extract::{Path as AxumPath, State},
    http::StatusCode,
};
use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

use crate::domain::{entities::secret::Claims, errors::project::ProjectError};
use crate::infrastructure::web::{
    api::support::ownership::require_owner,
    server::AppState,
};

use super::dto::{DiskUploadSpec, StartGameUploadRequest, StartGameUploadResponse, VariantUploadSpec};
use super::game_uploads::fetch_stored_game_artifact;
use super::manifest::{parse_mouse_config, parse_variants, validate_manifest};
use super::shared::{ensure_upload_not_expired, storage_error, user_id};

/// The content-addressed object key of one disk part: the browser requests
/// parts by `{offset}-{offset+chunk_size}.img.zst`, and every part is zero-
/// padded to the full chunk size (including the last one), matching `split_asset`.
fn disk_part_name(storage_key: &str, part_index: u64, chunk_size: u64) -> String {
    let offset = part_index * chunk_size;
    format!("{storage_key}/{offset}-{}.img.zst", offset + chunk_size)
}

pub async fn start_game_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<StartGameUploadRequest>,
) -> Result<Json<StartGameUploadResponse>, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let manifest_sha = validate_manifest(&request.manifest)?;
    let active: Option<i64> = sqlx::query_scalar(
        "SELECT v.id FROM v86_system_versions v JOIN v86_systems s ON s.id = v.system_id WHERE v.id = ? AND (s.is_active = 1 OR EXISTS (SELECT 1 FROM game_v86_games g WHERE g.system_version_id = v.id AND g.game_id = ?))",
    )
    .bind(request.system_version_id)
    .bind(request.source_project_id)
    .fetch_optional(&state.project_service.pool)
    .await?;
    if active.is_none() {
        return Err(ProjectError::InvalidDemo(
            "The selected v86 system version is unavailable.".to_string(),
        ));
    }
    let variants = parse_variants(&request.manifest)?;
    if variants.is_empty() || variants.len() != request.plans.variants.len() {
        return Err(ProjectError::InvalidDemo(
            "The build plan does not match the manifest variants.".to_string(),
        ));
    }
    // Reject malformed mouse settings so a typo never reaches the descriptor.
    parse_mouse_config(&request.manifest)?;
    let upload_id = Uuid::new_v4().to_string();
    let chunk_size = state.project_demo_config.v86_download_chunk_size;
    let max_disk = state.project_demo_config.max_v86_game_extracted_size.saturating_mul(2);

    // When editing an existing game, the stored artifact resolves the
    // manifest-only fast path (no new ZIP) and validates the revision.
    let stored = match request.source_project_id {
        Some(game_id) => {
            require_owner(&state.game_service.pool, "games", game_id, uploader_id).await?;
            let artifact = fetch_stored_game_artifact(&state.project_service.pool, game_id)
                .await
                .map_err(ProjectError::InternalError)?
                .ok_or(ProjectError::ProjectNotFound)?;
            if artifact.artifact_revision != request.expected_artifact_revision {
                return Err(ProjectError::Conflict(
                    "The v86 artifact changed in another editor.".to_string(),
                ));
            }
            Some(artifact)
        }
        None => None,
    };

    // Disk plan. A new ZIP (disk plan present) is deduplicated against any
    // project that already built the same disk; a manifest-only edit (no plan)
    // reuses the source project's stored disk wholesale.
    let disk = match &request.plans.disk {
        Some(plan) => {
            if plan.size_bytes == 0 || plan.size_bytes > max_disk {
                return Err(ProjectError::InvalidDemo(
                    "The game disk exceeds the configured limit.".to_string(),
                ));
            }
            let existing: Option<i64> = sqlx::query_scalar(
                "SELECT chunk_count FROM game_v86_games
                 WHERE disk_sha256 = ? AND disk_storage_key IS NOT NULL LIMIT 1",
            )
            .bind(&plan.sha256)
            .fetch_optional(&state.project_service.pool)
            .await?;
            Some(match existing {
                Some(chunk_count) => DiskUploadSpec {
                    sha256: plan.sha256.clone(),
                    size_bytes: plan.size_bytes,
                    chunk_size_bytes: chunk_size,
                    chunk_count: chunk_count as u64,
                    reuse: true,
                },
                None => DiskUploadSpec {
                    sha256: plan.sha256.clone(),
                    size_bytes: plan.size_bytes,
                    chunk_size_bytes: chunk_size,
                    chunk_count: plan.size_bytes.div_ceil(chunk_size),
                    reuse: false,
                },
            })
        }
        None => {
            let artifact = stored.as_ref().ok_or_else(|| {
                ProjectError::InvalidDemo(
                    "A game disk is required for new projects.".to_string(),
                )
            })?;
            let disk_sha = artifact.disk_sha256.clone().ok_or_else(|| {
                ProjectError::InvalidDemo("The source project has no game disk.".to_string())
            })?;
            let disk_size = artifact.disk_size_bytes.ok_or_else(|| {
                ProjectError::InvalidDemo("The source project has no game disk.".to_string())
            })?;
            Some(DiskUploadSpec {
                sha256: disk_sha,
                size_bytes: disk_size as u64,
                chunk_size_bytes: chunk_size,
                chunk_count: artifact.chunk_count as u64,
                reuse: true,
            })
        }
    };

    // Per-variant launcher CDs: deduplicate by the ISO content hash so
    // manifest-only edits that produce identical CDs skip re-uploading.
    let mut variants_out = Vec::with_capacity(request.plans.variants.len());
    for plan in &request.plans.variants {
        if plan.size_bytes == 0 {
            return Err(ProjectError::InvalidDemo(
                "A launcher CD plan has a zero size.".to_string(),
            ));
        }
        let existing: Option<String> = sqlx::query_scalar(
            r#"SELECT iso_storage_key FROM (
                 SELECT g.iso_storage_key FROM game_v86_games g WHERE g.iso_sha256 = ?
                 UNION
                 SELECT v.iso_storage_key FROM game_v86_variants v WHERE v.iso_sha256 = ?
               ) LIMIT 1"#,
        )
        .bind(&plan.sha256)
        .bind(&plan.sha256)
        .fetch_optional(&state.project_service.pool)
        .await?;
        variants_out.push(VariantUploadSpec {
            index: plan.index,
            sha256: plan.sha256.clone(),
            size_bytes: plan.size_bytes,
            reuse: existing.is_some(),
        });
    }

    let expires_at =
        Utc::now() + Duration::hours(state.project_demo_config.upload_session_ttl_hours as i64);
    // Finished sessions of the same project cannot pile up.
    if let Some(project_id) = request.source_project_id {
        sqlx::query(
            "DELETE FROM project_v86_upload_sessions WHERE source_project_id = ? AND status != 'active'",
        )
        .bind(project_id)
        .execute(&state.project_service.pool)
        .await?;
    }
    let first = variants_out.first().ok_or_else(|| {
        ProjectError::InvalidDemo("The build plan has no launcher CDs.".to_string())
    })?;
    let disk_key = disk.as_ref().map(|d| format!("v86/games/{}", d.sha256));
    let disk_reuse = disk.as_ref().map_or(false, |d| d.reuse);
    sqlx::query(
        r#"INSERT INTO project_v86_upload_sessions
           (id, uploader_id, source_project_id, system_version_id,
            expected_artifact_revision, manifest_text, manifest_sha256,
            staged_disk_storage_key, staged_disk_sha256, staged_disk_size_bytes,
            staged_disk_chunk_count, disk_reuse, received_disk_parts,
            staged_iso_storage_key, staged_iso_sha256, staged_iso_size_bytes,
            expires_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&upload_id)
    .bind(uploader_id)
    .bind(request.source_project_id)
    .bind(request.system_version_id)
    .bind(request.expected_artifact_revision)
    .bind(&request.manifest)
    .bind(manifest_sha)
    .bind(&disk_key)
    .bind(disk.as_ref().map(|d| &d.sha256))
    .bind(disk.as_ref().map(|d| d.size_bytes as i64))
    .bind(disk.as_ref().map(|d| d.chunk_count as i64))
    .bind(disk_reuse)
    .bind(Option::<String>::None)
    .bind(&format!("v86/games/{}", first.sha256))
    .bind(&first.sha256)
    .bind(first.size_bytes as i64)
    .bind(expires_at.to_rfc3339())
    .execute(&state.project_service.pool)
    .await?;
    for (variant, plan) in variants.iter().zip(&request.plans.variants) {
        let spec = variants_out.iter().find(|s| s.index == plan.index).unwrap();
        sqlx::query(
            r#"INSERT INTO project_v86_staged_variants
               (upload_id, variant_index, name, exe, args, iso_storage_key,
                iso_size_bytes, iso_sha256, reuse)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&upload_id)
        .bind(variant.index)
        .bind(&variant.name)
        .bind(&variant.exe)
        .bind(&variant.args)
        .bind(&format!("v86/games/{}", plan.sha256))
        .bind(plan.size_bytes as i64)
        .bind(&plan.sha256)
        .bind(spec.reuse)
        .execute(&state.project_service.pool)
        .await?;
    }
    Ok(Json(StartGameUploadResponse {
        upload_id,
        disk,
        variants: variants_out,
    }))
}

/// Stores one zstd-compressed disk part at its content-addressed key. The part
/// name is the byte range `{offset}-{offset+chunk_size}.img.zst`, matching the
/// layout the browser streams with `use_parts`.
pub async fn upload_game_disk_part(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath((upload_id, part_index)): AxumPath<(String, u64)>,
    bytes: Bytes,
) -> Result<StatusCode, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let row = sqlx::query(
        "SELECT staged_disk_storage_key, staged_disk_chunk_count, disk_reuse, status, expires_at FROM project_v86_upload_sessions WHERE id = ? AND uploader_id = ?",
    )
    .bind(&upload_id)
    .bind(uploader_id)
    .fetch_optional(&state.project_service.pool)
    .await?
    .ok_or(ProjectError::ProjectNotFound)?;
    ensure_upload_not_expired(row.get::<String, _>("expires_at").as_str())?;
    if row.get::<String, _>("status") != "active" {
        return Err(ProjectError::InvalidDemo(
            "The v86 game upload is no longer active.".to_string(),
        ));
    }
    let chunk_count: i64 = row.get("staged_disk_chunk_count");
    if row.get::<i64, _>("disk_reuse") != 0 {
        return Err(ProjectError::Conflict(
            "The game disk already exists; no parts are expected.".to_string(),
        ));
    }
    let key: String = row.get("staged_disk_storage_key");
    if part_index >= chunk_count as u64 {
        return Err(ProjectError::InvalidDemo(
            "Disk part index is out of range.".to_string(),
        ));
    }
    let part = disk_part_name(&key, part_index, state.project_demo_config.v86_download_chunk_size);
    state
        .storage
        .put_object_bytes(&part, bytes.to_vec())
        .await
        .map_err(storage_error)?;
    // Record the part atomically so parallel PUTs cannot drop indices (a plain
    // INSERT, unlike the old read-modify-write of the received_disk_parts JSON
    // column which raced under parallel uploads). Re-uploading a part is a no-op.
    sqlx::query(
        "INSERT OR IGNORE INTO project_v86_received_disk_parts (upload_id, part_index) VALUES (?, ?)",
    )
    .bind(&upload_id)
    .bind(part_index as i64)
    .execute(&state.project_service.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Stores one variant's launcher CD. The SHA-256 of the received bytes must
/// match the client's plan, so content-addressed keys stay truthful even though
/// the server no longer builds the CD itself.
pub async fn upload_game_variant_iso(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath((upload_id, variant_index)): AxumPath<(String, i32)>,
    bytes: Bytes,
) -> Result<StatusCode, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let row = sqlx::query(
        r#"SELECT s.expires_at, s.status, v.iso_storage_key, v.iso_sha256, v.reuse
           FROM project_v86_upload_sessions s
           JOIN project_v86_staged_variants v ON v.upload_id = s.id
           WHERE s.id = ? AND s.uploader_id = ? AND v.variant_index = ?"#,
    )
    .bind(&upload_id)
    .bind(uploader_id)
    .bind(variant_index)
    .fetch_optional(&state.project_service.pool)
    .await?
    .ok_or(ProjectError::ProjectNotFound)?;
    ensure_upload_not_expired(row.get::<String, _>("expires_at").as_str())?;
    if row.get::<String, _>("status") != "active" {
        return Err(ProjectError::InvalidDemo(
            "The v86 game upload is no longer active.".to_string(),
        ));
    }
    if row.get::<i64, _>("reuse") != 0 {
        return Err(ProjectError::Conflict(
            "The launcher CD already exists; no upload is expected.".to_string(),
        ));
    }
    let expected: String = row.get("iso_sha256");
    let actual = hex::encode(Sha256::digest(&bytes));
    if actual != expected {
        return Err(ProjectError::InvalidDemo(
            "The launcher CD failed its checksum check.".to_string(),
        ));
    }
    let key: String = row.get("iso_storage_key");
    state
        .storage
        .put_object_bytes(&format!("{key}/full.iso"), bytes.to_vec())
        .await
        .map_err(storage_error)?;
    sqlx::query(
        "UPDATE project_v86_staged_variants SET received = 1 WHERE upload_id = ? AND variant_index = ?",
    )
    .bind(&upload_id)
    .bind(variant_index)
    .execute(&state.project_service.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

