// The runtime descriptor the browser player boots from, plus the admin
// capture runtime and the static in-guest launcher file.
use std::collections::HashMap;

use axum::{
    Extension, Json,
    extract::{Path as AxumPath, State},
    response::Response,
};
use sqlx::Row;

use crate::domain::{entities::secret::Claims, errors::project::ProjectError};
use crate::infrastructure::web::{
    api::handlers::game::require_game_owner,
    server::AppState,
};

use super::constants::{V86_MEMORY_SIZE, V86_SAVE_FLOPPY_BYTES, V86_STATE_VERSION, V86_TOPOLOGY_VERSION};
use super::dto::{V86RuntimeDescriptor, VariantDescriptor};
use super::manifest::{
    MouseConfig, parse_mouse_config, parse_system_specs, resolve_system_machine,
    save_files_from_manifest,
};
use super::serving::streamed_fs_file;
use super::shared::user_id;
use std::sync::Arc;

/// How to locate the game behind a runtime descriptor. The public player
/// resolves a published slug; the admin snapshot studio resolves a game by
/// id so it can also work on drafts.
pub enum RuntimeLookup<'a> {
    PublishedSlug(&'a str),
    GameId(i64),
}

pub async fn runtime_descriptor(
    pool: &sqlx::SqlitePool,
    slug: &str,
    public_base_url: Option<&str>,
) -> Result<Option<V86RuntimeDescriptor>, ProjectError> {
    runtime_descriptor_for(pool, RuntimeLookup::PublishedSlug(slug), public_base_url, true).await
}

