use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};

use axum::{
    Extension, Json,
    body::Bytes,
    extract::{Path as AxumPath, Query, State},
    http::{HeaderValue, StatusCode, header},
    response::Response,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Row, Sqlite, Transaction};
use tokio_util::io::ReaderStream;
use uuid::Uuid;

use crate::{
    domain::{entities::secret::Claims, errors::project::ProjectError},
    infrastructure::{
        storage::ObjectStore,
        web::{
            api::handlers::game::require_game_owner,
            server::AppState,
        },
    },
};

const MANIFEST_MAX_BYTES: usize = 64 * 1024;

/// Capacity of the 1.44 MB FAT12 floppy used as the save transport box.
const V86_SAVE_FLOPPY_BYTES: usize = 1474560;
/// Hard cap for a floppy save upload so the body limit stays bounded.
const V86_SAVE_MAX_UPLOAD_BYTES: usize = 2 * 1024 * 1024;
/// Per-user cooldown between cloud saves (matches the client).
const V86_SAVE_RATE_LIMIT_MS: u64 = 30_000;

/// RAM / VGA the player hands to v86. A restored `initial_state` only makes
/// sense against the same sizes it was captured with, so snapshots record
/// these and `runtime_descriptor` refuses to serve a snapshot that disagrees.
const V86_MEMORY_SIZE: u64 = 64 * 1024 * 1024;
const V86_VGA_MEMORY_SIZE: u64 = 8 * 1024 * 1024;

/// v86's `save_state()` container format version (`libv86.js` throws
/// "Version mismatch" on anything else). Bumping the vendored v86 build
/// invalidates every stored snapshot, which degrades to a normal cold boot.
const V86_STATE_VERSION: i64 = 6;

/// Which set of emulated devices the player builds. A restored state assumes
/// the layout it was captured on, so changing the device list here retires
/// every older snapshot rather than restoring one into a machine it does not
/// match. Keep in sync with V86_TOPOLOGY_VERSION in V86Player.svelte.js.
const V86_TOPOLOGY_VERSION: i64 = 2;

/// Zstandard frame magic, little-endian. Snapshots are compressed in the
/// browser and stored verbatim: v86's `restore_state` sniffs this magic and
/// decompresses internally, so nothing on the server ever unpacks them.
const ZSTD_MAGIC: [u8; 4] = [0x28, 0xb5, 0x2f, 0xfd];

/// Hard cap for a snapshot upload. A 64 MB machine dumps ~72 MB raw, which
/// compresses to roughly 10-25 MB; 192 MB leaves generous headroom.
const V86_SNAPSHOT_MAX_BYTES: u64 = 192 * 1024 * 1024;

/// A single launch variant resolved from the manifest. `name` is the display
/// label and the "source of truth" for how many variants exist; `exe`/`args`
/// are that variant's coalesced values (falling back to the root keys).
#[derive(Debug, Clone)]
struct VariantSpec {
    index: i32,
    name: String,
    exe: String,
    args: String,
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

#[derive(Deserialize)]
pub struct UpdateSystemRequest {
    pub name: Option<String>,
    pub is_active: Option<bool>,
    pub is_default: Option<bool>,
    pub expected_current_version: Option<i64>,
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

fn user_id(claims: &Claims) -> Result<i64, ProjectError> {
    claims
        .user_id
        .parse::<i64>()
        .map_err(|_| ProjectError::InternalError("Cannot parse user id".to_string()))
}

fn validate_manifest(manifest: &str) -> Result<String, ProjectError> {
    if manifest.as_bytes().len() > MANIFEST_MAX_BYTES {
        return Err(ProjectError::InvalidDemo(
            "The v86 manifest cannot exceed 64 KiB.".to_string(),
        ));
    }
    if manifest.contains('\0') {
        return Err(ProjectError::InvalidDemo(
            "The v86 manifest cannot contain NUL characters.".to_string(),
        ));
    }
    Ok(hex::encode(Sha256::digest(manifest.as_bytes())))
}

fn validate_file_name(name: &str, extension: &str) -> Result<(), ProjectError> {
    let lower = name.to_ascii_lowercase();
    if !lower.ends_with(extension) || name.contains('/') || name.contains('\\') {
        return Err(ProjectError::InvalidDemo(format!(
            "Expected a {extension} file."
        )));
    }
    Ok(())
}

fn ensure_upload_not_expired(expires_at: &str) -> Result<(), ProjectError> {
    let expires_at = chrono::DateTime::parse_from_rfc3339(expires_at)
        .map_err(|_| ProjectError::InternalError("Invalid upload expiry timestamp.".to_string()))?;
    if expires_at.with_timezone(&Utc) <= Utc::now() {
        return Err(ProjectError::Conflict(
            "This upload session has expired.".to_string(),
        ));
    }
    Ok(())
}

fn storage_error(error: crate::infrastructure::storage::StorageError) -> ProjectError {
    ProjectError::InternalError(error.to_string())
}

fn transient_storage_key(kind: &str, upload_id: &str, extension: &str) -> String {
    format!("v86/tmp/{kind}/{upload_id}.{extension}")
}

/// The content-addressed object key of one disk part: the browser requests
/// parts by `{offset}-{offset+chunk_size}.img.zst`, and every part is zero-
/// padded to the full chunk size (including the last one), matching `split_asset`.
fn disk_part_name(storage_key: &str, part_index: u64, chunk_size: u64) -> String {
    let offset = part_index * chunk_size;
    format!("{storage_key}/{offset}-{}.img.zst", offset + chunk_size)
}

fn parse_part_etags(etags: Option<&str>) -> Vec<(i32, String)> {
    match etags {
        Some(text) if !text.is_empty() => serde_json::from_str::<Vec<String>>(text)
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .map(|(index, etag)| ((index as i32) + 1, etag))
            .collect(),
        _ => Vec::new(),
    }
}

fn append_part_etag(existing: Option<&str>, etag: &str) -> String {
    let mut etags: Vec<String> = match existing {
        Some(text) if !text.is_empty() => serde_json::from_str(text).unwrap_or_default(),
        _ => Vec::new(),
    };
    etags.push(etag.to_string());
    serde_json::to_string(&etags).unwrap_or_else(|_| "[]".to_string())
}

#[allow(dead_code)]
fn split_asset(
    source: &Path,
    destination: &Path,
    chunk_size: u64,
    extension: &str,
    progress: Option<&Mutex<ChunkProgress>>,
    compression_level: i32,
) -> Result<u64, ProjectError> {
    fs::create_dir_all(destination)?;
    let mut input = File::open(source)?;
    let file_len = input.metadata()?.len();
    let total_chunks = file_len.div_ceil(chunk_size as u64);
    if let Some(p) = progress {
        let mut p = p.lock().unwrap();
        p.total_chunks = total_chunks;
        p.completed_chunks = 0;
        p.message = format!("Compressing chunk 0/{total_chunks}");
    }
    let mut start = 0_u64;
    let mut count = 0_u64;
    let mut buffer = vec![0_u8; chunk_size as usize];
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        if read < buffer.len() {
            buffer[read..].fill(0);
        }
        let end = start + chunk_size;
        let part_path = destination.join(format!("{start}-{end}.{extension}"));
        let output = File::create(&part_path)?;
        let mut encoder = zstd::stream::write::Encoder::new(output, compression_level)?;
        encoder.write_all(&buffer)?;
        encoder.finish()?;
        start = end;
        count += 1;
        if let Some(p) = progress {
            let mut p = p.lock().unwrap();
            p.completed_chunks = count;
            p.message = format!("Compressing chunk {count}/{total_chunks}");
        }
    }
    if let Some(p) = progress {
        let mut p = p.lock().unwrap();
        p.completed_chunks = count;
        p.total_chunks = total_chunks;
        p.message = format!("Compressing chunk {total_chunks}/{total_chunks}");
    }
    Ok(count)
}


fn normalize_manifest_path(value: &str) -> Result<String, ProjectError> {
    let mut normalized = value.trim().trim_matches('"').replace('\\', "/");
    let upper = normalized.to_ascii_uppercase();
    if upper.starts_with("D:/GAME/") {
        normalized = normalized[8..].to_string();
    } else if upper.starts_with("D:/") {
        normalized = normalized[3..].to_string();
    }
    while normalized.starts_with("./") {
        normalized = normalized[2..].to_string();
    }
    let path = Path::new(&normalized);
    if normalized.is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(ProjectError::InvalidDemo(
            "The Windows 9x manifest contains an unsafe executable path.".to_string(),
        ));
    }
    Ok(normalized)
}

/// Parses a manifest into a lower-cased key -> value map. Line comments and
/// bare `[section]` headers are ignored, matching INI-style manifests.
fn parse_manifest_fields(manifest: &str) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    for line in manifest.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with(';')
            || (line.starts_with('[') && line.ends_with(']'))
        {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            fields.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    fields
}

/// Per-project mouse settings resolved from the manifest.
#[derive(Debug, Clone, Copy)]
pub struct MouseConfig {
    pub revert_mouse_y: bool,
    pub mouse_speed: f64,
}

impl Default for MouseConfig {
    fn default() -> Self {
        MouseConfig {
            revert_mouse_y: false,
            mouse_speed: 1.0,
        }
    }
}

