// v86 system registry: listing (admin + public) and system settings.
use std::{collections::HashSet, sync::Arc};

use axum::{
    Json,
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
};
use sqlx::Row;

use crate::domain::errors::project::ProjectError;
use crate::infrastructure::web::server::AppState;

use super::dto::{
    ActiveSystemsQuery, PublicSystemVersion, UpdateSystemRequest, V86SystemResponse,
    V86SystemVersionResponse,
};
use super::manifest::{parse_system_specs, resolve_system_machine};

/// Guest RAM bounds. Windows 9x is fine at 64; XP wants 256-512. Anything
/// outside this range is a typo or an attempt to starve/oom the emulator.
pub(super) fn validate_memory_size_mb(memory_size_mb: i64) -> Result<(), ProjectError> {
    if !(32..=1024).contains(&memory_size_mb) {
        return Err(ProjectError::InvalidDemo(
            "Memory size must be between 32 and 1024 MB.".to_string(),
        ));
    }
    Ok(())
}

pub async fn list_systems(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<V86SystemResponse>>, ProjectError> {
    let rows = sqlx::query(
        r#"SELECT s.id, s.name, s.platform_key, s.memory_size_mb, s.specs, s.is_active, s.is_default,
                  s.current_version,
                  COUNT(g.game_id) AS project_count,
                  SUM(CASE WHEN posts.status = 'published' THEN 1 ELSE 0 END) AS published_count
           FROM v86_systems s
           LEFT JOIN v86_system_versions v ON v.system_id = s.id
           LEFT JOIN game_v86_games g ON g.system_version_id = v.id
           LEFT JOIN games gm ON gm.id = g.game_id
           LEFT JOIN posts ON posts.id = gm.post_id
           GROUP BY s.id ORDER BY s.name"#,
    )
    .fetch_all(&state.project_service.pool)
    .await?;
    let building: HashSet<i64> = sqlx::query_scalar(
        "SELECT DISTINCT system_id FROM v86_system_upload_sessions WHERE status = 'building' AND system_id IS NOT NULL",
    )
    .fetch_all(&state.project_service.pool)
    .await?
    .into_iter()
    .collect();

    let mut systems = Vec::with_capacity(rows.len());
    for row in rows {
        let system_id: i64 = row.get("id");
        let versions = sqlx::query(
            "SELECT id, version_number, original_file_name, size_bytes, sha256, chunk_size_bytes, chunk_count FROM v86_system_versions WHERE system_id = ? ORDER BY version_number DESC",
        )
        .bind(system_id)
        .fetch_all(&state.project_service.pool)
        .await?
        .into_iter()
        .map(|version| V86SystemVersionResponse {
            id: version.get("id"),
            version_number: version.get("version_number"),
            original_file_name: version.get("original_file_name"),
            size_bytes: version.get("size_bytes"),
            sha256: version.get("sha256"),
            chunk_size_bytes: version.get("chunk_size_bytes"),
            chunk_count: version.get("chunk_count"),
        })
        .collect();
        systems.push(V86SystemResponse {
            id: system_id,
            name: row.get("name"),
            platform_key: row.get("platform_key"),
            memory_size_mb: row.try_get("memory_size_mb").unwrap_or(64),
            specs: parse_system_specs(row.try_get::<Option<String>, _>("specs").ok().flatten().as_deref()),
            is_active: row.get::<i64, _>("is_active") != 0,
            is_default: row.get::<i64, _>("is_default") != 0,
            current_version: row.get("current_version"),
            pending_build: building.contains(&system_id),
            project_count: row.get("project_count"),
            published_project_count: row.get::<Option<i64>, _>("published_count").unwrap_or(0),
            versions,
        });
    }
    Ok(Json(systems))
}
/// Unguarded, and deliberately narrow: only the *current* version of each
/// active system. `get_system_chunk` serves these to anyone, so it exposes no
/// image that was not already publicly fetchable.
pub async fn list_public_systems(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<PublicSystemVersion>>, ProjectError> {
    let rows = sqlx::query(
        r#"SELECT v.id, v.version_number, v.sha256, v.storage_key, v.size_bytes, v.chunk_size_bytes,
                  s.name AS system_name, s.platform_key, s.memory_size_mb, s.specs
           FROM v86_system_versions v
           JOIN v86_systems s ON s.id = v.system_id
           WHERE s.is_active = 1
             AND v.version_number = s.current_version
             AND v.chunk_count > 0
           ORDER BY s.name, v.version_number DESC"#,
    )
    .fetch_all(&state.project_service.pool)
    .await?;
    let public_base_url = state.artifact_base_url();
    Ok(Json(
        rows.into_iter()
            .map(|row| {
                let storage_key: String = row.get("storage_key");
                let base_url = match public_base_url {
                    Some(base) => format!("{}/{storage_key}/.img.zst", base.trim_end_matches('/')),
                    None => format!("{storage_key}/.img.zst"),
                };
                let mut vga_memory_size_mb = 8;
                PublicSystemVersion {
                    id: row.get("id"),
                    version_number: row.get("version_number"),
                    system_name: row.get("system_name"),
                    platform_key: row.get("platform_key"),
                    memory_size_mb: row.try_get("memory_size_mb").unwrap_or(64),
                    specs: {
                        let specs = parse_system_specs(
                            row.try_get::<Option<String>, _>("specs").ok().flatten().as_deref(),
                        );
                        vga_memory_size_mb = (resolve_system_machine(&row.get::<String, _>("platform_key"), &specs).0 / 1048576) as i64;
                        specs
                    },
                    vga_memory_size_mb,
                    sha256: row.get("sha256"),
                    storage_key,
                    base_url,
                    size_bytes: row.get("size_bytes"),
                    chunk_size_bytes: row.get("chunk_size_bytes"),
                }
            })
            .collect(),
    ))
}
pub async fn list_active_systems(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ActiveSystemsQuery>,
) -> Result<Json<Vec<V86SystemResponse>>, ProjectError> {
    let Json(mut systems) = list_systems(State(state)).await?;
    systems.retain(|system| {
        (system.is_active && system.current_version > 0 && !system.versions.is_empty())
            || query.include_version_id.is_some_and(|version_id| {
                system
                    .versions
                    .iter()
                    .any(|version| version.id == version_id)
            })
    });
    Ok(Json(systems))
}
pub async fn update_system(
    State(state): State<Arc<AppState>>,
    AxumPath(system_id): AxumPath<i64>,
    Json(request): Json<UpdateSystemRequest>,
) -> Result<StatusCode, ProjectError> {
    let current: i64 = sqlx::query_scalar("SELECT current_version FROM v86_systems WHERE id = ?")
        .bind(system_id)
        .fetch_optional(&state.project_service.pool)
        .await?
        .ok_or(ProjectError::ProjectNotFound)?;
    if request
        .expected_current_version
        .is_some_and(|expected| expected != current)
    {
        return Err(ProjectError::Conflict(
            "The system changed in another session.".to_string(),
        ));
    }
    let mut tx = state.project_service.pool.begin().await?;
    if let Some(name) = request.name {
        let name = name.trim();
        if name.is_empty() || name.len() > 100 {
            return Err(ProjectError::InvalidDemo(
                "Invalid system name.".to_string(),
            ));
        }
        sqlx::query("UPDATE v86_systems SET name = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(name)
            .bind(system_id)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(active) = request.is_active {
        sqlx::query(
            "UPDATE v86_systems SET is_active = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(active as i64)
        .bind(system_id)
        .execute(&mut *tx)
        .await?;
    }
    if request.is_default == Some(true) {
        sqlx::query("UPDATE v86_systems SET is_default = 0 WHERE is_default = 1")
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE v86_systems SET is_default = 1, is_active = 1, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(system_id)
        .execute(&mut *tx)
        .await?;
    } else if request.is_default == Some(false) {
        sqlx::query(
            "UPDATE v86_systems SET is_default = 0, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(system_id)
        .execute(&mut *tx)
        .await?;
    }
    if let Some(memory_size_mb) = request.memory_size_mb {
        validate_memory_size_mb(memory_size_mb)?;
        sqlx::query(
            "UPDATE v86_systems SET memory_size_mb = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(memory_size_mb)
        .bind(system_id)
        .execute(&mut *tx)
        .await?;
    }
    if let Some(specs) = request.specs {
        if let Some(mb) = specs.vga_memory_size_mb {
            if !(1..=32).contains(&mb) {
                return Err(ProjectError::InvalidDemo(
                    "VRAM must be between 1 and 32 MB.".to_string(),
                ));
            }
        }
        sqlx::query(
            "UPDATE v86_systems SET specs = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(serde_json::to_string(&specs).ok())
        .bind(system_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