pub async fn runtime_descriptor_for(
    pool: &sqlx::SqlitePool,
    lookup: RuntimeLookup<'_>,
    public_base_url: Option<&str>,
    include_snapshot: bool,
) -> Result<Option<V86RuntimeDescriptor>, ProjectError> {
    let filter = match lookup {
        RuntimeLookup::PublishedSlug(_) => "posts.slug = ? AND posts.status = 'published'",
        RuntimeLookup::GameId(_) => "g.game_id = ?",
    };
    let sql = format!(
        r#"SELECT s.name AS system_name, s.platform_key, s.memory_size_mb, s.specs, v.id AS system_version_id,
                  v.storage_key AS base_storage_key,
                  v.size_bytes AS base_size, v.sha256 AS base_sha,
                  g.game_id, g.disk_size_bytes, g.disk_sha256,
                  g.iso_size_bytes, g.iso_sha256, g.manifest_text, g.manifest_sha256,
                  g.chunk_size_bytes, g.artifact_revision,
                  posts.slug AS slug,
                  gm.demo_width, gm.demo_height
           FROM game_v86_games g
           JOIN games gm ON gm.id = g.game_id
           JOIN posts ON posts.id = gm.post_id
           JOIN v86_system_versions v ON v.id = g.system_version_id
           JOIN v86_systems s ON s.id = v.system_id
           WHERE {filter} AND gm.launcher_type = 'v86'"#
    );
    let query = sqlx::query(&sql);
    let query = match lookup {
        RuntimeLookup::PublishedSlug(slug) => query.bind(slug),
        RuntimeLookup::GameId(id) => query.bind(id),
    };
    let row = query.fetch_optional(pool).await?;
    let Some(row) = row else { return Ok(None) };
    let slug: String = row.get("slug");
    let slug = slug.as_str();
    let version_id: i64 = row.get("system_version_id");
    let base_sha: String = row.get("base_sha");
    let base_storage_key: String = row.get("base_storage_key");
    let game_sha: String = row.get("disk_sha256");
    let iso_sha: String = row.get("iso_sha256");
    let game_id: i64 = row.get("game_id");
    let save_supported = has_save_paths(&row.get::<String, _>("manifest_text"));
    // Per-system RAM (falls back to the legacy 64 MB when the column is missing
    // on a not-yet-migrated database).
    let memory_size: u64 = row
        .try_get::<i64, _>("memory_size_mb")
        .ok()
        .map(|mb| mb.max(1) as u64 * 1024 * 1024)
        .unwrap_or(V86_MEMORY_SIZE);
    let system_specs = parse_system_specs(row.try_get::<Option<String>, _>("specs").ok().flatten().as_deref());
    let (vga_memory_size, _) = resolve_system_machine(row.get("platform_key"), &system_specs);

    // Per-variant autorun CDs. Always at least one row (backfilled on migrate).
    let variant_rows = sqlx::query(
        r#"SELECT variant_index, name, exe, args, iso_storage_key, iso_size_bytes, iso_sha256
           FROM game_v86_variants WHERE game_id = ? ORDER BY variant_index"#,
    )
    .bind(game_id)
    .fetch_all(pool)
    .await?;
    let iso_url_for = |sha: &str| match public_base_url {
        Some(base) => {
            let base = base.trim_end_matches('/');
            format!("{base}/v86/games/{sha}/full.iso")
        }
        None => format!("games/s/{slug}/v86/{sha}/full.iso"),
    };
    let snapshot_url_for = |sha: &str| match public_base_url {
        Some(base) => {
            let base = base.trim_end_matches('/');
            format!("{base}/v86/snapshots/{sha}/state.zst")
        }
        None => format!("v86/snapshots/{sha}/state.zst"),
    };

    // Variant snapshots additionally pin the disc they were captured with:
    // rebuilding a variant's CD changes its contents under a state that has
    // already cached them.
    let variant_snapshots: HashMap<i32, (String, i64)> = match include_snapshot {
        false => HashMap::new(),
        true => sqlx::query(
            r#"SELECT s.variant_index, s.sha256, s.size_bytes
               FROM game_v86_snapshots s
               JOIN game_v86_variants v
                 ON v.game_id = s.game_id AND v.variant_index = s.variant_index
               WHERE s.game_id = ? AND s.variant_index > 0
                 AND s.system_version_id = ? AND s.game_disk_sha256 = ?
                 AND s.iso_sha256 = v.iso_sha256
                 AND s.state_version = ? AND s.topology_version = ?
                 AND s.memory_size = ? AND s.vga_memory_size = ?"#,
        )
        .bind(game_id)
        .bind(version_id)
        .bind(&game_sha)
        .bind(V86_STATE_VERSION)
        .bind(V86_TOPOLOGY_VERSION)
        .bind(memory_size as i64)
        .bind(vga_memory_size as i64)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|row| {
            (
                row.get::<i32, _>("variant_index"),
                (row.get::<String, _>("sha256"), row.get::<i64, _>("size_bytes")),
            )
        })
        .collect(),
    };

    let describe_variant = |row: &sqlx::sqlite::SqliteRow| {
        let index: i32 = row.get("variant_index");
        let snapshot = variant_snapshots.get(&index);
        VariantDescriptor {
            index,
            name: row.get("name"),
            exe: row.get("exe"),
            args: row.get("args"),
            iso_url: iso_url_for(row.get::<String, _>("iso_sha256").as_str()),
            iso_size_bytes: row.get::<i64, _>("iso_size_bytes") as u64,
            iso_sha256: row.get("iso_sha256"),
            snapshot_url: snapshot.map(|(sha, _)| snapshot_url_for(sha)),
            snapshot_size_bytes: snapshot.map(|(_, size)| *size as u64),
            snapshot_sha256: snapshot.map(|(sha, _)| sha.clone()),
        }
    };

    let variants: Vec<VariantDescriptor> =
        variant_rows.iter().skip(1).map(&describe_variant).collect();
    // Variant 1 is the default and is represented both by project_v86_games'
    // legacy iso_* columns and by the top of the variants list.
    let default_variant = variant_rows.first().map(&describe_variant);

    let (base_url, game_url, iso_url) = match public_base_url {
        Some(base) => {
            let base = base.trim_end_matches('/');
            (
                format!("{base}/{base_storage_key}/.img.zst"),
                format!("{base}/v86/games/{game_sha}/.img.zst"),
                format!("{base}/v86/games/{iso_sha}/full.iso"),
            )
        }
        None => (
            format!("{base_storage_key}/.img.zst"),
            format!("games/s/{slug}/v86/disk/{game_sha}/.img.zst"),
            format!("games/s/{slug}/v86/{iso_sha}/full.iso"),
        ),
    };

    // A snapshot is only offered when every dimension it depends on still
    // matches. Anything else (replaced base disk, rebuilt game disk, upgraded
    // v86, resized memory) simply yields None and the player cold-boots, so a
    // stale snapshot can never produce a broken restore.
    let snapshot = match include_snapshot {
        false => None,
        true => {
            sqlx::query(
                r#"SELECT sha256, size_bytes FROM game_v86_snapshots
                   WHERE game_id = ? AND variant_index = 0
                     AND system_version_id = ? AND game_disk_sha256 = ?
                     AND state_version = ? AND topology_version = ?
                     AND memory_size = ? AND vga_memory_size = ?"#,
            )
            .bind(game_id)
            .bind(version_id)
            .bind(&game_sha)
            .bind(V86_STATE_VERSION)
            .bind(V86_TOPOLOGY_VERSION)
            .bind(memory_size as i64)
            .bind(vga_memory_size as i64)
            .fetch_optional(pool)
            .await?
        }
    };
    let (snapshot_url, snapshot_size_bytes, snapshot_sha256) = match snapshot {
        Some(row) => {
            let sha: String = row.get("sha256");
            (
                Some(snapshot_url_for(&sha)),
                Some(row.get::<i64, _>("size_bytes") as u64),
                Some(sha),
            )
        }
        None => (None, None, None),
    };
    let mouse_config = parse_mouse_config(&row.get::<String, _>("manifest_text"))
        .unwrap_or_else(|_| MouseConfig::default());
    Ok(Some(V86RuntimeDescriptor {
        platform_key: row.get("platform_key"),
        system_name: row.get("system_name"),
        system_version_id: version_id,
        artifact_revision: row.get("artifact_revision"),
        manifest_sha256: row.get("manifest_sha256"),
        slug: slug.to_string(),
        memory_size,
        vga_memory_size,
        display_width: row
            .get::<Option<String>, _>("demo_width")
            .unwrap_or_else(|| "100%".to_string()),
        display_height: row
            .get::<Option<String>, _>("demo_height")
            .unwrap_or_else(|| "520px".to_string()),
        chunk_size_bytes: row.get::<i64, _>("chunk_size_bytes") as u64,
        base_size_bytes: row.get::<i64, _>("base_size") as u64,
        base_sha256: base_sha.clone(),
        base_url,
        game_size_bytes: row.get::<i64, _>("disk_size_bytes") as u64,
        game_sha256: game_sha.clone(),
        game_url,
        iso_size_bytes: row.get::<i64, _>("iso_size_bytes") as u64,
        iso_sha256: iso_sha.clone(),
        iso_url,
        variants: match default_variant {
            Some(v) => {
                let mut all = vec![v];
                all.extend(variants);
                all
            }
            None => variants,
        },
        save_supported,
        save_max_bytes: V86_SAVE_FLOPPY_BYTES as u64,
        snapshot_url,
        snapshot_size_bytes,
        snapshot_sha256,
        revert_mouse_y: mouse_config.revert_mouse_y,
        mouse_speed: mouse_config.mouse_speed,
    }))
}