/// Resolves `revert_mouse_y` and `mouse_speed` from the manifest. Missing keys
/// fall back to the defaults (no revert, 1.0 speed); present-but-invalid
/// values are rejected so a typo surfaces at upload instead of silently
/// degrading every visitor's mouse.
pub fn parse_mouse_config(manifest: &str) -> Result<MouseConfig, ProjectError> {
    let fields = parse_manifest_fields(manifest);
    let revert_mouse_y = match fields.get("revert_mouse_y").map(String::as_str) {
        None | Some("") => false,
        Some("1") | Some("true") | Some("yes") | Some("on") => true,
        Some("0") | Some("false") | Some("no") | Some("off") => false,
        Some(other) => {
            return Err(ProjectError::InvalidDemo(format!(
                "Invalid revert_mouse_y value '{other}': expected 0 or 1."
            )));
        }
    };
    let mouse_speed = match fields.get("mouse_speed").map(String::as_str) {
        None | Some("") => 1.0,
        Some(raw) => {
            let value: f64 = raw.trim().parse().map_err(|_| {
                ProjectError::InvalidDemo(format!(
                    "Invalid mouse_speed value '{raw}': expected a number."
                ))
            })?;
            if !value.is_finite() || value <= 0.0 {
                return Err(ProjectError::InvalidDemo(
                    "mouse_speed must be a positive number.".to_string(),
                ));
            }
            value
        }
    };
    Ok(MouseConfig {
        revert_mouse_y,
        mouse_speed,
    })
}

/// Lowest suffix index (`name1` -> 1, `exe3` -> 3) for a `{base}{digits}` key,
/// or `None` when the key carries no numeric suffix.
fn key_index(base: &str, key: &str) -> Option<i32> {
    let rest = key.strip_prefix(base)?;
    if rest.is_empty() || !rest.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    rest.parse::<i32>().ok()
}

/// Resolves the manifest's launch variants. Names are the source of truth:
/// the variant count comes from the highest named index, names must be
/// contiguous from 1..K, and every variant must resolve an executable.
/// Projects with no `name` keys inherit a single (unnamed) variant so existing
/// manifests keep working unchanged.
fn parse_variants(manifest: &str) -> Result<Vec<VariantSpec>, ProjectError> {
    let fields = parse_manifest_fields(manifest);

    // Names define the variant set.
    let mut name_indices = HashSet::new();
    for key in fields.keys() {
        if key == "name" || key == "name1" {
            name_indices.insert(1);
        } else if let Some(index) = key_index("name", key) {
            name_indices.insert(index.max(1));
        }
    }
    let max_name = name_indices.iter().copied().max();

    // Highest index referenced by ANY variant-scoped key (name/exe/args).
    let mut explicit_max = 0;
    for key in fields.keys() {
        for base in ["name", "exe", "args"] {
            if let Some(i) = key_index(base, key) {
                explicit_max = explicit_max.max(i);
            }
        }
    }

    let k: i32 = match max_name {
        Some(m) => m,
        None => {
            // No named variants.
            if explicit_max > 1 {
                return Err(ProjectError::InvalidDemo(
                    "Variant keys (nameN/exeN/argsN) require a name for variant 1.".to_string(),
                ));
            }
            1 // a single, unnamed (legacy) variant
        }
    };

    // Names must be contiguous 1..=K.
    for i in 1..=k {
        let named = if i == 1 {
            fields.contains_key("name") || fields.contains_key("name1")
        } else {
            fields.contains_key(&format!("name{i}"))
        };
        if !named {
            return Err(ProjectError::InvalidDemo(format!(
                "The v86 manifest must name each variant contiguously (missing name for variant {i})."
            )));
        }
    }
    // No key may reference an index beyond the named set.
    if explicit_max > k {
        return Err(ProjectError::InvalidDemo(format!(
            "Variant keys reference index {explicit_max} but only {k} named variants exist."
        )));
    }

    let mut variants = Vec::new();
    for i in 1..=k {
        let name = resolve_for(&fields, "name", i, false).unwrap_or_default();
        let exe = resolve_for(&fields, "exe", i, true)
            .ok_or_else(|| {
                ProjectError::InvalidDemo(format!(
                    "Variant {i} requires an executable (exe{i} or exe)."
                ))
            })?
            .trim()
            .to_string();
        if exe.is_empty() {
            return Err(ProjectError::InvalidDemo(format!(
                "Variant {i} requires an executable (exe{i} or exe)."
            )));
        }
        let exe = normalize_manifest_path(&exe)?;
        if !exe.to_ascii_lowercase().ends_with(".exe") {
            return Err(ProjectError::InvalidDemo(format!(
                "The Windows 9x manifest executable for variant {i} must be an .exe file."
            )));
        }
        let args = resolve_for(&fields, "args", i, true).unwrap_or_default();
        variants.push(VariantSpec { index: i, name, exe, args });
    }
    Ok(variants)
}

fn resolve_for(
    fields: &HashMap<String, String>,
    base: &str,
    index: i32,
    fallback_root: bool,
) -> Option<String> {
    let base = base.to_string();
    if index > 1 {
        return fields
            .get(&format!("{base}{index}"))
            .or_else(|| fallback_root.then(|| fields.get(&base)).flatten())
            .cloned();
    }
    fields
        .get(&base)
        .or_else(|| fields.get(&format!("{base}1")))
        .cloned()
}

const SAVE_FILE_MAX_LEN: usize = 260;
const SAVE_FILE_MAX_COUNT: usize = 64;

/// Validates a single save entry from the manifest: a relative path under the
/// D:\ game drive with `/` or `\` separators, e.g. `Save0001.dat` or
/// `A\save0001.dat`. No absolute paths, no `.`/`..` components, and none of the
/// INI-dividing chars.
fn validate_save_file(entry: &str) -> Result<(), &'static str> {
    if entry.is_empty() {
        return Err("empty");
    }
    if entry.len() > SAVE_FILE_MAX_LEN {
        return Err("too long");
    }
    for component in entry.split(['/', '\\']) {
        if component.is_empty() {
            return Err("empty path component");
        }
        if component == ".." || component == "." {
            return Err("unsafe path component");
        }
        for byte in component.bytes() {
            if !(b'!'..=b'~').contains(&byte) {
                return Err("unsupported character");
            }
            if matches!(
                byte,
                b',' | b';' | b'=' | b':' | b'"' | b'<' | b'>' | b'|' | b'?' | b'*'
            ) {
                return Err("unsupported character");
            }
        }
    }
    Ok(())
}

/// Resolves the manifest's `save_paths`/`save_path`/`saves` into the exact save
/// entries to collect (e.g. `Save0001.dat; A/save0001.dat; backup/what.bak`).
/// An entry with no separator matches that *basename* anywhere under the D:\
/// game drive root; an entry with a folder matches that exact relative path.
/// The in-guest launcher walks the whole game tree and collects whatever
/// matches, so the save layout never has to be known in advance.
fn save_files_from_manifest(manifest: &str) -> Result<Vec<String>, ProjectError> {
    let fields = parse_manifest_fields(manifest);
    let raw = fields
        .get("save_paths")
        .or_else(|| fields.get("save_path"))
        .or_else(|| fields.get("saves"))
        .map(String::as_str)
        .unwrap_or("");
    let mut files = Vec::new();
    let mut seen = HashSet::new();
    for entry in raw.split(|ch| ch == ',' || ch == ';').map(str::trim) {
        let entry = entry.trim_matches('"');
        if entry.is_empty() {
            continue;
        }
        validate_save_file(entry).map_err(|reason| {
            ProjectError::InvalidDemo(format!(
                "Invalid Windows 9x save entry '{entry}': {reason}."
            ))
        })?;
        let normalized = entry.replace('/', "\\");
        if seen.insert(normalized.to_ascii_lowercase()) {
            files.push(normalized);
            if files.len() >= SAVE_FILE_MAX_COUNT {
                break;
            }
        }
    }
    Ok(files)
}


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


pub async fn list_systems(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<V86SystemResponse>>, ProjectError> {
    let rows = sqlx::query(
        r#"SELECT s.id, s.name, s.platform_key, s.is_active, s.is_default,
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

#[derive(Serialize)]
pub struct PublicSystemVersion {
    pub id: i64,
    pub version_number: i64,
    pub system_name: String,
    pub platform_key: String,
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

/// Unguarded, and deliberately narrow: only the *current* version of each
/// active system. `get_system_chunk` serves these to anyone, so it exposes no
/// image that was not already publicly fetchable.
pub async fn list_public_systems(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<PublicSystemVersion>>, ProjectError> {
    let rows = sqlx::query(
        r#"SELECT v.id, v.version_number, v.sha256, v.storage_key, v.size_bytes, v.chunk_size_bytes,
                  s.name AS system_name, s.platform_key
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
                PublicSystemVersion {
                    id: row.get("id"),
                    version_number: row.get("version_number"),
                    system_name: row.get("system_name"),
                    platform_key: row.get("platform_key"),
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

pub async fn start_system_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<StartSystemUploadRequest>,
) -> Result<Json<StartSystemUploadResponse>, ProjectError> {
    let uploader_id = user_id(&claims)?;
    if request.platform_key != "windows9x" {
        return Err(ProjectError::InvalidDemo(
            "Only the windows9x v86 platform is currently supported.".to_string(),
        ));
    }
    validate_file_name(&request.file_name, ".img")?;
    let name = request.name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err(ProjectError::InvalidDemo(
            "A system name between 1 and 100 characters is required.".to_string(),
        ));
    }
    if request.size_bytes == 0 || request.size_bytes > state.project_demo_config.max_v86_base_size {
        return Err(ProjectError::InvalidDemo(
            "The base IMG exceeds the configured limit.".to_string(),
        ));
    }
    let existing: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM v86_systems WHERE name = ? AND id != COALESCE(?, 0)",
    )
    .bind(name)
    .bind(request.system_id)
    .fetch_one(&state.project_service.pool)
    .await?;
    if existing > 0 {
        return Err(ProjectError::InvalidDemo(
            "A system with this name already exists.".to_string(),
        ));
    }

    let chunk_size = state.project_demo_config.v86_download_chunk_size;
    let chunk_count = request.size_bytes.div_ceil(chunk_size);
    let storage_key = format!("v86/assets/systems/{}", request.sha256);

    // Content-addressed dedup: if a version with this exact sha already exists,
    // skip the upload entirely.
    let existing_version: Option<String> = sqlx::query_scalar(
        "SELECT storage_key FROM v86_system_versions WHERE sha256 = ? LIMIT 1",
    )
    .bind(&request.sha256)
    .fetch_optional(&state.project_service.pool)
    .await?;

    if let Some(existing_key) = existing_version {
        let upload_id = Uuid::new_v4().to_string();
        let expires_at =
            Utc::now() + Duration::hours(state.project_demo_config.upload_session_ttl_hours as i64);
        sqlx::query(
            r#"INSERT INTO v86_system_upload_sessions
               (id, uploader_id, system_id, name, platform_key, expected_current_version,
                original_file_name, expected_size_bytes, staged_storage_key, staged_sha256,
                staged_chunk_count, reuse, status, expires_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, 'active', ?)"#,
        )
        .bind(&upload_id)
        .bind(uploader_id)
        .bind(request.system_id)
        .bind(name)
        .bind(&request.platform_key)
        .bind(request.expected_current_version.unwrap_or(0))
        .bind(&request.file_name)
        .bind(request.size_bytes as i64)
        .bind(&existing_key)
        .bind(&request.sha256)
        .bind(chunk_count as i64)
        .bind(expires_at.to_rfc3339())
        .execute(&state.project_service.pool)
        .await?;
        return Ok(Json(StartSystemUploadResponse {
            upload_id,
            reuse: true,
            chunk_size_bytes: chunk_size,
            chunk_count,
            storage_key: Some(existing_key),
        }));
    }

    let upload_id = Uuid::new_v4().to_string();
    let expires_at =
        Utc::now() + Duration::hours(state.project_demo_config.upload_session_ttl_hours as i64);
    sqlx::query(
        r#"INSERT INTO v86_system_upload_sessions
           (id, uploader_id, system_id, name, platform_key, expected_current_version,
            original_file_name, expected_size_bytes, staged_storage_key, staged_sha256,
            staged_chunk_count, reuse, expires_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?)"#,
    )
    .bind(&upload_id)
    .bind(uploader_id)
    .bind(request.system_id)
    .bind(name)
    .bind(&request.platform_key)
    .bind(request.expected_current_version.unwrap_or(0))
    .bind(&request.file_name)
    .bind(request.size_bytes as i64)
    .bind(&storage_key)
    .bind(&request.sha256)
    .bind(chunk_count as i64)
    .bind(expires_at.to_rfc3339())
    .execute(&state.project_service.pool)
    .await?;
    Ok(Json(StartSystemUploadResponse {
        upload_id,
        reuse: false,
        chunk_size_bytes: chunk_size,
        chunk_count,
        storage_key: None,
    }))
}

async fn append_upload_chunk(
    state: &AppState,
    table: &str,
    upload_id: &str,
    uploader_id: i64,
    chunk_index: u64,
    bytes: Bytes,
) -> Result<ChunkUploadResponse, ProjectError> {
    let query = format!(
        "SELECT expected_size_bytes, received_size_bytes, next_chunk_index, upload_chunk_size_bytes, temp_storage_key, r2_upload_id, r2_part_etags, status, expires_at FROM {table} WHERE id = ? AND uploader_id = ?"
    );
    let row = sqlx::query(&query)
        .bind(upload_id)
        .bind(uploader_id)
        .fetch_optional(&state.project_service.pool)
        .await?
        .ok_or(ProjectError::ProjectNotFound)?;
    let status: String = row.get("status");
    let expected: i64 = row.get("expected_size_bytes");
    let received: i64 = row.get("received_size_bytes");
    let next: i64 = row.get("next_chunk_index");
    let chunk_size: i64 = row.get("upload_chunk_size_bytes");
    let temp_key: String = row.get("temp_storage_key");
    ensure_upload_not_expired(row.get::<String, _>("expires_at").as_str())?;
    if status != "active" || next != chunk_index as i64 {
        return Err(ProjectError::Conflict(
            "The upload chunk is stale or out of order.".to_string(),
        ));
    }
    if bytes.is_empty()
        || bytes.len() as i64 > chunk_size
        || received + bytes.len() as i64 > expected
    {
        return Err(ProjectError::InvalidDemo(
            "Invalid upload chunk size.".to_string(),
        ));
    }

    let storage = &state.storage;
    let multipart_id = row.get::<Option<String>, _>("r2_upload_id").ok_or_else(|| {
        ProjectError::InternalError("Upload session is missing its multipart id.".to_string())
    })?;
    let etag = storage
        .upload_part(&temp_key, &multipart_id, (chunk_index as i32) + 1, bytes.to_vec())
        .await
        .map_err(storage_error)?;

    let new_received = received + bytes.len() as i64;
    let new_next = next + 1;
    let new_etags = append_part_etag(row.get::<Option<String>, _>("r2_part_etags").as_deref(), &etag);
    let update = format!(
        "UPDATE {table} SET received_size_bytes = ?, next_chunk_index = ?, r2_part_etags = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND status = 'active' AND next_chunk_index = ?"
    );
    let changed = sqlx::query(&update)
        .bind(new_received)
        .bind(new_next)
        .bind(&new_etags)
        .bind(upload_id)
        .bind(next)
        .execute(&state.project_service.pool)
        .await?;
    if changed.rows_affected() != 1 {
        let _ = storage.abort_multipart(&temp_key, &multipart_id).await;
        let failed = format!(
            "UPDATE {table} SET status = 'failed', error_message = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
        );
        sqlx::query(&failed)
            .bind("Chunk relay conflict")
            .bind(upload_id)
            .execute(&state.project_service.pool)
            .await
            .ok();
        return Err(ProjectError::Conflict(
            "The upload was changed concurrently.".to_string(),
        ));
    }
    Ok(ChunkUploadResponse {
        received_size_bytes: new_received as u64,
        next_chunk_index: new_next as u64,
    })
}

pub async fn upload_system_part(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath((upload_id, part_index)): AxumPath<(String, u64)>,
    bytes: Bytes,
) -> Result<StatusCode, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let row = sqlx::query(
        r#"SELECT status, expected_size_bytes, staged_storage_key,
                  staged_chunk_count, reuse, expires_at
           FROM v86_system_upload_sessions WHERE id = ? AND uploader_id = ?"#,
    )
    .bind(&upload_id)
    .bind(uploader_id)
    .fetch_optional(&state.project_service.pool)
    .await?
    .ok_or(ProjectError::ProjectNotFound)?;
    ensure_upload_not_expired(row.get::<String, _>("expires_at").as_str())?;
    if row.get::<String, _>("status") != "active" {
        return Err(ProjectError::InvalidDemo(
            "The system upload is no longer active.".to_string(),
        ));
    }
    if row.get::<i64, _>("reuse") != 0 {
        return Err(ProjectError::Conflict(
            "The base image already exists; no upload is expected.".to_string(),
        ));
    }
    let chunk_count: i64 = row.get("staged_chunk_count");
    if part_index >= chunk_count as u64 {
        return Err(ProjectError::InvalidDemo(
            "Part index exceeds the expected chunk count.".to_string(),
        ));
    }

    let storage_key: String = row.get("staged_storage_key");
    let chunk_size: u64 = state.project_demo_config.v86_download_chunk_size;
    let offset = part_index * chunk_size;
    let end = (offset + chunk_size).min(row.get::<i64, _>("expected_size_bytes") as u64);
    let part_name = format!("{storage_key}/{offset}-{end}.img.zst");

    let storage = &state.storage;
    storage
        .put_object_bytes(&part_name, bytes.to_vec())
        .await
        .map_err(storage_error)?;

    // Record the part atomically. INSERT is concurrency-safe, unlike the old
    // read-modify-write of a received_parts JSON column.
    let changed = sqlx::query(
        "INSERT INTO v86_system_upload_parts (upload_id, part_index) VALUES (?, ?)",
    )
    .bind(&upload_id)
    .bind(part_index as i64)
    .execute(&state.project_service.pool)
    .await;
    match changed {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(sqlx::Error::Database(db_err))
            if db_err.is_unique_violation() =>
        {
            Err(ProjectError::Conflict(
                "This part was already uploaded.".to_string(),
            ))
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn abort_system_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(upload_id): AxumPath<String>,
) -> Result<StatusCode, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let row = sqlx::query(
        "SELECT staged_storage_key, reuse, status FROM v86_system_upload_sessions WHERE id = ? AND uploader_id = ?",
    )
    .bind(&upload_id)
    .bind(uploader_id)
    .fetch_optional(&state.project_service.pool)
    .await?
    .ok_or(ProjectError::ProjectNotFound)?;
    if row.get::<String, _>("status") == "consumed" {
        return Err(ProjectError::Conflict(
            "A consumed upload cannot be aborted.".to_string(),
        ));
    }
    sqlx::query(
        "UPDATE v86_system_upload_sessions SET status = 'aborted', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(&upload_id)
    .execute(&state.project_service.pool)
    .await?;
    let reuse: i64 = row.get("reuse");
    if reuse == 0 {
        if let Some(key) = row.get::<Option<String>, _>("staged_storage_key") {
            let _ = state.storage.delete_prefix(&key).await;
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn complete_system_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(upload_id): AxumPath<String>,
) -> Result<StatusCode, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let row = sqlx::query(
        r#"SELECT system_id, name, platform_key, expected_current_version, original_file_name,
                  expected_size_bytes, staged_storage_key, staged_sha256, staged_chunk_count,
                  reuse, status, expires_at
           FROM v86_system_upload_sessions WHERE id = ? AND uploader_id = ?"#,
    )
    .bind(&upload_id)
    .bind(uploader_id)
    .fetch_optional(&state.project_service.pool)
    .await?
    .ok_or(ProjectError::ProjectNotFound)?;
    ensure_upload_not_expired(row.get::<String, _>("expires_at").as_str())?;
    if row.get::<String, _>("status") != "active" {
        return Err(ProjectError::InvalidDemo(
            "The system upload is no longer active.".to_string(),
        ));
    }

    let reuse: i64 = row.get("reuse");
    if reuse == 0 {
        let chunk_count: i64 = row.get("staged_chunk_count");
        let received: Vec<i64> = sqlx::query_scalar(
            "SELECT part_index FROM v86_system_upload_parts WHERE upload_id = ?",
        )
        .bind(&upload_id)
        .fetch_all(&state.project_service.pool)
        .await?;
        if received.len() as i64 != chunk_count {
            return Err(ProjectError::InvalidDemo(
                "The base IMG upload is incomplete.".to_string(),
            ));
        }
        for index in 0..chunk_count {
            if !received.contains(&index) {
                return Err(ProjectError::InvalidDemo(
                    "The base IMG upload is incomplete.".to_string(),
                ));
            }
        }
    }

    let system_id_opt: Option<i64> = row.get("system_id");
    let expected_version: i64 = row.get("expected_current_version");
    let name: String = row.get("name");
    let platform_key: String = row.get("platform_key");
    let original_file_name: String = row.get("original_file_name");

    let existing: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM v86_systems WHERE name = ? AND id != COALESCE(?, 0)",
    )
    .bind(&name)
    .bind(system_id_opt)
    .fetch_one(&state.project_service.pool)
    .await?;
    if existing > 0 {
        return Err(ProjectError::InvalidDemo(
            "A system with this name already exists.".to_string(),
        ));
    }

    let (system_id, version_number) = {
        let mut tx = state.project_service.pool.begin().await?;
        let (sid, vn) = if let Some(sid) = system_id_opt {
            let updated = sqlx::query(
                "UPDATE v86_systems SET current_version = current_version + 1, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND current_version = ?",
            )
            .bind(sid)
            .bind(expected_version)
            .execute(&mut *tx)
            .await?;
            if updated.rows_affected() != 1 {
                return Err(ProjectError::Conflict(
                    "The system was replaced by another administrator.".to_string(),
                ));
            }
            (sid, expected_version + 1)
        } else {
            let result = sqlx::query("INSERT INTO v86_systems (name, platform_key) VALUES (?, ?)")
                .bind(&name)
                .bind(&platform_key)
                .execute(&mut *tx)
                .await?;
            let new_id = result.last_insert_rowid();
            sqlx::query("UPDATE v86_systems SET current_version = 1 WHERE id = ?")
                .bind(new_id)
                .execute(&mut *tx)
                .await?;
            (new_id, 1)
        };
        tx.commit().await?;
        (sid, vn)
    };

    if reuse != 0 {
        // Dedup: image already exists; create version row and mark consumed immediately.
        let storage_key: String = row.get("staged_storage_key");
        let expected_size: i64 = row.get("expected_size_bytes");
        let sha256: String = row.get("staged_sha256");
        let chunk_count: i64 = row.get("staged_chunk_count");
        let chunk_size: i64 = state.project_demo_config.v86_download_chunk_size as i64;
        sqlx::query(
            r#"INSERT INTO v86_system_versions
               (system_id, version_number, original_file_name, storage_key, size_bytes,
                sha256, chunk_size_bytes, chunk_count)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(system_id)
        .bind(version_number)
        .bind(&original_file_name)
        .bind(&storage_key)
        .bind(expected_size)
        .bind(&sha256)
        .bind(chunk_size)
        .bind(chunk_count)
        .execute(&state.project_service.pool)
        .await
        .map_err(|e| ProjectError::InternalError(format!("Failed to create version: {e}")))?;
        sqlx::query(
            "UPDATE v86_system_upload_sessions SET status = 'consumed', system_id = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(system_id)
        .bind(&upload_id)
        .execute(&state.project_service.pool)
        .await?;
        return Ok(StatusCode::OK);
    }

    let chunk_count: i64 = row.get("staged_chunk_count");
    let storage_key: String = row.get("staged_storage_key");
    let sha256: String = row.get("staged_sha256");
    let expected_size: i64 = row.get("expected_size_bytes");
    let chunk_size: i64 = state.project_demo_config.v86_download_chunk_size as i64;

    // The client hashes the IMG and validates its boot sector before uploading,
    // so the server only records arrival — no decompression or re-hashing. The
    // parts are content-addressed under the sha the client reported.
    sqlx::query(
        r#"INSERT INTO v86_system_versions
           (system_id, version_number, original_file_name, storage_key, size_bytes,
            sha256, chunk_size_bytes, chunk_count)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(system_id)
    .bind(version_number)
    .bind(&original_file_name)
    .bind(&storage_key)
    .bind(expected_size)
    .bind(&sha256)
    .bind(chunk_size)
    .bind(chunk_count)
    .execute(&state.project_service.pool)
    .await
    .map_err(|e| ProjectError::InternalError(format!("Failed to create version: {e}")))?;
    sqlx::query(
        "UPDATE v86_system_upload_sessions SET status = 'consumed', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(&upload_id)
    .execute(&state.project_service.pool)
    .await?;
    Ok(StatusCode::OK)
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
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Clone, Serialize)]
pub struct ChunkProgress {
    pub upload_id: String,
    pub kind: String,
    pub total_chunks: u64,
    pub completed_chunks: u64,
    pub message: String,
}

fn chunk_progress_map() -> &'static Mutex<HashMap<String, ChunkProgress>> {
    static MAP: OnceLock<Mutex<HashMap<String, ChunkProgress>>> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Serialize)]
pub struct UploadStatusResponse {
    pub status: String,
    pub error_message: Option<String>,
    pub chunk_progress: Option<ChunkProgress>,
    pub active_uploads: Vec<ChunkProgress>,
}

pub async fn get_system_upload_status(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(upload_id): AxumPath<String>,
) -> Result<Json<UploadStatusResponse>, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let row = sqlx::query(
        "SELECT status, error_message FROM v86_system_upload_sessions WHERE id = ? AND uploader_id = ?",
    )
    .bind(&upload_id)
    .bind(uploader_id)
    .fetch_optional(&state.project_service.pool)
    .await?
    .ok_or(ProjectError::ProjectNotFound)?;
    let progress = chunk_progress_map().lock().unwrap().get(&upload_id).cloned();
    let active_uploads = chunk_progress_map()
        .lock()
        .unwrap()
        .values()
        .cloned()
        .collect();
    Ok(Json(UploadStatusResponse {
        status: row.get("status"),
        error_message: row.get("error_message"),
        chunk_progress: progress,
        active_uploads,
    }))
}

#[derive(Serialize)]
pub struct ServerStatusResponse {
    pub ok: bool,
    pub active_uploads: Vec<ChunkProgress>,
}

pub async fn get_server_status(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<ServerStatusResponse>, ProjectError> {
    let active_uploads = chunk_progress_map()
        .lock()
        .unwrap()
        .values()
        .cloned()
        .collect();
    Ok(Json(ServerStatusResponse {
        ok: true,
        active_uploads,
    }))
}

pub async fn delete_system_version(
    State(state): State<Arc<AppState>>,
    AxumPath((system_id, version_id)): AxumPath<(i64, i64)>,
) -> Result<StatusCode, ProjectError> {
    let usage: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM game_v86_games WHERE system_version_id = ?")
            .bind(version_id)
            .fetch_one(&state.project_service.pool)
            .await?;
    if usage > 0 {
        return Err(ProjectError::Conflict(format!(
            "This system version is used by {usage} game(s) and cannot be deleted."
        )));
    }
    let row = sqlx::query(
        "SELECT v.storage_key, v.version_number, s.current_version FROM v86_system_versions v JOIN v86_systems s ON s.id = v.system_id WHERE v.id = ? AND v.system_id = ?",
    )
    .bind(version_id)
    .bind(system_id)
    .fetch_optional(&state.project_service.pool)
    .await?
    .ok_or(ProjectError::ProjectNotFound)?;
    if row.get::<i64, _>("version_number") == row.get::<i64, _>("current_version") {
        return Err(ProjectError::Conflict(
            "The current system version cannot be deleted; replace it first.".to_string(),
        ));
    }
    sqlx::query(
        "DELETE FROM project_v86_upload_sessions WHERE system_version_id = ?",
    )
    .bind(version_id)
    .execute(&state.project_service.pool)
    .await?;
    let storage_key: String = row.get("storage_key");
    sqlx::query("DELETE FROM v86_system_versions WHERE id = ?")
        .bind(version_id)
        .execute(&state.project_service.pool)
        .await?;
    // Content-addressed: only delete the prefix if no other version references it.
    let remaining: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM v86_system_versions WHERE storage_key = ?",
    )
    .bind(&storage_key)
    .fetch_one(&state.project_service.pool)
    .await?;
    if remaining == 0 {
        let _ = state.storage.delete_prefix(&storage_key).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_system(
    State(state): State<Arc<AppState>>,
    AxumPath(system_id): AxumPath<i64>,
) -> Result<StatusCode, ProjectError> {
    let usage: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM game_v86_games g
           JOIN v86_system_versions v ON v.id = g.system_version_id
           WHERE v.system_id = ?"#,
    )
    .bind(system_id)
    .fetch_one(&state.project_service.pool)
    .await?;
    if usage > 0 {
        return Err(ProjectError::Conflict(format!(
            "This system is referenced by {usage} game(s); deactivate it instead."
        )));
    }
    let keys: Vec<String> =
        sqlx::query_scalar("SELECT storage_key FROM v86_system_versions WHERE system_id = ?")
            .bind(system_id)
            .fetch_all(&state.project_service.pool)
            .await?;
    sqlx::query(
        r#"DELETE FROM project_v86_upload_sessions
           WHERE system_version_id IN (
             SELECT id FROM v86_system_versions WHERE system_id = ?
           )"#,
    )
    .bind(system_id)
    .execute(&state.project_service.pool)
    .await?;
    let changed = sqlx::query("DELETE FROM v86_systems WHERE id = ?")
        .bind(system_id)
        .execute(&state.project_service.pool)
        .await?;
    if changed.rows_affected() != 1 {
        return Err(ProjectError::ProjectNotFound);
    }
    for key in &keys {
        let remaining: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM v86_system_versions WHERE storage_key = ?",
        )
        .bind(key)
        .fetch_one(&state.project_service.pool)
        .await?;
        if remaining == 0 {
            let _ = state.storage.delete_prefix(key).await;
        }
    }
    Ok(StatusCode::NO_CONTENT)
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
            require_game_owner(&state, game_id, uploader_id).await?;
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
    .bind(&first.sha256)
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

/// The stored artifact of a project, used to resolve the manifest-only fast
/// path (reuse the stored disk) and to validate the expected revision.
struct StoredGameArtifact {
    artifact_revision: i64,
    disk_sha256: Option<String>,
    disk_size_bytes: Option<i64>,
    chunk_count: i64,
}

async fn fetch_stored_game_artifact(
    pool: &sqlx::SqlitePool,
    game_id: i64,
) -> Result<Option<StoredGameArtifact>, String> {
    let row = sqlx::query(
        r#"SELECT artifact_revision, disk_sha256, disk_size_bytes, chunk_count
           FROM game_v86_games WHERE game_id = ?"#,
    )
    .bind(game_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(row.map(|row| StoredGameArtifact {
        artifact_revision: row.get("artifact_revision"),
        disk_sha256: row.get("disk_sha256"),
        disk_size_bytes: row.get("disk_size_bytes"),
        chunk_count: row.get("chunk_count"),
    }))
}

/// Deletes the content-addressed artifacts this session uploaded, but never
/// shared/reused objects: the parts live under the disk sha prefix only when
/// the client actually uploaded them, and reused variant CDs are skipped.
async fn delete_uploaded_game_artifacts(
    storage: &ObjectStore,
    pool: &sqlx::SqlitePool,
    upload_id: &str,
    disk_storage_key: Option<&str>,
    disk_reuse: bool,
) {
    if !disk_reuse {
        if let Some(key) = disk_storage_key {
            let _ = storage.delete_prefix(key).await;
        }
    }
    let uploaded_isos: Vec<String> = sqlx::query_scalar(
        "SELECT iso_storage_key FROM project_v86_staged_variants WHERE upload_id = ? AND reuse = 0",
    )
    .bind(upload_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    for key in uploaded_isos {
        let _ = storage.delete_object(&format!("{key}/full.iso")).await;
    }
}

pub async fn complete_game_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(upload_id): AxumPath<String>,
) -> Result<StatusCode, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let row = sqlx::query(
        r#"SELECT staged_disk_chunk_count, disk_reuse, status, expires_at
           FROM project_v86_upload_sessions WHERE id = ? AND uploader_id = ?"#,
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

    // Every non-reused artifact must have arrived before the session can be
    // finalized. The client already built, hashed, and self-checked the disk
    // and CD images against this plan (see upload-controller.js), so the
    // server only records arrival — no decompression or re-hashing. Parts are
    // tracked atomically, and a part's R2 object is written before its
    // tracking row, so a complete session has all its content in R2.
    let disk_reuse: i64 = row.get("disk_reuse");
    if disk_reuse == 0 {
        let chunk_count: i64 = row.get("staged_disk_chunk_count");
        let received: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM project_v86_received_disk_parts WHERE upload_id = ?",
        )
        .bind(&upload_id)
        .fetch_one(&state.project_service.pool)
        .await?;
        if received != chunk_count {
            return Err(ProjectError::InvalidDemo(
                "The game disk upload is incomplete.".to_string(),
            ));
        }
    }
    let missing_isos: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM project_v86_staged_variants
           WHERE upload_id = ? AND reuse = 0 AND received = 0"#,
    )
    .bind(&upload_id)
    .fetch_one(&state.project_service.pool)
    .await?;
    if missing_isos > 0 {
        return Err(ProjectError::InvalidDemo(
            "The v86 game upload is incomplete.".to_string(),
        ));
    }

    sqlx::query(
        "UPDATE project_v86_upload_sessions SET status = 'ready', updated_at = CURRENT_TIMESTAMP WHERE id = ? AND status = 'active'",
    )
    .bind(&upload_id)
    .execute(&state.project_service.pool)
    .await?;
    Ok(StatusCode::OK)
}


pub async fn attach_ready_game_tx(
    tx: &mut Transaction<'_, Sqlite>,
    game_id: i64,
    uploader_id: i64,
    upload_id: &str,
    chunk_size: u64,
) -> Result<i64, ProjectError> {
    let row = sqlx::query(
        r#"SELECT source_project_id, system_version_id, expected_artifact_revision,
                  manifest_text, manifest_sha256,
                  staged_disk_storage_key, staged_disk_sha256, staged_disk_size_bytes,
                  staged_disk_chunk_count,
                  staged_iso_storage_key, staged_iso_sha256, staged_iso_size_bytes,
                  status, expires_at
           FROM project_v86_upload_sessions
           WHERE id = ? AND uploader_id = ?"#,
    )
    .bind(upload_id)
    .bind(uploader_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(ProjectError::ProjectNotFound)?;
    ensure_upload_not_expired(row.get::<String, _>("expires_at").as_str())?;
    if row.get::<String, _>("status") != "ready" {
        return Err(ProjectError::InvalidDemo(
            "The v86 game package is not ready.".to_string(),
        ));
    }
    let source_project: Option<i64> = row.get("source_project_id");
    if source_project.is_some() && source_project != Some(game_id) {
        return Err(ProjectError::Forbidden);
    }
    let expected: i64 = row.get("expected_artifact_revision");
    let current: i64 = sqlx::query_scalar(
        "SELECT COALESCE((SELECT artifact_revision FROM game_v86_games WHERE game_id = ?), 0)",
    )
    .bind(game_id)
    .fetch_one(&mut **tx)
    .await?;
    if current != expected {
        return Err(ProjectError::Conflict(
            "The v86 artifact changed while this package was building.".to_string(),
        ));
    }
    let revision = current + 1;
    let artifact_change = sqlx::query(
        r#"INSERT INTO game_v86_games
           (game_id, system_version_id, manifest_text, manifest_sha256,
            launcher_config_sha256, game_config_sha256,
            disk_storage_key, disk_size_bytes, disk_sha256,
            iso_storage_key, iso_size_bytes, iso_sha256, chunk_size_bytes,
            chunk_count, artifact_revision)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
           ON CONFLICT(game_id) DO UPDATE SET
             system_version_id = excluded.system_version_id,
             manifest_text = excluded.manifest_text,
             manifest_sha256 = excluded.manifest_sha256,
             launcher_config_sha256 = excluded.launcher_config_sha256,
             game_config_sha256 = excluded.game_config_sha256,
             disk_storage_key = excluded.disk_storage_key,
             disk_size_bytes = excluded.disk_size_bytes,
             disk_sha256 = excluded.disk_sha256,
             iso_storage_key = excluded.iso_storage_key,
             iso_size_bytes = excluded.iso_size_bytes,
             iso_sha256 = excluded.iso_sha256,
             chunk_size_bytes = excluded.chunk_size_bytes,
             chunk_count = excluded.chunk_count,
             artifact_revision = excluded.artifact_revision,
             updated_at = CURRENT_TIMESTAMP
           WHERE game_v86_games.artifact_revision = ?"#,
    )
    .bind(game_id)
    .bind(row.get::<i64, _>("system_version_id"))
    .bind(row.get::<String, _>("manifest_text"))
    .bind(row.get::<String, _>("manifest_sha256"))
    .bind(row.get::<String, _>("manifest_sha256"))
    .bind(row.get::<String, _>("manifest_sha256"))
    .bind(row.get::<Option<String>, _>("staged_disk_storage_key"))
    .bind(row.get::<Option<i64>, _>("staged_disk_size_bytes"))
    .bind(row.get::<Option<String>, _>("staged_disk_sha256"))
    .bind(row.get::<String, _>("staged_iso_storage_key"))
    .bind(row.get::<i64, _>("staged_iso_size_bytes"))
    .bind(row.get::<String, _>("staged_iso_sha256"))
    .bind(chunk_size as i64)
    .bind(row.get::<i64, _>("staged_disk_chunk_count"))
    .bind(revision)
    .bind(expected)
    .execute(&mut **tx)
    .await?;
    if artifact_change.rows_affected() != 1 {
        return Err(ProjectError::Conflict(
            "The v86 artifact changed while this package was building.".to_string(),
        ));
    }
    // Replace the game's variant CDs with the newly staged set. Variant 1
    // mirrors the iso_* columns on game_v86_games (kept for compatibility).
    sqlx::query("DELETE FROM game_v86_variants WHERE game_id = ?")
        .bind(game_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query(
        r#"INSERT INTO game_v86_variants
           (game_id, variant_index, name, exe, args, iso_storage_key, iso_size_bytes, iso_sha256)
           SELECT ?, variant_index, name, exe, args, iso_storage_key, iso_size_bytes, iso_sha256
           FROM project_v86_staged_variants WHERE upload_id = ?"#,
    )
    .bind(game_id)
    .bind(upload_id)
    .execute(&mut **tx)
    .await?;
    let changed = sqlx::query(
        "UPDATE project_v86_upload_sessions SET status = 'consumed', updated_at = CURRENT_TIMESTAMP WHERE id = ? AND status = 'ready'",
    )
    .bind(upload_id)
    .execute(&mut **tx)
    .await?;
    if changed.rows_affected() != 1 {
        return Err(ProjectError::Conflict(
            "The staged v86 package was already consumed.".to_string(),
        ));
    }
    Ok(revision)
}

pub async fn abort_game_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(upload_id): AxumPath<String>,
) -> Result<StatusCode, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let row = sqlx::query(
        "SELECT staged_disk_storage_key, disk_reuse, status FROM project_v86_upload_sessions WHERE id = ? AND uploader_id = ?",
    )
    .bind(&upload_id)
    .bind(uploader_id)
    .fetch_optional(&state.project_service.pool)
    .await?
    .ok_or(ProjectError::ProjectNotFound)?;
    let status: String = row.get("status");
    if status == "consumed" {
        return Err(ProjectError::Conflict(
            "A consumed upload cannot be aborted.".to_string(),
        ));
    }
    sqlx::query(
        "UPDATE project_v86_upload_sessions SET status = 'aborted', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(&upload_id)
    .execute(&state.project_service.pool)
    .await?;
    // Remove only the content this session uploaded. Shared/reused artifacts
    // are left untouched.
    let disk_key: Option<String> = row.get("staged_disk_storage_key");
    let disk_reuse: i64 = row.get("disk_reuse");
    delete_uploaded_game_artifacts(
        &state.storage,
        &state.project_service.pool,
        &upload_id,
        disk_key.as_deref(),
        disk_reuse != 0,
    )
    .await;
    chunk_progress_map().lock().unwrap().remove(&upload_id);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_game_upload_status(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(upload_id): AxumPath<String>,
) -> Result<Json<UploadStatusResponse>, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let row = sqlx::query(
        "SELECT status, error_message FROM project_v86_upload_sessions WHERE id = ? AND uploader_id = ?",
    )
    .bind(&upload_id)
    .bind(uploader_id)
    .fetch_optional(&state.project_service.pool)
    .await?
    .ok_or(ProjectError::ProjectNotFound)?;
    let progress = chunk_progress_map().lock().unwrap().get(&upload_id).cloned();
    let active_uploads = chunk_progress_map()
        .lock()
        .unwrap()
        .values()
        .cloned()
        .collect();
    Ok(Json(UploadStatusResponse {
        status: row.get("status"),
        error_message: row.get("error_message"),
        chunk_progress: progress,
        active_uploads,
    }))
}

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
        r#"SELECT s.name AS system_name, s.platform_key, v.id AS system_version_id,
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
        .bind(V86_MEMORY_SIZE as i64)
        .bind(V86_VGA_MEMORY_SIZE as i64)
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
            .bind(V86_MEMORY_SIZE as i64)
            .bind(V86_VGA_MEMORY_SIZE as i64)
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
        memory_size: V86_MEMORY_SIZE,
        vga_memory_size: V86_VGA_MEMORY_SIZE,
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

/// Storage key for a snapshot blob. Content-addressed, so identical states
/// dedupe and the object can be cached immutably forever.
fn snapshot_storage_key(sha256: &str) -> String {
    format!("v86/snapshots/{sha256}/state.zst")
}

fn validate_sha256_hex(value: &str) -> Result<(), ProjectError> {
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(ProjectError::InvalidDemo(
            "Expected a hex-encoded sha256 digest.".to_string(),
        ));
    }
    Ok(())
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

pub async fn get_game_snapshot(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(game_id): AxumPath<i64>,
) -> Result<Json<Vec<SnapshotStatusResponse>>, ProjectError> {
    require_game_owner(&state, game_id, user_id(&claims)?).await?;
    // A variant row is only fresh if its disc still matches too, so the join
    // to game_v86_variants is deliberately left (a deleted variant leaves
    // its snapshot present but stale rather than hiding it).
    let rows = sqlx::query(
        r#"SELECT s.variant_index, s.size_bytes, s.raw_size_bytes, s.created_at,
                  (s.system_version_id = g.system_version_id
                   AND s.game_disk_sha256 = g.disk_sha256
                   AND s.state_version = ?
                   AND s.topology_version = ?
                   AND s.memory_size = ?
                   AND s.vga_memory_size = ?
                   AND (s.variant_index = 0 OR s.iso_sha256 = v.iso_sha256)) AS fresh
           FROM game_v86_snapshots s
           JOIN game_v86_games g ON g.game_id = s.game_id
           LEFT JOIN game_v86_variants v
             ON v.game_id = s.game_id AND v.variant_index = s.variant_index
           WHERE s.game_id = ?
           ORDER BY s.variant_index"#,
    )
    .bind(V86_STATE_VERSION)
    .bind(V86_TOPOLOGY_VERSION)
    .bind(V86_MEMORY_SIZE as i64)
    .bind(V86_VGA_MEMORY_SIZE as i64)
    .bind(game_id)
    .fetch_all(&state.project_service.pool)
    .await?;
    Ok(Json(
        rows.iter()
            .map(|row| SnapshotStatusResponse {
                variant_index: row.get("variant_index"),
                exists: true,
                stale: row.get::<Option<i64>, _>("fresh").unwrap_or(0) == 0,
                size_bytes: Some(row.get::<i64, _>("size_bytes") as u64),
                raw_size_bytes: Some(row.get::<i64, _>("raw_size_bytes") as u64),
                created_at: Some(row.get("created_at")),
            })
            .collect(),
    ))
}

pub async fn start_snapshot_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<StartSnapshotUploadRequest>,
) -> Result<Json<StartUploadResponse>, ProjectError> {
    let uploader_id = user_id(&claims)?;
    require_game_owner(&state, request.game_id, uploader_id).await?;
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
    if request.memory_size != V86_MEMORY_SIZE || request.vga_memory_size != V86_VGA_MEMORY_SIZE {
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

    // The snapshot embeds dirty blocks from these exact disks; if the game
    // has been re-uploaded since capture started, the state is already void.
    let game = sqlx::query(
        "SELECT system_version_id, disk_sha256 FROM game_v86_games WHERE game_id = ?",
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
    .map_err(|error| {
        let storage = state.storage.clone();
        let multipart_id = multipart_id.clone();
        tokio::spawn(async move {
            let _ = storage.abort_multipart(&transient_key, &multipart_id).await;
        });
        error
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

async fn fail_snapshot_session(state: &AppState, upload_id: &str, message: &str) {
    sqlx::query(
        "UPDATE game_v86_snapshot_upload_sessions SET status = 'failed', error_message = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(message)
    .bind(upload_id)
    .execute(&state.project_service.pool)
    .await
    .ok();
}

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
    require_game_owner(&state, game_id, uploader_id).await?;

    let storage = &state.storage;
    let temp_key: String = row.get("temp_storage_key");
    let multipart_id = row.get::<Option<String>, _>("r2_upload_id").ok_or_else(|| {
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
    let game = sqlx::query(
        "SELECT system_version_id, disk_sha256 FROM game_v86_games WHERE game_id = ?",
    )
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
    if let Some(previous) = previous_key {
        if previous != storage_key {
            let still_referenced: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM game_v86_snapshots WHERE storage_key = ?",
            )
            .bind(&previous)
            .fetch_one(&state.project_service.pool)
            .await
            .unwrap_or(1);
            if still_referenced == 0 {
                let _ = storage.delete_object(&previous).await;
            }
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
    if row.get::<String, _>("status") == "active" {
        if let Some(multipart_id) = row.get::<Option<String>, _>("r2_upload_id") {
            let _ = state
                .storage
                .abort_multipart(
                    &row.get::<String, _>("temp_storage_key"),
                    &multipart_id,
                )
                .await;
        }
    }
    sqlx::query(
        "UPDATE game_v86_snapshot_upload_sessions SET status = 'aborted', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(&upload_id)
    .execute(&state.project_service.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_game_snapshot(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath((game_id, variant_index)): AxumPath<(i64, i32)>,
) -> Result<StatusCode, ProjectError> {
    require_game_owner(&state, game_id, user_id(&claims)?).await?;
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

/// Public serve for the local-storage fallback (no R2 public domain). With R2
/// configured the descriptor points straight at the bucket instead.
pub async fn get_snapshot_blob(
    State(state): State<Arc<AppState>>,
    AxumPath((sha256, part)): AxumPath<(String, String)>,
) -> Result<Response, ProjectError> {
    if part != "state.zst" {
        return Err(ProjectError::ProjectNotFound);
    }
    validate_sha256_hex(&sha256).map_err(|_| ProjectError::ProjectNotFound)?;
    // Only serve digests referenced by a published project, matching the disk
    // and ISO routes. A snapshot is a fully booted machine with the game on
    // it, so a draft's state must not be reachable before the post goes live.
    let storage_key: Option<String> = sqlx::query_scalar(
        r#"SELECT s.storage_key FROM game_v86_snapshots s
           JOIN games gm ON gm.id = s.game_id
           JOIN posts ON posts.id = gm.post_id
           WHERE s.sha256 = ? AND posts.status = 'published' AND gm.launcher_type = 'v86'
           LIMIT 1"#,
    )
    .bind(&sha256)
    .fetch_optional(&state.project_service.pool)
    .await?;
    let storage_key = storage_key.ok_or(ProjectError::ProjectNotFound)?;
    match &state.storage {
        ObjectStore::R2(_) => {
            streamed_object(
                &state.storage,
                &storage_key,
                "application/octet-stream",
                IMMUTABLE_CACHE_CONTROL,
            )
            .await
        }
        ObjectStore::Fs(_) => {
            streamed_fs_file(
                state.project_demo_config.dir.join(&storage_key),
                "application/octet-stream",
                IMMUTABLE_CACHE_CONTROL,
            )
            .await
        }
    }
}

const IMMUTABLE_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

fn streamed_response(
    body: axum::body::Body,
    size: u64,
    content_type: &'static str,
    cache_control: &'static str,
) -> Response {
    let mut response = Response::new(body);
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(cache_control),
    );
    if let Ok(value) = HeaderValue::from_str(&size.to_string()) {
        response
            .headers_mut()
            .insert(header::CONTENT_LENGTH, value);
    }
    response
}

async fn streamed_fs_file(
    path: PathBuf,
    content_type: &'static str,
    cache_control: &'static str,
) -> Result<Response, ProjectError> {
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|_| ProjectError::ProjectNotFound)?;
    let size = file
        .metadata()
        .await
        .map_err(|_| ProjectError::ProjectNotFound)?
        .len();
    Ok(streamed_response(
        axum::body::Body::from_stream(ReaderStream::new(file)),
        size,
        content_type,
        cache_control,
    ))
}

/// Streams an object from the store by key. Keys mirror the R2 layout, so the
/// same key resolves in either backend.
async fn streamed_object(
    storage: &ObjectStore,
    key: &str,
    content_type: &'static str,
    cache_control: &'static str,
) -> Result<Response, ProjectError> {
    let size = storage
        .object_size(key)
        .await
        .map_err(storage_error)?
        .ok_or(ProjectError::ProjectNotFound)?;
    let reader = storage.get_object_reader(key).await.map_err(storage_error)?;
    Ok(streamed_response(
        axum::body::Body::from_stream(ReaderStream::new(reader)),
        size,
        content_type,
        cache_control,
    ))
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

pub async fn get_system_chunk(
    State(state): State<Arc<AppState>>,
    AxumPath((sha256, part)): AxumPath<(String, String)>,
) -> Result<Response, ProjectError> {
    let storage_key: Option<String> = sqlx::query_scalar(
        r#"SELECT v.storage_key FROM v86_system_versions v
           JOIN v86_systems s ON s.id = v.system_id
           WHERE v.sha256 = ?
             AND s.is_active = 1
             AND v.version_number = s.current_version
           LIMIT 1"#,
    )
    .bind(&sha256)
    .fetch_optional(&state.project_service.pool)
    .await?;
    let storage_key = storage_key.ok_or(ProjectError::ProjectNotFound)?;
    if part == ".img" || part.contains('/') || !(part.ends_with(".img") || part.ends_with(".img.zst")) {
        return Err(ProjectError::ProjectNotFound);
    }
    streamed_fs_file(
        state.project_demo_config.dir.join(&storage_key).join(part),
        "application/octet-stream",
        IMMUTABLE_CACHE_CONTROL,
    )
    .await
}

pub async fn get_game_chunk(
    State(state): State<Arc<AppState>>,
    AxumPath((slug, sha256, part)): AxumPath<(String, String, String)>,
) -> Result<Response, ProjectError> {
    let storage_key: Option<String> = sqlx::query_scalar(
        r#"SELECT iso_storage_key
           FROM (
                SELECT g.iso_storage_key FROM game_v86_games g
                JOIN games gm ON gm.id = g.game_id
                JOIN posts ON posts.id = gm.post_id
                WHERE posts.slug = ? AND posts.status = 'published'
                  AND gm.launcher_type = 'v86' AND g.iso_sha256 = ?
             UNION
                SELECT v.iso_storage_key FROM game_v86_variants v
                JOIN games gm ON gm.id = v.game_id
                JOIN posts ON posts.id = gm.post_id
                WHERE posts.slug = ? AND posts.status = 'published'
                  AND gm.launcher_type = 'v86' AND v.iso_sha256 = ?
           ) LIMIT 1"#,
    )
    .bind(&slug)
    .bind(&sha256)
    .bind(&slug)
    .bind(&sha256)
    .fetch_optional(&state.project_service.pool)
    .await?;
    let storage_key = storage_key.ok_or(ProjectError::ProjectNotFound)?;
    if part == ".iso" || part.contains('/') || !(part.ends_with(".iso") || part.ends_with(".iso.zst")) {
        return Err(ProjectError::ProjectNotFound);
    }
    streamed_fs_file(
        state
            .project_demo_config
            .dir
            .join(storage_key)
            .join("parts")
            .join(part),
        "application/octet-stream",
        IMMUTABLE_CACHE_CONTROL,
    )
    .await
}

pub async fn get_game_disk_chunk(
    State(state): State<Arc<AppState>>,
    AxumPath((slug, sha256, part)): AxumPath<(String, String, String)>,
) -> Result<Response, ProjectError> {
    let storage_key: Option<String> = sqlx::query_scalar(
        r#"SELECT g.disk_storage_key FROM game_v86_games g
           JOIN games gm ON gm.id = g.game_id
           JOIN posts ON posts.id = gm.post_id
           WHERE posts.slug = ? AND posts.status = 'published'
             AND gm.launcher_type = 'v86' AND g.disk_sha256 = ?"#,
    )
    .bind(&slug)
    .bind(&sha256)
    .fetch_optional(&state.project_service.pool)
    .await?;
    let storage_key = storage_key.ok_or(ProjectError::ProjectNotFound)?;
    if part == ".img" || part.contains('/') || !(part.ends_with(".img") || part.ends_with(".img.zst"))
    {
        return Err(ProjectError::ProjectNotFound);
    }
    match &state.storage {
        ObjectStore::R2(_) => {
            let key = format!("{storage_key}/{part}");
            streamed_object(
                &state.storage,
                &key,
                "application/octet-stream",
                IMMUTABLE_CACHE_CONTROL,
            )
            .await
        }
        ObjectStore::Fs(_) => {
            streamed_fs_file(
                state.project_demo_config.dir.join(storage_key).join(part),
                "application/octet-stream",
                IMMUTABLE_CACHE_CONTROL,
            )
            .await
        }
    }
}

pub async fn get_game_iso(
    State(state): State<Arc<AppState>>,
    AxumPath((slug, sha256)): AxumPath<(String, String)>,
) -> Result<Response, ProjectError> {
    let storage_key: Option<String> = sqlx::query_scalar(
        r#"SELECT iso_storage_key
           FROM (
                SELECT g.iso_storage_key FROM game_v86_games g
                JOIN games gm ON gm.id = g.game_id
                JOIN posts ON posts.id = gm.post_id
                WHERE posts.slug = ? AND posts.status = 'published'
                  AND gm.launcher_type = 'v86' AND g.iso_sha256 = ?
             UNION
                SELECT v.iso_storage_key FROM game_v86_variants v
                JOIN games gm ON gm.id = v.game_id
                JOIN posts ON posts.id = gm.post_id
                WHERE posts.slug = ? AND posts.status = 'published'
                  AND gm.launcher_type = 'v86' AND v.iso_sha256 = ?
           ) LIMIT 1"#,
    )
    .bind(&slug)
    .bind(&sha256)
    .bind(&slug)
    .bind(&sha256)
    .fetch_optional(&state.project_service.pool)
    .await?;
    let storage_key = storage_key.ok_or(ProjectError::ProjectNotFound)?;
    if Path::new(&storage_key).components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(ProjectError::InternalError(
            "Invalid v86 game storage key.".to_string(),
        ));
    }

    match &state.storage {
        ObjectStore::R2(_) => {
            let key = format!("v86/games/{sha256}/full.iso");
            let size = state
                .storage
                .object_size(&key)
                .await
                .map_err(storage_error)?
                .ok_or(ProjectError::ProjectNotFound)?;
            let reader = state
                .storage
                .get_object_reader(&key)
                .await
                .map_err(storage_error)?;
            let mut response = Response::new(axum::body::Body::from_stream(ReaderStream::new(reader)));
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/octet-stream"),
            );
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            );
            response.headers_mut().insert(
                header::CONTENT_LENGTH,
                HeaderValue::from_str(&size.to_string())
                    .map_err(|error| ProjectError::InternalError(error.to_string()))?,
            );
            response.headers_mut().insert(
                header::ETAG,
                HeaderValue::from_str(&format!("\"{sha256}\""))
                    .map_err(|error| ProjectError::InternalError(error.to_string()))?,
            );
            return Ok(response);
        }
        ObjectStore::Fs(_) => {
            // New uploads land at {iso_storage_key}/full.iso; installs that
            // predate the upload pipeline kept the built CD as game.iso.
            let base = state.project_demo_config.dir.join(storage_key);
            let path = if base.join("full.iso").is_file() {
                base.join("full.iso")
            } else {
                base.join("game.iso")
            };
            let file = tokio::fs::File::open(path)
                .await
                .map_err(|_| ProjectError::ProjectNotFound)?;
            let size = file.metadata().await?.len();
            let mut response =
                Response::new(axum::body::Body::from_stream(ReaderStream::new(file)));
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/octet-stream"),
            );
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            );
            response.headers_mut().insert(
                header::CONTENT_LENGTH,
                HeaderValue::from_str(&size.to_string())
                    .map_err(|error| ProjectError::InternalError(error.to_string()))?,
            );
            response.headers_mut().insert(
                header::ETAG,
                HeaderValue::from_str(&format!("\"{sha256}\""))
                    .map_err(|error| ProjectError::InternalError(error.to_string()))?,
            );
            return Ok(response);
        }
    }
}

fn zstd_compress(data: &[u8]) -> Result<Vec<u8>, ProjectError> {
    let mut encoder = zstd::stream::write::Encoder::new(Vec::new(), 19)
        .map_err(|e| ProjectError::InternalError(format!("zstd encode start: {e}")))?;
    std::io::Write::write_all(&mut encoder, data)
        .map_err(|e| ProjectError::InternalError(format!("zstd encode: {e}")))?;
    encoder
        .finish()
        .map_err(|e| ProjectError::InternalError(format!("zstd encode finish: {e}")))
}

#[allow(dead_code)]
fn zstd_decode(data: &[u8]) -> Result<Vec<u8>, ProjectError> {
    use std::io::Read;
    let mut output = Vec::with_capacity(V86_SAVE_FLOPPY_BYTES);
    zstd::stream::read::Decoder::new(data)
        .map_err(|e| ProjectError::InternalError(format!("zstd decode start: {e}")))?
        .read_to_end(&mut output)
        .map_err(|e| ProjectError::InternalError(format!("zstd decode: {e}")))?;
    Ok(output)
}

fn save_rate_limit_key(user_id: i64, game_id: i64) -> String {
    format!("{user_id}:{game_id}")
}

fn save_rate_limited(key: &str) -> bool {
    let now = Utc::now().timestamp_millis();
    static MAP: OnceLock<Mutex<HashMap<String, i64>>> = OnceLock::new();
    let map = MAP.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = map.lock().unwrap();
    if let Some(&last) = map.get(key)
        && now - last < V86_SAVE_RATE_LIMIT_MS as i64
    {
        return true;
    }
    map.insert(key.to_string(), now);
    false
}

async fn save_game_id(
    state: &AppState,
    slug: &str,
) -> Result<i64, ProjectError> {
    sqlx::query_scalar(
        r#"SELECT gm.id FROM games gm
           JOIN posts ON posts.id = gm.post_id
           WHERE posts.slug = ? AND posts.status = 'published' AND gm.launcher_type = 'v86'"#,
    )
    .bind(slug)
    .fetch_optional(&state.project_service.pool)
    .await?
    .ok_or(ProjectError::ProjectNotFound)
}

pub async fn get_game_save(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    AxumPath(slug): AxumPath<String>,
) -> Result<Response, ProjectError> {
    let Some(claims) = opt_claims else {
        return Err(ProjectError::ProjectNotFound);
    };
    let user_id = user_id(&claims)?;
    let game_id = save_game_id(&state, &slug).await?;
    let row = sqlx::query(
        "SELECT storage_key, size_bytes FROM game_v86_saves WHERE game_id = ? AND user_id = ?",
    )
    .bind(game_id)
    .bind(user_id)
    .fetch_optional(&state.project_service.pool)
    .await?;
    let (storage_key, _size) = match row {
        Some(row) => (row.get::<String, _>("storage_key"), row.get::<i64, _>("size_bytes")),
        None => return Err(ProjectError::SaveNotFound),
    };
    match &state.storage {
        ObjectStore::R2(_) => {
            streamed_object(
                &state.storage,
                &storage_key,
                "application/octet-stream",
                "no-store",
            )
            .await
        }
        ObjectStore::Fs(_) => {
            streamed_fs_file(
                state.project_demo_config.dir.join(storage_key),
                "application/octet-stream",
                "no-store",
            )
            .await
        }
    }
}

pub async fn put_game_save(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    AxumPath(slug): AxumPath<String>,
    bytes: Bytes,
) -> Result<StatusCode, ProjectError> {
    let user_id = user_id(&opt_claims.ok_or(ProjectError::Forbidden)?)?;
    let game_id = save_game_id(&state, &slug).await?;
    if bytes.is_empty() || bytes.len() > V86_SAVE_MAX_UPLOAD_BYTES {
        return Err(ProjectError::InvalidDemo(
            "The save image exceeds the allowed size.".to_string(),
        ));
    }
    let key = save_rate_limit_key(user_id, game_id);
    if save_rate_limited(&key) {
        return Err(ProjectError::Conflict(
            "Please wait before saving again.".to_string(),
        ));
    }
    // Level-19 zstd over up to 2 MB is seconds of CPU; run it with the hash
    // on the blocking pool.
    let (compressed, sha) = tokio::task::spawn_blocking(move || -> Result<_, ProjectError> {
        let compressed = zstd_compress(&bytes)?;
        let sha = hex::encode(Sha256::digest(&compressed));
        Ok((compressed, sha))
    })
    .await
    .map_err(|e| ProjectError::InternalError(e.to_string()))??;
    let storage_key = format!("v86/saves/{user_id}/{game_id}/save.zst");
    let size_bytes = compressed.len();
    state
        .storage
        .put_object_bytes(&storage_key, compressed)
        .await
        .map_err(storage_error)?;
    sqlx::query(
        r#"INSERT INTO game_v86_saves (game_id, user_id, storage_key, size_bytes, sha256)
           VALUES (?, ?, ?, ?, ?)
           ON CONFLICT(game_id, user_id) DO UPDATE SET
             storage_key = excluded.storage_key,
             size_bytes = excluded.size_bytes,
             sha256 = excluded.sha256,
             updated_at = CURRENT_TIMESTAMP"#,
    )
    .bind(game_id)
    .bind(user_id)
    .bind(&storage_key)
    .bind(size_bytes as i64)
    .bind(&sha)
    .execute(&state.project_service.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_game_save(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    AxumPath(slug): AxumPath<String>,
) -> Result<StatusCode, ProjectError> {
    let user_id = user_id(&opt_claims.ok_or(ProjectError::Forbidden)?)?;
    let game_id = save_game_id(&state, &slug).await?;
    let row = sqlx::query(
        "SELECT storage_key FROM game_v86_saves WHERE game_id = ? AND user_id = ?",
    )
    .bind(game_id)
    .bind(user_id)
    .fetch_optional(&state.project_service.pool)
    .await?;
    if let Some(row) = row {
        let storage_key: String = row.get("storage_key");
        let _ = state.storage.delete_object(&storage_key).await;
        sqlx::query("DELETE FROM game_v86_saves WHERE game_id = ? AND user_id = ?")
            .bind(game_id)
            .bind(user_id)
            .execute(&state.project_service.pool)
            .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_is_exact_but_rejects_nul_and_oversize() {
        let text = "exe=Doraemon.exe\r\nargs=-m -5";
        assert_eq!(
            validate_manifest(text).ok().unwrap(),
            hex::encode(Sha256::digest(text.as_bytes()))
        );
        assert!(validate_manifest("exe=a.exe\0args=-m").is_err());
        assert!(validate_manifest(&"x".repeat(MANIFEST_MAX_BYTES + 1)).is_err());
    }

    #[test]
    fn immutable_parts_use_v86_range_names() {
        use std::io::Read;
        let root = std::env::temp_dir().join(format!("v86-parts-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("disk.img");
        fs::write(&source, b"abcdefghij").unwrap();
        let parts = root.join("parts");
        assert_eq!(split_asset(&source, &parts, 4, "img.zst", None, 6).ok().unwrap(), 3);
        let read_decompressed = |name| {
            let compressed = fs::read(parts.join(name)).unwrap();
            let mut decoder = zstd::stream::read::Decoder::new(&compressed[..]).unwrap();
            let mut buf = Vec::new();
            decoder.read_to_end(&mut buf).unwrap();
            buf
        };
        assert_eq!(read_decompressed("0-4.img.zst"), b"abcd");
        assert_eq!(read_decompressed("4-8.img.zst"), b"efgh");
        assert_eq!(read_decompressed("8-12.img.zst"), b"ij\0\0");
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn save_files_parse_and_validate() {
        assert_eq!(
            save_files_from_manifest(
                "exe=a.exe\nsave_paths=Save0001.dat; A/save0001.dat; backup/what.bak"
            )
            .ok()
            .unwrap(),
            vec![
                "Save0001.dat".to_string(),
                r"A\save0001.dat".to_string(),
                r"backup\what.bak".to_string(),
            ]
        );
        assert_eq!(
            save_files_from_manifest("exe=a.exe\nsaves=Save0001.dat,save0001.dat")
                .ok()
                .unwrap(),
            vec!["Save0001.dat".to_string()]
        );
        assert_eq!(
            save_files_from_manifest("exe=a.exe\nsaves=").ok().unwrap().len(),
            0
        );
        for bad in [
            "save_paths=/abs.dat",
            "save_paths=a\\b\\",
            "save_paths=a//b.dat",
            "save_paths=../x.dat",
            "save_paths=./x.dat",
            "save_paths=a\\b\\..\\c.dat",
            "save_paths=a=b.dat",
            "save_paths=a:b.dat",
            "save_paths=*.dat",
            "save_paths=?.dat",
            "save_paths=a b.dat",
        ] {
            assert!(
                save_files_from_manifest(&format!("exe=a.exe\n{bad}")).is_err(),
                "{bad} should be rejected"
            );
        }
    }

    #[test]
    fn mouse_config_defaults_and_parses() {
        let default = parse_mouse_config("exe=a.exe").ok().unwrap();
        assert!(!default.revert_mouse_y);
        assert_eq!(default.mouse_speed, 1.0);

        let inverted = parse_mouse_config("exe=a.exe\nrevert_mouse_y=1").ok().unwrap();
        assert!(inverted.revert_mouse_y);
        assert_eq!(inverted.mouse_speed, 1.0);

        let fast = parse_mouse_config("exe=a.exe\nmouse_speed=2.5").ok().unwrap();
        assert!(!fast.revert_mouse_y);
        assert_eq!(fast.mouse_speed, 2.5);

        let both = parse_mouse_config("exe=a.exe\nrevert_mouse_y=true\nmouse_speed=0.5")
            .ok()
            .unwrap();
        assert!(both.revert_mouse_y);
        assert_eq!(both.mouse_speed, 0.5);

        for bad in ["revert_mouse_y=banana", "mouse_speed=nope", "mouse_speed=-1", "mouse_speed=0"] {
            assert!(parse_mouse_config(&format!("exe=a.exe\n{bad}")).is_err(), "{bad}");
        }
    }
}