fn has_save_paths(manifest: &str) -> bool {
    !save_files_from_manifest(manifest)
        .map(|files| files.is_empty())
        .unwrap_or(true)
}

/// Runtime descriptor for the admin snapshot studio. Resolves by game id so
/// drafts work, and always omits the snapshot: capture must start from a cold
/// boot, never from a previously captured state.
pub async fn get_game_capture_runtime(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(game_id): AxumPath<i64>,
) -> Result<Json<V86RuntimeDescriptor>, ProjectError> {
    require_game_owner(&state, game_id, user_id(&claims)?).await?;
    runtime_descriptor_for(
        &state.project_service.pool,
        RuntimeLookup::GameId(game_id),
        state.artifact_base_url(),
        false,
    )
    .await?
    .map(Json)
    .ok_or(ProjectError::ProjectNotFound)
}

/// Serves the static Windows 9x in-guest launcher so the editor can build the
/// autorun CDs in the browser. The launcher changes rarely and feeds the
/// content hash of every CD, so it is cached for an hour.
pub async fn get_game_launcher(
    State(state): State<Arc<AppState>>,
) -> Result<Response, ProjectError> {
    let assets_dir = &state.project_demo_config.v86_assets_dir;
    // The env var points at the assets root; the launcher lives under the
    // platform folder. Fall back to a direct LAUNCHER.EXE for installs that
    // configured the platform folder directly.
    let platform_dir = assets_dir.join("v86").join("windows9x");
    let launcher = if platform_dir.join("LAUNCHER.EXE").is_file() {
        platform_dir.join("LAUNCHER.EXE")
    } else {
        assets_dir.join("LAUNCHER.EXE")
    };
    streamed_fs_file(launcher, "application/octet-stream", "public, max-age=3600").await
}

