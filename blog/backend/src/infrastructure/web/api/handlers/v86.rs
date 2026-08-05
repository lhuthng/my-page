use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex, OnceLock},
};

use axum::{
    Extension, Json,
    body::Bytes,
    extract::{Path as AxumPath, Query, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Row, Sqlite, Transaction};
use tokio_util::io::ReaderStream;
use uuid::Uuid;
use zip::ZipArchive;

use crate::{
    domain::{entities::secret::Claims, errors::project::ProjectError},
    infrastructure::{
        storage::r2::R2Client,
        web::server::AppState,
    },
};

const MANIFEST_MAX_BYTES: usize = 64 * 1024;

/// Capacity of the 1.44 MB FAT12 floppy used as the save transport box.
const V86_SAVE_FLOPPY_BYTES: usize = 1474560;
/// Hard cap for a floppy save upload so the body limit stays bounded.
const V86_SAVE_MAX_UPLOAD_BYTES: usize = 2 * 1024 * 1024;
/// Per-user cooldown between cloud saves (matches the client).
const V86_SAVE_RATE_LIMIT_MS: u64 = 30_000;

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
}

#[derive(Deserialize)]
pub struct StartGameUploadRequest {
    pub source_project_id: Option<i64>,
    pub system_version_id: i64,
    pub expected_artifact_revision: i64,
    pub manifest: String,
    pub file_name: Option<String>,
    pub size_bytes: Option<u64>,
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
    pub save_supported: bool,
    pub save_max_bytes: u64,
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

fn require_r2(state: &AppState) -> Result<R2Client, ProjectError> {
    state.r2.clone().ok_or_else(|| {
        ProjectError::InternalError(
            "R2 storage is not configured; cannot process v86 uploads.".to_string(),
        )
    })
}

fn r2_error(error: crate::infrastructure::storage::r2::R2Error) -> ProjectError {
    ProjectError::InternalError(error.to_string())
}

fn transient_r2_key(kind: &str, upload_id: &str, extension: &str) -> String {
    format!("v86/tmp/{kind}/{upload_id}.{extension}")
}

fn parse_r2_part_etags(etags: Option<&str>) -> Vec<(i32, String)> {
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

fn append_r2_part_etag(existing: Option<&str>, etag: &str) -> String {
    let mut etags: Vec<String> = match existing {
        Some(text) if !text.is_empty() => serde_json::from_str(text).unwrap_or_default(),
        _ => Vec::new(),
    };
    etags.push(etag.to_string());
    serde_json::to_string(&etags).unwrap_or_else(|_| "[]".to_string())
}

fn temp_upload_path(state: &AppState, kind: &str, upload_id: &str) -> PathBuf {
    state
        .project_demo_config
        .dir
        .join("v86")
        .join("tmp")
        .join(kind)
        .join(format!("{upload_id}.upload"))
}

fn sha256_file(path: &Path) -> Result<(u64, String), ProjectError> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut size = 0_u64;
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
        size += read as u64;
    }
    Ok((size, hex::encode(hash.finalize())))
}

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

fn is_reserved_windows_name(component: &str) -> bool {
    let stem = component
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end_matches(' ')
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit()
            && stem.as_bytes()[3] != b'0')
}

async fn rollback_system_version(
    pool: &sqlx::SqlitePool,
    upload_id: &str,
    system_id: i64,
    version_number: i64,
    is_new_system: bool,
) -> Result<(), sqlx::Error> {
    if is_new_system {
        sqlx::query("DELETE FROM v86_systems WHERE id = ?")
            .bind(system_id)
            .execute(pool)
            .await?;
    } else {
        sqlx::query(
            "UPDATE v86_systems SET current_version = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND current_version = ?",
        )
        .bind(version_number - 1)
        .bind(system_id)
        .bind(version_number)
        .execute(pool)
        .await?;
    }
    sqlx::query(
        "UPDATE v86_system_upload_sessions SET status = 'failed', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(upload_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Skips macOS-created junk that carries no game data: the `__MACOSX/`
/// resource-fork tree and `.DS_Store` metadata files.
fn is_macos_junk(normalized: &str) -> bool {
    let first = normalized.split('/').next().unwrap_or("");
    if first.eq_ignore_ascii_case("__MACOSX") {
        return true;
    }
    let name = normalized.rsplit('/').next().unwrap_or("");
    name.eq_ignore_ascii_case(".DS_Store")
}

fn validate_and_extract_game_zip(
    zip_path: &Path,
    destination: &Path,
    max_files: usize,
    max_extracted_size: u64,
) -> Result<(), ProjectError> {
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| ProjectError::UploadFailed(format!("Invalid game ZIP: {e}")))?;
    if archive.len() > max_files {
        return Err(ProjectError::InvalidDemo(format!(
            "The game ZIP exceeds the {max_files} file limit."
        )));
    }
    fs::create_dir_all(destination)?;
    let mut seen = HashSet::new();
    let mut expanded = 0_u64;
    let mut extracted_files = 0_u64;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|e| ProjectError::UploadFailed(e.to_string()))?;
        let normalized = entry.name().replace('\\', "/");
        if normalized.contains('\0') || normalized.starts_with('/') {
            return Err(ProjectError::InvalidDemo(
                "The game ZIP contains an unsafe path.".to_string(),
            ));
        }
        if is_macos_junk(&normalized) {
            continue;
        }
        let relative = Path::new(&normalized);
        if relative.components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            return Err(ProjectError::InvalidDemo(
                "The game ZIP contains path traversal.".to_string(),
            ));
        }
        if let Some(mode) = entry.unix_mode()
            && mode & 0o170000 == 0o120000
        {
            return Err(ProjectError::InvalidDemo(
                "Symbolic links are not accepted in game ZIPs.".to_string(),
            ));
        }
        for component in relative.components() {
            if let Component::Normal(value) = component {
                let value = value.to_string_lossy();
                if is_reserved_windows_name(&value)
                    || value.ends_with(' ')
                    || value.ends_with('.')
                    || value.chars().any(|c| c.is_control())
                    || value.chars().any(|c| "<>:\"|?*".contains(c))
                {
                    return Err(ProjectError::InvalidDemo(
                        "The game ZIP contains a Windows-incompatible path.".to_string(),
                    ));
                }
            }
        }
        if format!(r"D:\{}", normalized).len() >= 260 {
            return Err(ProjectError::InvalidDemo(
                "The game ZIP contains a path longer than Windows 95 supports.".to_string(),
            ));
        }
        let key = normalized.to_lowercase();
        if !seen.insert(key) {
            return Err(ProjectError::InvalidDemo(
                "The game ZIP contains case-insensitive duplicate paths.".to_string(),
            ));
        }
        let lower = normalized.to_ascii_lowercase();
        if [".img", ".iso", ".jsdos", ".7z", ".rar"]
            .iter()
            .any(|suffix| lower.ends_with(suffix))
        {
            return Err(ProjectError::InvalidDemo(
                "Nested disk images and archives are not accepted.".to_string(),
            ));
        }
        expanded = expanded.saturating_add(entry.size());
        if expanded > max_extracted_size {
            return Err(ProjectError::InvalidDemo(
                "The expanded game ZIP exceeds the configured limit.".to_string(),
            ));
        }
        if entry.compressed_size() > 0
            && entry.size() > 10 * 1024 * 1024
            && entry.size() / entry.compressed_size() > 200
        {
            return Err(ProjectError::InvalidDemo(
                "The game ZIP has an unsafe compression ratio.".to_string(),
            ));
        }

        let output_path = destination.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&output_path)?;
        } else {
            extracted_files += 1;
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut output = File::create(output_path)?;
            std::io::copy(&mut entry, &mut output)?;
        }
    }
    if extracted_files == 0 {
        return Err(ProjectError::InvalidDemo(
            "The game ZIP contains no game files.".to_string(),
        ));
    }
    Ok(())
}

fn unwrap_single_top_level_dir(game_dir: &Path) -> Result<(), ProjectError> {
    loop {
        let entries: Vec<PathBuf> = fs::read_dir(game_dir)?.flatten().map(|e| e.path()).collect();
        if entries.len() != 1 {
            return Ok(());
        }
        let only = &entries[0];
        if !only.is_dir() {
            return Ok(());
        }
        let only_name = only
            .file_name()
            .ok_or_else(|| {
                ProjectError::InternalError(
                    "Could not unwrap an extracted game folder.".to_string(),
                )
            })?
            .to_owned();
        let inner = fs::read_dir(only)?
            .flatten()
            .map(|e| e.path())
            .collect::<Vec<_>>();
        let mut same_named_child = None;
        for item in inner {
            let name = item
                .file_name()
                .ok_or_else(|| {
                    ProjectError::InternalError(
                        "Could not unwrap an extracted game folder.".to_string(),
                    )
                })?;
            if name == only_name {
                // A child directory sharing the wrapper's name (Game/Game): it
                // cannot be renamed onto the wrapper it lives in, so bubble it
                // up once the wrapper is gone.
                same_named_child = Some(item);
                continue;
            }
            let target = game_dir.join(name);
            if target.exists() {
                return Err(ProjectError::InvalidDemo(
                    "The game ZIP contains conflicting top-level paths.".to_string(),
                ));
            }
            fs::rename(&item, &target)?;
        }
        if let Some(child) = same_named_child {
            let temp = game_dir.join(format!(".__unwrap_{}", Uuid::new_v4()));
            fs::rename(&child, &temp)?;
            fs::remove_dir(only)?;
            fs::rename(&temp, game_dir.join(only_name))?;
        } else {
            fs::remove_dir(only)?;
        }
    }
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
            "The Windows 95 manifest contains an unsafe executable path.".to_string(),
        ));
    }
    Ok(normalized)
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
                "Invalid Windows 95 save entry '{entry}': {reason}."
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

fn windows95_launcher_config(manifest: &str) -> Result<String, ProjectError> {
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

    let requested = fields
        .get("exe")
        .or_else(|| fields.get("executable"))
        .ok_or_else(|| {
            ProjectError::InvalidDemo(
                "The Windows 95 manifest must contain exe=<game executable>.".to_string(),
            )
        })?;
    let requested = normalize_manifest_path(requested)?;
    if !requested.to_ascii_lowercase().ends_with(".exe") {
        return Err(ProjectError::InvalidDemo(
            "The Windows 95 manifest executable must be an .exe file.".to_string(),
        ));
    }

    // The executable path is taken verbatim from the manifest: the game tree is
    // baked into the disk at full-build time, so manifest-only edits do not need
    // to re-extract the ZIP to resolve or verify the path.
    let relative_windows = requested.replace('/', "\\");
    let executable = format!(r"D:\{relative_windows}");
    if executable.len() >= 260 {
        return Err(ProjectError::InvalidDemo(
            "The resolved Windows 95 executable path is too long.".to_string(),
        ));
    }
    let working_directory = match requested.rfind('/') {
        Some(index) if index > 0 => format!(r"D:\{}", requested[..index].replace('/', "\\")),
        _ => r"D:\".to_string(),
    };
    let arguments = fields
        .get("args")
        .or_else(|| fields.get("arguments"))
        .map(String::as_str)
        .unwrap_or("");
    let delay_ms = fields.get("delay_ms").map(String::as_str).unwrap_or("1000");
    if !delay_ms.chars().all(|character| character.is_ascii_digit()) {
        return Err(ProjectError::InvalidDemo(
            "The Windows 95 manifest delay_ms must be a number.".to_string(),
        ));
    }

    let save_files = save_files_from_manifest(manifest)?;
    let mut config = String::new();
    config.push_str(&format!(
        "[game]\r\nexecutable={executable}\r\nworking_directory={working_directory}\r\narguments={arguments}\r\ndelay_ms={delay_ms}\r\n"
    ));
    if !save_files.is_empty() {
        config.push_str("[saves]\r\n");
        for file in &save_files {
            config.push_str(&format!("file={file}\r\n"));
        }
    }
    Ok(config)
}

/// Resolves an mtools binary name through the optional tool prefix dir.
fn mtool(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        let base = prefix.trim_end_matches('/');
        let name = name.trim_start_matches('/');
        format!("{base}/{name}")
    }
}

/// Returns the mtools command, pushing `prefix/name` args so callers just pass
/// their `-i ...` flags. The command is resolved through `mtools_bin`.
fn run_mtool(mtools_bin: &str, name: &str, args: &[&str]) -> Result<(), ProjectError> {
    let binary = mtool(mtools_bin, name);
    let output = Command::new(&binary)
        .args(args)
        .output()
        .map_err(|e| {
            ProjectError::InternalError(format!("Could not start mtools {name} ({binary}): {e}"))
        })?;
    if !output.status.success() {
        return Err(ProjectError::UploadFailed(format!(
            "mtools {name} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

/// Computes a total byte size (rounded up to a whole sector) for a FAT image
/// that comfortably holds `payload_bytes`, leaving `slack_fraction` of free
/// space so the game has room for save files without repartitioning.
fn fat_image_size(payload_bytes: u64) -> u64 {
    const BLOCK: u64 = 512;
    // The game disk is capped well below 2GiB so it stays FAT16-compatible.
    let slack = payload_bytes / 2;
    let raw = payload_bytes + slack + 8 * 1024 * 1024;
    let raw = raw.min(1536 * 1024 * 1024);
    raw.div_ceil(BLOCK) * BLOCK
}

/// Extracts (skipping macOS junk) and unwraps the game ZIP into `build_dir`,
/// returning the game tree path and its total byte size.
fn prepare_game_tree(
    build_dir: &Path,
    zip_path: &Path,
    max_files: usize,
    max_extracted_size: u64,
) -> Result<(PathBuf, u64), ProjectError> {
    let game_dir = build_dir.join("game");
    validate_and_extract_game_zip(zip_path, &game_dir, max_files, max_extracted_size)?;
    // A ZIP that wraps its payload in a single top-level folder is common; drop
    // the wrapper so the game lands at the drive root instead of a doubled path.
    unwrap_single_top_level_dir(&game_dir)?;
    let extracted_bytes = dir_size(&game_dir);
    Ok((game_dir, extracted_bytes))
}

/// Writes a classic MBR with one bootable FAT16 (0x06) partition starting at
/// sector 63, mirroring the base Win95 disk layout that v86 mounts reliably.
fn write_mbr_partition(image_path: &Path, total_sectors: u64) -> Result<(), ProjectError> {
    use std::io::{Seek, SeekFrom, Write};
    let part_sectors = total_sectors
        .checked_sub(63)
        .ok_or_else(|| ProjectError::UploadFailed("Game disk is too small.".to_string()))?;
    let part_sectors: u32 = part_sectors
        .try_into()
        .map_err(|_| ProjectError::UploadFailed("Game disk partition is too large.".to_string()))?;
    // End CHS from the whole-disk LBA with 255 heads / 63 sectors per track.
    let end_lba = 63 + part_sectors - 1;
    let heads: u32 = 255;
    let sectors_per_track: u32 = 63;
    let end_cyl = end_lba / (heads * sectors_per_track);
    let remainder = end_lba % (heads * sectors_per_track);
    let end_head = (remainder / sectors_per_track) as u8;
    let end_sector = (remainder % sectors_per_track + 1) as u8;
    let end_cyl = end_cyl as u8;
    let mut entry = [0u8; 16];
    entry[0] = 0x80; // bootable
    entry[1] = 0; // start CHS head
    entry[2] = 1; // start CHS sector
    entry[3] = 0; // start CHS cylinder
    entry[4] = 0x06; // FAT16
    entry[5] = end_head;
    entry[6] = end_sector;
    entry[7] = end_cyl;
    entry[8..12].copy_from_slice(&63u32.to_le_bytes());
    entry[12..16].copy_from_slice(&part_sectors.to_le_bytes());
    let mut file = fs::OpenOptions::new().write(true).open(image_path)?;
    file.seek(SeekFrom::Start(446))?;
    file.write_all(&entry)?;
    file.seek(SeekFrom::Start(510))?;
    file.write_all(&[0x55, 0xAA])?;
    file.flush()?;
    Ok(())
}

/// Builds the partitioned FAT16 game disk (D:) from an extracted game tree.
fn build_game_disk(
    mtools_bin: &str,
    image_path: &Path,
    game_dir: &Path,
    extracted_bytes: u64,
) -> Result<(), ProjectError> {
    let total_bytes = fat_image_size(extracted_bytes);
    let total_sectors = total_bytes / 512;
    // Create a partitioned FAT16 disk image so classic Windows 95 mounts the
    // whole game drive reliably (a raw superfloppy with mformat's phantom MBR
    // entry can show up as inaccessible). This mirrors the base disk's proven
    // MBR + FAT16 partition layout: C: (base) → D: (game) → E: (cdrom).
    {
        let image = fs::File::create(image_path)?;
        image.set_len(total_bytes)?;
    }
    write_mbr_partition(image_path, total_sectors)?;
    // mtools `@@N` offsets are in bytes, so partition sector 63 is byte 32256.
    let partition_arg = format!("{}@@{}", image_path.to_str().unwrap(), 63 * 512);
    run_mtool(mtools_bin, "mformat", &["-i", &partition_arg, "::"])?;
    // Copy each top-level entry by name so the unwrapped game lands at the
    // drive root (no GAME folder) instead of mcopy nesting a source dir.
    let mut sources: Vec<PathBuf> = fs::read_dir(game_dir)?.flatten().map(|e| e.path()).collect();
    sources.sort();
    if !sources.is_empty() {
        let mut args = vec!["-i", &partition_arg, "-s", "-o"];
        args.extend(sources.iter().map(|path| path.to_str().unwrap()));
        args.push("::/");
        run_mtool(mtools_bin, "mcopy", &args)?;
    }
    Ok(())
}

/// Builds the tiny autorun CD (E:). The launcher + config live here (read-only),
/// so the shared Win95 base never needs an auto-run-on-fixed-drive hack.
fn build_game_cdrom(
    xorriso_bin: &str,
    assets_dir: &Path,
    manifest: &str,
    disc_dir: &Path,
    cdrom_path: &Path,
) -> Result<(), ProjectError> {
    fs::create_dir_all(disc_dir)?;
    let launcher = assets_dir
        .join("v86")
        .join("windows95")
        .join("LAUNCHER.EXE");
    if !launcher.is_file() {
        return Err(ProjectError::InternalError(format!(
            "The Windows 95 v86 launcher is missing at {}.",
            launcher.display()
        )));
    }
    fs::copy(launcher, disc_dir.join("LAUNCHER.EXE"))?;
    fs::write(
        disc_dir.join("AUTORUN.INF"),
        b"[autorun]\r\nopen=LAUNCHER.EXE\r\n",
    )?;
    fs::write(
        disc_dir.join("V86GAME.INI"),
        windows95_launcher_config(manifest)?.as_bytes(),
    )?;
    fs::write(disc_dir.join("V86GAME.MANIFEST"), manifest.as_bytes())?;

    let output = Command::new(xorriso_bin)
        .args(["-as", "mkisofs", "-J", "-V", "V86GAME", "-o"])
        .arg(cdrom_path)
        .arg(disc_dir)
        .output()
        .map_err(|e| {
            ProjectError::InternalError(format!("Could not start xorriso ({xorriso_bin}): {e}"))
        })?;
    if !output.status.success() {
        return Err(ProjectError::UploadFailed(format!(
            "ISO generation failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

/// Builds a writable FAT16 game disk (partitioned, mounted as D: by the guest)
/// plus a tiny read-only autorun CD (mounted as E:) that launches the game
/// launcher. Game files sit at the D: drive root with no wrapper folder.
/// Returns (disk_image_path, cdrom_iso_path, extracted_game_bytes).
#[allow(clippy::too_many_arguments)]
fn build_windows95_disk(
    state_dir: &Path,
    assets_dir: &Path,
    xorriso_bin: &str,
    mtools_bin: &str,
    upload_id: &str,
    zip_path: &Path,
    manifest: &str,
    max_files: usize,
    max_extracted_size: u64,
) -> Result<(PathBuf, PathBuf, u64), ProjectError> {
    let build_dir = state_dir
        .join("v86")
        .join("tmp")
        .join("build")
        .join(upload_id);
    if build_dir.exists() {
        fs::remove_dir_all(&build_dir)?;
    }
    let (game_dir, extracted_bytes) =
        prepare_game_tree(&build_dir, zip_path, max_files, max_extracted_size)?;
    let image_path = build_dir.join("game.img");
    build_game_disk(mtools_bin, &image_path, &game_dir, extracted_bytes)?;
    let cdrom_path = build_dir.join("boot.iso");
    build_game_cdrom(
        xorriso_bin,
        assets_dir,
        manifest,
        &build_dir.join("disc"),
        &cdrom_path,
    )?;
    Ok((image_path, cdrom_path, extracted_bytes))
}

/// Rebuilds only the autorun CD when the manifest changed. The game ZIP is not
/// needed: the launcher, config and manifest are static/small and the disk is
/// reused as-is.
fn build_windows95_cdrom_only(
    state_dir: &Path,
    assets_dir: &Path,
    xorriso_bin: &str,
    upload_id: &str,
    manifest: &str,
) -> Result<PathBuf, ProjectError> {
    let build_dir = state_dir
        .join("v86")
        .join("tmp")
        .join("build")
        .join(upload_id);
    if build_dir.exists() {
        fs::remove_dir_all(&build_dir)?;
    }
    let cdrom_path = build_dir.join("boot.iso");
    build_game_cdrom(
        xorriso_bin,
        assets_dir,
        manifest,
        &build_dir.join("disc"),
        &cdrom_path,
    )?;
    Ok(cdrom_path)
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

async fn require_project_owner(
    state: &AppState,
    project_id: i64,
    uploader_id: i64,
) -> Result<(), ProjectError> {
    let owner: Option<i64> = sqlx::query_scalar(
        "SELECT posts.user_id FROM projects JOIN posts ON posts.id = projects.post_id WHERE projects.id = ?",
    )
    .bind(project_id)
    .fetch_optional(&state.project_service.pool)
    .await?;
    if owner != Some(uploader_id) {
        return Err(ProjectError::Forbidden);
    }
    Ok(())
}

pub async fn list_systems(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<V86SystemResponse>>, ProjectError> {
    let rows = sqlx::query(
        r#"SELECT s.id, s.name, s.platform_key, s.is_active, s.is_default,
                  s.current_version,
                  COUNT(g.project_id) AS project_count,
                  SUM(CASE WHEN posts.status = 'published' THEN 1 ELSE 0 END) AS published_count
           FROM v86_systems s
           LEFT JOIN v86_system_versions v ON v.system_id = s.id
           LEFT JOIN project_v86_games g ON g.system_version_id = v.id
           LEFT JOIN projects p ON p.id = g.project_id
           LEFT JOIN posts ON posts.id = p.post_id
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
) -> Result<Json<StartUploadResponse>, ProjectError> {
    let uploader_id = user_id(&claims)?;
    if request.platform_key != "windows95" {
        return Err(ProjectError::InvalidDemo(
            "Only the windows95 v86 platform is currently supported.".to_string(),
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
    let upload_id = Uuid::new_v4().to_string();
    let r2 = require_r2(&state)?;
    let transient_key = transient_r2_key("systems", &upload_id, "img");
    let multipart = r2
        .create_multipart(&transient_key)
        .await
        .map_err(r2_error)?;
    let r2_upload_id = multipart
        .upload_id()
        .ok_or_else(|| {
            ProjectError::InternalError("R2 did not return a multipart upload id.".to_string())
        })?
        .to_string();
    let expires_at =
        Utc::now() + Duration::hours(state.project_demo_config.upload_session_ttl_hours as i64);
    sqlx::query(
        r#"INSERT INTO v86_system_upload_sessions
           (id, uploader_id, system_id, name, platform_key, expected_current_version,
            original_file_name, expected_size_bytes, upload_chunk_size_bytes,
            temp_storage_key, r2_upload_id, expires_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&upload_id)
    .bind(uploader_id)
    .bind(request.system_id)
    .bind(name)
    .bind(&request.platform_key)
    .bind(request.expected_current_version.unwrap_or(0))
    .bind(&request.file_name)
    .bind(request.size_bytes as i64)
    .bind(state.project_demo_config.v86_upload_chunk_size as i64)
    .bind(&transient_key)
    .bind(&r2_upload_id)
    .bind(expires_at.to_rfc3339())
    .execute(&state.project_service.pool)
    .await
    .map_err(|error| {
        let r2 = r2.clone();
        tokio::spawn(async move {
            let _ = r2.abort_multipart(&transient_key, &r2_upload_id).await;
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

    let r2 = require_r2(state)?;
    let r2_upload_id = row.get::<Option<String>, _>("r2_upload_id").ok_or_else(|| {
        ProjectError::InternalError("Upload session is missing its R2 multipart id.".to_string())
    })?;
    let etag = r2
        .upload_part(&temp_key, &r2_upload_id, (chunk_index as i32) + 1, bytes.to_vec())
        .await
        .map_err(r2_error)?;

    let new_received = received + bytes.len() as i64;
    let new_next = next + 1;
    let new_etags = append_r2_part_etag(row.get::<Option<String>, _>("r2_part_etags").as_deref(), &etag);
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
        let _ = r2.abort_multipart(&temp_key, &r2_upload_id).await;
        let failed = format!(
            "UPDATE {table} SET status = 'failed', error_message = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
        );
        sqlx::query(&failed)
            .bind("R2 chunk relay conflict")
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

pub async fn append_system_chunk(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath((upload_id, chunk_index)): AxumPath<(String, u64)>,
    bytes: Bytes,
) -> Result<Json<ChunkUploadResponse>, ProjectError> {
    Ok(Json(
        append_upload_chunk(
            &state,
            "v86_system_upload_sessions",
            &upload_id,
            user_id(&claims)?,
            chunk_index,
            bytes,
        )
        .await?,
    ))
}

pub async fn abort_system_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(upload_id): AxumPath<String>,
) -> Result<StatusCode, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let row = sqlx::query(
        "SELECT temp_storage_key, r2_upload_id, status FROM v86_system_upload_sessions WHERE id = ? AND uploader_id = ?",
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
    if let Some(r2) = &state.r2 {
        let temp_key: String = row.get("temp_storage_key");
        if let Some(r2_upload_id) = row.get::<Option<String>, _>("r2_upload_id") {
            let _ = r2.abort_multipart(&temp_key, &r2_upload_id).await;
        }
        let _ = r2.delete_object(&temp_key).await;
    }
    tokio::fs::remove_file(temp_upload_path(&state, "systems", &upload_id))
        .await
        .ok();
    Ok(StatusCode::NO_CONTENT)
}

pub async fn complete_system_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(upload_id): AxumPath<String>,
) -> Result<StatusCode, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let row = sqlx::query(
        "SELECT system_id, name, platform_key, expected_current_version, original_file_name, expected_size_bytes, received_size_bytes, temp_storage_key, r2_upload_id, r2_part_etags, status, expires_at FROM v86_system_upload_sessions WHERE id = ? AND uploader_id = ?",
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
            "The base IMG upload is incomplete.".to_string(),
        ));
    }
    let r2 = require_r2(&state)?;
    let temp_key: String = row.get("temp_storage_key");
    let r2_upload_id = row.get::<Option<String>, _>("r2_upload_id").ok_or_else(|| {
        ProjectError::InternalError("Upload session is missing its R2 multipart id.".to_string())
    })?;
    let etags = parse_r2_part_etags(row.get::<Option<String>, _>("r2_part_etags").as_deref());
    r2.complete_multipart(&temp_key, &r2_upload_id, etags)
        .await
        .map_err(r2_error)?;
    let signature = r2
        .get_object_range(&temp_key, 0, 511)
        .await
        .map_err(r2_error)?;
    if signature.len() < 512 || signature[510..512] != [0x55, 0xaa] {
        let _ = r2.delete_object(&temp_key).await;
        sqlx::query(
            "UPDATE v86_system_upload_sessions SET status = 'failed', error_message = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind("The base IMG does not contain a valid boot-sector signature.")
        .bind(&upload_id)
        .execute(&state.project_service.pool)
        .await
        .ok();
        return Err(ProjectError::InvalidDemo(
            "The base IMG does not contain a valid boot-sector signature.".to_string(),
        ));
    }
    let system_id_opt: Option<i64> = row.get("system_id");
    let expected_version: i64 = row.get("expected_current_version");
    let name: String = row.get("name");
    let platform_key: String = row.get("platform_key");
    let original_file_name: String = row.get("original_file_name");
    let is_new_system = system_id_opt.is_none();

    // Guard: name must still be unique in case another upload was created after this session
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

    // Phase 1: Brief tx to reserve version slot and set current_version
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

    sqlx::query(
        "UPDATE v86_system_upload_sessions SET status = 'building', system_id = COALESCE(system_id, ?), updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(system_id)
    .bind(&upload_id)
    .execute(&state.project_service.pool)
    .await?;

    let progress = Arc::new(Mutex::new(ChunkProgress {
        upload_id: upload_id.clone(),
        kind: "system".to_string(),
        total_chunks: 0,
        completed_chunks: 0,
        message: "Preparing…".to_string(),
    }));
    chunk_progress_map()
        .lock()
        .unwrap()
        .insert(upload_id.clone(), progress.lock().unwrap().clone());

    // Spawn background task for Phases 2-3 (sha256, compression, R2 push, DB commit)
    let pool = state.project_service.pool.clone();
    let r2_c = r2.clone();
    let config_dir = state.project_demo_config.dir.clone();
    let chunk_size = state.project_demo_config.v86_download_chunk_size;
    let upload_id_c = upload_id.clone();
    let temp_key_c = temp_key.clone();
    let local_download = temp_upload_path(&state, "systems", &upload_id);
    let progress_c = progress.clone();
    let original_file_name_c = original_file_name.clone();

    tokio::spawn(async move {
        let result = process_system_upload(
            &pool,
            &r2_c,
            &config_dir,
            &upload_id_c,
            &temp_key_c,
            &local_download,
            chunk_size,
            progress_c,
            system_id,
            version_number,
            is_new_system,
            &original_file_name_c,
        )
        .await;
        if let Err(e) = result {
            tracing::error!("Background system upload failed for {upload_id_c}: {e}");
            let _ = rollback_system_version(
                &pool, &upload_id_c, system_id, version_number, is_new_system,
            )
            .await;
            chunk_progress_map().lock().unwrap().remove(&upload_id_c);
        }
    });

    Ok(StatusCode::ACCEPTED)
}

async fn push_dir_to_r2(r2: &R2Client, storage_key: &str, dir: &Path) -> Result<(), String> {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        names.push(entry.file_name().to_string_lossy().to_string());
    }
    names.sort();
    for name in names {
        let key = format!("{storage_key}/{name}");
        r2.put_object_from_file(&key, &dir.join(&name))
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn process_system_upload(
    pool: &sqlx::SqlitePool,
    r2: &R2Client,
    config_dir: &Path,
    upload_id: &str,
    transient_key: &str,
    local_download: &Path,
    chunk_size: u64,
    progress: Arc<Mutex<ChunkProgress>>,
    system_id: i64,
    version_number: i64,
    _is_new_system: bool,
    original_file_name: &str,
) -> Result<(), String> {
    if let Some(parent) = local_download.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| e.to_string())?;
    }
    r2.download_to_file(transient_key, local_download)
        .await
        .map_err(|e| e.to_string())?;
    let source = local_download.to_path_buf();

    // Phase 2a: Compute sha256 and file size (heavy I/O, offloaded)
    let (size, sha256) = tokio::task::spawn_blocking({
        let source = source.clone();
        move || sha256_file(&source)
    })
    .await
    .map_err(|e| format!("sha256 task panicked: {e}"))?
    .map_err(|e| e.to_string())?;

    let total_chunks = size.div_ceil(chunk_size as u64);
    {
        let mut p = progress.lock().unwrap();
        p.total_chunks = total_chunks;
        p.message = format!("Compressing chunk 0/{total_chunks}");
    }

    // Phase 2b: Compression into a transient local parts dir (offloaded)
    let parts_dir = config_dir
        .join("v86")
        .join("tmp")
        .join("systems")
        .join(format!("{upload_id}.parts"));
    let chunk_count = tokio::task::spawn_blocking({
        let source = source.clone();
        let parts = parts_dir.clone();
        let progress_for_compress = progress.clone();
        move || {
            split_asset(
                &source,
                &parts,
                chunk_size,
                "img.zst",
                Some(&*progress_for_compress),
                19,
            )
        }
    })
    .await
    .map_err(|e| format!("compression task panicked: {e}"))?
    .map_err(|e| e.to_string())?;

    {
        let mut p = progress.lock().unwrap();
        p.message = "Uploading to R2…".to_string();
    }

    // Reserve the version row first so we can key the parts by the version id.
    let version_id = {
        let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
        let placeholder = format!("v86/assets/systems/pending/{upload_id}");
        let result = sqlx::query(
            r#"INSERT INTO v86_system_versions
               (system_id, version_number, original_file_name, storage_key, size_bytes,
                sha256, chunk_size_bytes, chunk_count)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(system_id)
        .bind(version_number)
        .bind(original_file_name)
        .bind(&placeholder)
        .bind(size as i64)
        .bind(&sha256)
        .bind(chunk_size as i64)
        .bind(chunk_count as i64)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
        tx.commit().await.map_err(|e| e.to_string())?;
        result.last_insert_rowid()
    };

    let storage_key = format!("v86/assets/systems/{version_id}/{sha256}");
    if let Err(error) = push_dir_to_r2(r2, &storage_key, &parts_dir).await {
        let _ = r2.delete_prefix(&storage_key).await;
        sqlx::query("DELETE FROM v86_system_versions WHERE id = ?")
            .bind(version_id)
            .execute(pool)
            .await
            .ok();
        return Err(error);
    }

    // The transient upload object is owned by this session; delete it now.
    let _ = r2.delete_object(transient_key).await;
    let _ = tokio::fs::remove_file(&source).await;
    let _ = tokio::fs::remove_dir_all(&parts_dir).await;

    // Phase 3: Point the version row at its real R2 key and mark the session consumed.
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    sqlx::query("UPDATE v86_system_versions SET storage_key = ? WHERE id = ?")
        .bind(&storage_key)
        .bind(version_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query(
        "UPDATE v86_system_upload_sessions SET status = 'consumed', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(upload_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;

    chunk_progress_map().lock().unwrap().remove(upload_id);

    Ok(())
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

pub async fn delete_system_version(
    State(state): State<Arc<AppState>>,
    AxumPath((system_id, version_id)): AxumPath<(i64, i64)>,
) -> Result<StatusCode, ProjectError> {
    let usage: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM project_v86_games WHERE system_version_id = ?")
            .bind(version_id)
            .fetch_one(&state.project_service.pool)
            .await?;
    if usage > 0 {
        return Err(ProjectError::Conflict(format!(
            "This system version is used by {usage} project(s) and cannot be deleted."
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
    if let Some(r2) = &state.r2 {
        let _ = r2.delete_prefix(&format!("v86/assets/systems/{version_id}")).await;
    }
    tokio::fs::remove_dir_all(state.project_demo_config.dir.join(storage_key))
        .await
        .ok();
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_system(
    State(state): State<Arc<AppState>>,
    AxumPath(system_id): AxumPath<i64>,
) -> Result<StatusCode, ProjectError> {
    let usage: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM project_v86_games g
           JOIN v86_system_versions v ON v.id = g.system_version_id
           WHERE v.system_id = ?"#,
    )
    .bind(system_id)
    .fetch_one(&state.project_service.pool)
    .await?;
    if usage > 0 {
        return Err(ProjectError::Conflict(format!(
            "This system is referenced by {usage} project(s); deactivate it instead."
        )));
    }
    let keys: Vec<String> =
        sqlx::query_scalar("SELECT storage_key FROM v86_system_versions WHERE system_id = ?")
            .bind(system_id)
            .fetch_all(&state.project_service.pool)
            .await?;
    let version_ids: Vec<i64> =
        sqlx::query_scalar("SELECT id FROM v86_system_versions WHERE system_id = ?")
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
    if let Some(r2) = &state.r2 {
        for version_id in version_ids {
            let _ = r2
                .delete_prefix(&format!("v86/assets/systems/{version_id}"))
                .await;
        }
    }
    for key in keys {
        tokio::fs::remove_dir_all(state.project_demo_config.dir.join(key))
            .await
            .ok();
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn start_game_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<StartGameUploadRequest>,
) -> Result<Json<StartUploadResponse>, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let manifest_sha = validate_manifest(&request.manifest)?;
    let active: Option<i64> = sqlx::query_scalar(
        "SELECT v.id FROM v86_system_versions v JOIN v86_systems s ON s.id = v.system_id WHERE v.id = ? AND (s.is_active = 1 OR EXISTS (SELECT 1 FROM project_v86_games g WHERE g.system_version_id = v.id AND g.project_id = ?))",
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
    let upload_id = Uuid::new_v4().to_string();
    let r2 = require_r2(&state)?;
    let mut r2_upload_id: Option<String> = None;
    let (file_name, expected_size, upload_required, temp_storage_key, received_size) =
        if let Some(project_id) = request.source_project_id
    {
        require_project_owner(&state, project_id, uploader_id).await?;
        let source = sqlx::query(
                "SELECT original_file_name, zip_storage_key, zip_size_bytes, artifact_revision FROM project_v86_games WHERE project_id = ?",
            )
            .bind(project_id)
            .fetch_optional(&state.project_service.pool)
            .await?
            .ok_or(ProjectError::ProjectNotFound)?;
        if source.get::<i64, _>("artifact_revision") != request.expected_artifact_revision {
            return Err(ProjectError::Conflict(
                "The v86 artifact changed in another editor.".to_string(),
            ));
        }
        if let (Some(file_name), Some(size)) = (request.file_name.as_ref(), request.size_bytes) {
            validate_file_name(file_name, ".zip")?;
            if size == 0 || size > state.project_demo_config.max_v86_game_zip_size {
                return Err(ProjectError::InvalidDemo(
                    "The game ZIP exceeds the configured limit.".to_string(),
                ));
            }
            let transient_key = transient_r2_key("games", &upload_id, "zip");
            let multipart = r2
                .create_multipart(&transient_key)
                .await
                .map_err(r2_error)?;
            r2_upload_id = multipart.upload_id().map(str::to_string);
            (file_name.clone(), size, true, transient_key, 0)
        } else {
            let source_key: String = source.get("zip_storage_key");
            // Manifest-only edit: the stored disk is reused and the launcher CD
            // is regenerated from static assets, so the ZIP object itself is
            // never required (it is no longer persisted at all).
            (
                source.get::<String, _>("original_file_name"),
                source.get::<i64, _>("zip_size_bytes") as u64,
                false,
                source_key,
                source.get::<i64, _>("zip_size_bytes") as i64,
            )
        }
    } else {
        let file_name = request.file_name.ok_or_else(|| {
            ProjectError::InvalidDemo("A game ZIP file name is required.".to_string())
        })?;
        validate_file_name(&file_name, ".zip")?;
        let size = request
            .size_bytes
            .ok_or_else(|| ProjectError::InvalidDemo("A game ZIP size is required.".to_string()))?;
        if size == 0 || size > state.project_demo_config.max_v86_game_zip_size {
            return Err(ProjectError::InvalidDemo(
                "The game ZIP exceeds the configured limit.".to_string(),
            ));
        }
        let transient_key = transient_r2_key("games", &upload_id, "zip");
        let multipart = r2
            .create_multipart(&transient_key)
            .await
            .map_err(r2_error)?;
        r2_upload_id = multipart.upload_id().map(str::to_string);
        (file_name, size, true, transient_key, 0)
    };
    let expires_at =
        Utc::now() + Duration::hours(state.project_demo_config.upload_session_ttl_hours as i64);
    // Manifest-only edits reuse the project's permanent zip_storage_key as the
    // temp_storage_key. That column is UNIQUE and stale session rows are never
    // deleted, so drop any row already holding the key before inserting, and
    // clear out the project's finished sessions so they cannot pile up.
    sqlx::query("DELETE FROM project_v86_upload_sessions WHERE temp_storage_key = ?")
        .bind(&temp_storage_key)
        .execute(&state.project_service.pool)
        .await?;
    if let Some(project_id) = request.source_project_id {
        sqlx::query(
            "DELETE FROM project_v86_upload_sessions WHERE source_project_id = ? AND status != 'active'",
        )
        .bind(project_id)
        .execute(&state.project_service.pool)
        .await?;
    }
    sqlx::query(
        r#"INSERT INTO project_v86_upload_sessions
           (id, uploader_id, source_project_id, system_version_id,
            expected_artifact_revision, manifest_text, manifest_sha256,
            original_file_name, expected_size_bytes, received_size_bytes,
            upload_chunk_size_bytes, temp_storage_key, r2_upload_id, expires_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&upload_id)
    .bind(uploader_id)
    .bind(request.source_project_id)
    .bind(request.system_version_id)
    .bind(request.expected_artifact_revision)
    .bind(&request.manifest)
    .bind(manifest_sha)
    .bind(&file_name)
    .bind(expected_size as i64)
    .bind(received_size)
    .bind(state.project_demo_config.v86_upload_chunk_size as i64)
    .bind(&temp_storage_key)
    .bind(&r2_upload_id)
    .bind(expires_at.to_rfc3339())
    .execute(&state.project_service.pool)
    .await
    .map_err(|error| {
        if let Some(upload_id) = &r2_upload_id {
            let r2 = r2.clone();
            let transient_key = temp_storage_key.clone();
            let upload_id = upload_id.clone();
            tokio::spawn(async move {
                let _ = r2.abort_multipart(&transient_key, &upload_id).await;
            });
        }
        error
    })?;
    Ok(Json(StartUploadResponse {
        upload_id,
        chunk_size_bytes: state.project_demo_config.v86_upload_chunk_size,
        next_chunk_index: 0,
        expected_size_bytes: expected_size,
        upload_required,
    }))
}

pub async fn append_game_chunk(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath((upload_id, chunk_index)): AxumPath<(String, u64)>,
    bytes: Bytes,
) -> Result<Json<ChunkUploadResponse>, ProjectError> {
    Ok(Json(
        append_upload_chunk(
            &state,
            "project_v86_upload_sessions",
            &upload_id,
            user_id(&claims)?,
            chunk_index,
            bytes,
        )
        .await?,
    ))
}

/// The stored artifact of a project, used to decide whether a rebuild is
/// needed when only the manifest changed.
struct StoredGameArtifact {
    zip_storage_key: String,
    zip_sha256: String,
    manifest_sha256: String,
    disk_storage_key: String,
    disk_sha256: String,
    disk_size_bytes: i64,
    iso_storage_key: String,
    iso_sha256: String,
    iso_size_bytes: i64,
    chunk_count: i64,
}

async fn fetch_stored_game_artifact(
    pool: &sqlx::SqlitePool,
    project_id: i64,
) -> Result<Option<StoredGameArtifact>, String> {
    let row = sqlx::query(
        r#"SELECT zip_storage_key, zip_sha256, manifest_sha256,
                  disk_storage_key, disk_sha256, disk_size_bytes,
                  iso_storage_key, iso_sha256, iso_size_bytes, chunk_count
           FROM project_v86_games WHERE project_id = ?"#,
    )
    .bind(project_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(row.map(|row| StoredGameArtifact {
        zip_storage_key: row.get("zip_storage_key"),
        zip_sha256: row.get("zip_sha256"),
        manifest_sha256: row.get("manifest_sha256"),
        disk_storage_key: row.get("disk_storage_key"),
        disk_sha256: row.get("disk_sha256"),
        disk_size_bytes: row.get("disk_size_bytes"),
        iso_storage_key: row.get("iso_storage_key"),
        iso_sha256: row.get("iso_sha256"),
        iso_size_bytes: row.get("iso_size_bytes"),
        chunk_count: row.get("chunk_count"),
    }))
}

/// Looks up any project that already has a game disk for the given ZIP sha.
/// Enables cross-project dedup: multiple projects can share the same content-
/// addressed game artifacts.
async fn fetch_shared_game_artifact(
    pool: &sqlx::SqlitePool,
    zip_sha: &str,
) -> Result<Option<StoredGameArtifact>, String> {
    let row = sqlx::query(
        r#"SELECT zip_storage_key, zip_sha256, manifest_sha256,
                  disk_storage_key, disk_sha256, disk_size_bytes,
                  iso_storage_key, iso_sha256, iso_size_bytes, chunk_count
           FROM project_v86_games
           WHERE zip_sha256 = ? AND disk_storage_key IS NOT NULL
           LIMIT 1"#,
    )
    .bind(zip_sha)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(row.map(|row| StoredGameArtifact {
        zip_storage_key: row.get("zip_storage_key"),
        zip_sha256: row.get("zip_sha256"),
        manifest_sha256: row.get("manifest_sha256"),
        disk_storage_key: row.get("disk_storage_key"),
        disk_sha256: row.get("disk_sha256"),
        disk_size_bytes: row.get("disk_size_bytes"),
        iso_storage_key: row.get("iso_storage_key"),
        iso_sha256: row.get("iso_sha256"),
        iso_size_bytes: row.get("iso_size_bytes"),
        chunk_count: row.get("chunk_count"),
    }))
}

#[allow(clippy::too_many_arguments)]
async fn process_game_upload(
    pool: &sqlx::SqlitePool,
    r2: &R2Client,
    config_dir: &Path,
    assets_dir: &Path,
    xorriso_bin: &str,
    mtools_bin: &str,
    upload_id: &str,
    transient_key: &str,
    local_download: &Path,
    manifest: &str,
    max_files: usize,
    max_extracted_size: u64,
    chunk_size: u64,
    source_project_id: Option<i64>,
    manifest_only: bool,
    progress: Arc<Mutex<ChunkProgress>>,
) -> Result<(), String> {
    let stored = match source_project_id {
        Some(project_id) => fetch_stored_game_artifact(pool, project_id).await?,
        None => None,
    };

    let manifest_sha = hex::encode(Sha256::digest(manifest.as_bytes()));

    let (zip_key, zip_sha, disk_key, disk_sha, disk_size, disk_chunk_count, iso_key, iso_sha, iso_size);
    let build_dir = config_dir.join("v86").join("tmp").join("build").join(upload_id);

    if manifest_only {
        // Manifest-only edit: the stored disk is reused as-is; the ZIP is never
        // downloaded. Only the launcher CD is (re)generated from static assets
        // when the manifest changed, so tagging it onto an empty source.
        let artifact = stored
            .as_ref()
            .ok_or_else(|| "source project artifact missing".to_string())?;
        zip_key = artifact.zip_storage_key.clone();
        zip_sha = artifact.zip_sha256.clone();
        disk_key = artifact.disk_storage_key.clone();
        disk_sha = artifact.disk_sha256.clone();
        disk_size = artifact.disk_size_bytes;
        disk_chunk_count = artifact.chunk_count;
        if artifact.manifest_sha256 == manifest_sha {
            // No-op: reuse the stored ISO as well; nothing to build or upload.
            {
                let mut p = progress.lock().unwrap();
                p.message = "No changes to rebuild.".to_string();
            }
            iso_key = artifact.iso_storage_key.clone();
            iso_sha = artifact.iso_sha256.clone();
            iso_size = artifact.iso_size_bytes;
        } else {
            // ISO-only: the manifest changed, so rebuild just the launcher CD.
            {
                let mut p = progress.lock().unwrap();
                p.message = "Rebuilding launcher…".to_string();
            }
            let (state_dir, assets, xorriso, uid, mf) = (
                config_dir.to_path_buf(),
                assets_dir.to_path_buf(),
                xorriso_bin.to_string(),
                upload_id.to_string(),
                manifest.to_string(),
            );
            let cdrom_path = tokio::task::spawn_blocking(move || {
                build_windows95_cdrom_only(&state_dir, &assets, &xorriso, &uid, &mf)
            })
            .await
            .map_err(|e| format!("cdrom build task panicked: {e}"))?
            .map_err(|e| e.to_string())?;
            let (iso_size_raw, iso_sha_raw) = tokio::task::spawn_blocking({
                let cdrom = cdrom_path.clone();
                move || sha256_file(&cdrom)
            })
            .await
            .map_err(|e| format!("sha256 task panicked: {e}"))?
            .map_err(|e| e.to_string())?;
            iso_sha = iso_sha_raw;
            iso_size = iso_size_raw as i64;
            iso_key = format!("v86/games/{iso_sha}");
            r2.put_object_from_file(&format!("{iso_key}/full.iso"), &cdrom_path)
                .await
                .map_err(|e| e.to_string())?;
            let _ = tokio::fs::remove_file(&cdrom_path).await;
            let _ = tokio::fs::remove_dir_all(&build_dir).await;
        }
    } else {
        if let Some(parent) = local_download.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| e.to_string())?;
        }
        r2.download_to_file(transient_key, local_download)
            .await
            .map_err(|e| e.to_string())?;
        let source = local_download.to_path_buf();

        let (_zip_size, computed_zip_sha) = tokio::task::spawn_blocking({
            let source = source.clone();
            move || sha256_file(&source)
        })
        .await
        .map_err(|e| format!("sha256 task panicked: {e}"))?
        .map_err(|e| e.to_string())?;
        zip_sha = computed_zip_sha;

        // Also look for an existing project with the same ZIP content to enable
        // cross-project dedup. Only consider rows that already have a game disk.
        let shared = fetch_shared_game_artifact(pool, &zip_sha).await?;
        // Reuse the stored disk when this source project already built it from
        // the same ZIP, or another project has a game disk for this content.
        let reuse_disk = stored
            .as_ref()
            .is_some_and(|artifact| artifact.zip_sha256 == zip_sha)
            || shared.is_some();

        if reuse_disk {
            let artifact = stored
                .as_ref()
                .filter(|a| a.zip_sha256 == zip_sha)
                .or(shared.as_ref())
                .expect("reuse_disk implies a stored or shared artifact");
            zip_key = format!("v86/games/zips/{zip_sha}.zip");
            disk_key = artifact.disk_storage_key.clone();
            disk_sha = artifact.disk_sha256.clone();
            disk_size = artifact.disk_size_bytes;
            disk_chunk_count = artifact.chunk_count;
            if artifact.manifest_sha256 == manifest_sha {
                // No-op: reuse the stored ISO as well; nothing to build or upload.
                {
                    let mut p = progress.lock().unwrap();
                    p.message = "No changes to rebuild.".to_string();
                }
                iso_key = artifact.iso_storage_key.clone();
                iso_sha = artifact.iso_sha256.clone();
                iso_size = artifact.iso_size_bytes;
            } else {
                // ISO-only: the manifest changed, so rebuild just the launcher CD.
                {
                    let mut p = progress.lock().unwrap();
                    p.message = "Rebuilding launcher…".to_string();
                }
                let (state_dir, assets, xorriso, uid, mf) = (
                    config_dir.to_path_buf(),
                    assets_dir.to_path_buf(),
                    xorriso_bin.to_string(),
                    upload_id.to_string(),
                    manifest.to_string(),
                );
                let cdrom_path = tokio::task::spawn_blocking(move || {
                    build_windows95_cdrom_only(&state_dir, &assets, &xorriso, &uid, &mf)
                })
                .await
                .map_err(|e| format!("cdrom build task panicked: {e}"))?
                .map_err(|e| e.to_string())?;
                let (iso_size_raw, iso_sha_raw) = tokio::task::spawn_blocking({
                    let cdrom = cdrom_path.clone();
                    move || sha256_file(&cdrom)
                })
                .await
                .map_err(|e| format!("sha256 task panicked: {e}"))?
                .map_err(|e| e.to_string())?;
                iso_sha = iso_sha_raw;
                iso_size = iso_size_raw as i64;
                iso_key = format!("v86/games/{iso_sha}");
                r2.put_object_from_file(&format!("{iso_key}/full.iso"), &cdrom_path)
                    .await
                    .map_err(|e| e.to_string())?;
                let _ = tokio::fs::remove_file(&cdrom_path).await;
                let _ = tokio::fs::remove_dir_all(&build_dir).await;
            }
        } else {
            // Full build: a new or changed ZIP. Extract, partition the disk,
            // split it into zstd chunks and upload everything.
            {
                let mut p = progress.lock().unwrap();
                p.message = "Building game disc…".to_string();
            }
            let (disk_path, cdrom_path, _extracted_bytes) = {
                let state_dir = config_dir.to_path_buf();
                let assets = assets_dir.to_path_buf();
                let xorriso = xorriso_bin.to_string();
                let mtools = mtools_bin.to_string();
                let uid = upload_id.to_string();
                let mf = manifest.to_string();
                let src = source.clone();
                tokio::task::spawn_blocking(move || {
                    build_windows95_disk(
                        &state_dir,
                        &assets,
                        &xorriso,
                        &mtools,
                        &uid,
                        &src,
                        &mf,
                        max_files,
                        max_extracted_size,
                    )
                })
                .await
                .map_err(|e| format!("disk build task panicked: {e}"))?
                .map_err(|e| e.to_string())?
            };

            let (disk_size_raw, disk_sha_raw) = tokio::task::spawn_blocking({
                let disk = disk_path.clone();
                move || sha256_file(&disk)
            })
            .await
            .map_err(|e| format!("sha256 task panicked: {e}"))?
            .map_err(|e| e.to_string())?;
            disk_sha = disk_sha_raw;
            disk_size = disk_size_raw as i64;

            let (iso_size_raw, iso_sha_raw) = tokio::task::spawn_blocking({
                let cdrom = cdrom_path.clone();
                move || sha256_file(&cdrom)
            })
            .await
            .map_err(|e| format!("sha256 task panicked: {e}"))?
            .map_err(|e| e.to_string())?;
            iso_sha = iso_sha_raw;
            iso_size = iso_size_raw as i64;

            // Split the raw FAT image into the same zstd chunk layout as the
            // base disk so the browser can stream it with use_parts.
            let parts_dir = config_dir
                .join("v86")
                .join("tmp")
                .join("games")
                .join(format!("{upload_id}.diskparts"));
            disk_chunk_count = tokio::task::spawn_blocking({
                let disk = disk_path.clone();
                let parts = parts_dir.clone();
                move || split_asset(&disk, &parts, chunk_size, "img.zst", None, 6)
            })
            .await
            .map_err(|e| format!("disk split task panicked: {e}"))?
            .map_err(|e| e.to_string())?
            .try_into()
            .unwrap();

            {
                let mut p = progress.lock().unwrap();
                p.message = "Uploading to R2…".to_string();
            }

            // Content-addressed artifacts the browser serves straight from R2.
            // The ZIP itself is no longer persisted: only the disk chunks and
            // launcher CD are stored, keyed by their own content hashes.
            zip_key = format!("v86/games/zips/{zip_sha}.zip");
            disk_key = format!("v86/games/{disk_sha}");
            iso_key = format!("v86/games/{iso_sha}");
            if let Err(error) = push_dir_to_r2(r2, &disk_key, &parts_dir).await {
                let _ = r2.delete_prefix(&disk_key).await;
                return Err(error);
            }
            r2.put_object_from_file(&format!("{iso_key}/full.iso"), &cdrom_path)
                .await
                .map_err(|e| e.to_string())?;
            let _ = tokio::fs::remove_file(&disk_path).await;
            let _ = tokio::fs::remove_file(&cdrom_path).await;
            let _ = tokio::fs::remove_dir_all(&parts_dir).await;
            let _ = tokio::fs::remove_dir_all(&build_dir).await;
        }
    }

    // Drop the session-owned transient ZIP. Manifest-only edits reuse the
    // stored disk key as the session key, which is not a transient object, so
    // it is never removed here.
    if transient_key.starts_with("v86/tmp/games/") {
        let _ = r2.delete_object(transient_key).await;
    }
    if manifest_only {
        let _ = tokio::fs::remove_dir_all(&build_dir).await;
    } else {
        let _ = tokio::fs::remove_file(&local_download).await;
    }

    sqlx::query(
        r#"UPDATE project_v86_upload_sessions
           SET status = 'ready',
               staged_zip_storage_key = ?, staged_zip_sha256 = ?,
               staged_disk_storage_key = ?, staged_disk_sha256 = ?, staged_disk_size_bytes = ?,
               staged_iso_storage_key = ?, staged_iso_sha256 = ?, staged_iso_size_bytes = ?,
               staged_iso_chunk_count = ?,
               updated_at = CURRENT_TIMESTAMP
           WHERE id = ? AND status = 'building'"#,
    )
    .bind(&zip_key)
    .bind(&zip_sha)
    .bind(&disk_key)
    .bind(&disk_sha)
    .bind(disk_size)
    .bind(&iso_key)
    .bind(&iso_sha)
    .bind(iso_size)
    .bind(disk_chunk_count)
    .bind(upload_id)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    chunk_progress_map().lock().unwrap().remove(upload_id);

    Ok(())
}

pub async fn complete_game_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(upload_id): AxumPath<String>,
) -> Result<StatusCode, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let row = sqlx::query(
        "SELECT manifest_text, source_project_id, original_file_name, expected_size_bytes, received_size_bytes, temp_storage_key, r2_upload_id, r2_part_etags, status, expires_at FROM project_v86_upload_sessions WHERE id = ? AND uploader_id = ?",
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
            "The game ZIP upload is incomplete.".to_string(),
        ));
    }
    let uploaded_transient = row.get::<Option<String>, _>("r2_upload_id").is_some();
    if let Some(r2_upload_id) = row.get::<Option<String>, _>("r2_upload_id") {
        let r2 = require_r2(&state)?;
        let temp_key: String = row.get("temp_storage_key");
        let etags = parse_r2_part_etags(row.get::<Option<String>, _>("r2_part_etags").as_deref());
        r2.complete_multipart(&temp_key, &r2_upload_id, etags)
            .await
            .map_err(r2_error)?;
    }
    sqlx::query(
        "UPDATE project_v86_upload_sessions SET status = 'building', updated_at = CURRENT_TIMESTAMP WHERE id = ? AND status = 'active'",
    )
    .bind(&upload_id)
    .execute(&state.project_service.pool)
    .await?;

    let manifest: String = row.get("manifest_text");
    let temp_key: String = row.get("temp_storage_key");
    let xorriso = state.project_demo_config.xorriso_bin.clone();
    let mtools = state.project_demo_config.mtools_bin.clone();
    let chunk_size = state.project_demo_config.v86_download_chunk_size;
    let max_files = state.project_demo_config.max_v86_game_files;
    let max_extracted = state.project_demo_config.max_v86_game_extracted_size;

    let progress = Arc::new(Mutex::new(ChunkProgress {
        upload_id: upload_id.clone(),
        kind: "game".to_string(),
        total_chunks: 0,
        completed_chunks: 0,
        message: "Preparing…".to_string(),
    }));
    chunk_progress_map()
        .lock()
        .unwrap()
        .insert(upload_id.clone(), progress.lock().unwrap().clone());

    let pool = state.project_service.pool.clone();
    let r2 = require_r2(&state)?;
    let r2_c = r2.clone();
    let config_dir = state.project_demo_config.dir.clone();
    let assets_dir = state.project_demo_config.v86_assets_dir.clone();
    let upload_id_c = upload_id.clone();
    let temp_key_c = temp_key.clone();
    let source_project_id: Option<i64> = row.get("source_project_id");
    // A source project with no fresh ZIP upload means a manifest-only edit:
    // the stored disk is reused and the launcher CD is rebuilt from static
    // assets — the ZIP is neither downloaded nor stored.
    let manifest_only = source_project_id.is_some() && !uploaded_transient;
    let local_download = temp_upload_path(&state, "games", &upload_id);
    let local_download_c = local_download.clone();
    let manifest_c = manifest.clone();
    let xorriso_c = xorriso.clone();
    let mtools_c = mtools.clone();
    let progress_c = progress.clone();

    tokio::spawn(async move {
        let result = process_game_upload(
            &pool,
            &r2_c,
            &config_dir,
            &assets_dir,
            &xorriso_c,
            &mtools_c,
            &upload_id_c,
            &temp_key_c,
            &local_download_c,
            &manifest_c,
            max_files,
            max_extracted,
            chunk_size,
            source_project_id,
            manifest_only,
            progress_c,
        )
        .await;
        if let Err(e) = result {
            tracing::error!("Background game upload failed for {upload_id_c}: {e}");
            if uploaded_transient {
                let _ = r2_c.delete_object(&temp_key_c).await;
            }
            let _ = tokio::fs::remove_file(&local_download_c).await;
            let _ = tokio::fs::remove_dir_all(config_dir.join("v86/tmp/build").join(&upload_id_c))
                .await;
            let _ = sqlx::query(
                "UPDATE project_v86_upload_sessions SET status = 'failed', error_message = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            )
            .bind(&e)
            .bind(&upload_id_c)
            .execute(&pool)
            .await;
            chunk_progress_map().lock().unwrap().remove(&upload_id_c);
        }
    });

    Ok(StatusCode::ACCEPTED)
}

pub async fn attach_ready_game_tx(
    tx: &mut Transaction<'_, Sqlite>,
    project_id: i64,
    uploader_id: i64,
    upload_id: &str,
    chunk_size: u64,
) -> Result<i64, ProjectError> {
    let row = sqlx::query(
        r#"SELECT source_project_id, system_version_id, expected_artifact_revision,
                  manifest_text, manifest_sha256, original_file_name,
                  expected_size_bytes, staged_zip_storage_key, staged_zip_sha256,
                  staged_disk_storage_key, staged_disk_sha256, staged_disk_size_bytes,
                  staged_iso_storage_key, staged_iso_sha256, staged_iso_size_bytes,
                  staged_iso_chunk_count, status, expires_at
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
    if source_project.is_some() && source_project != Some(project_id) {
        return Err(ProjectError::Forbidden);
    }
    let expected: i64 = row.get("expected_artifact_revision");
    let current: i64 = sqlx::query_scalar(
        "SELECT COALESCE((SELECT artifact_revision FROM project_v86_games WHERE project_id = ?), 0)",
    )
    .bind(project_id)
    .fetch_one(&mut **tx)
    .await?;
    if current != expected {
        return Err(ProjectError::Conflict(
            "The v86 artifact changed while this package was building.".to_string(),
        ));
    }
    let revision = current + 1;
    let artifact_change = sqlx::query(
        r#"INSERT INTO project_v86_games
           (project_id, system_version_id, manifest_text, manifest_sha256,
            original_file_name, zip_storage_key, zip_size_bytes, zip_sha256,
            disk_storage_key, disk_size_bytes, disk_sha256,
            iso_storage_key, iso_size_bytes, iso_sha256, chunk_size_bytes,
            chunk_count, artifact_revision)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
           ON CONFLICT(project_id) DO UPDATE SET
             system_version_id = excluded.system_version_id,
             manifest_text = excluded.manifest_text,
             manifest_sha256 = excluded.manifest_sha256,
             original_file_name = excluded.original_file_name,
             zip_storage_key = excluded.zip_storage_key,
             zip_size_bytes = excluded.zip_size_bytes,
             zip_sha256 = excluded.zip_sha256,
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
           WHERE project_v86_games.artifact_revision = ?"#,
    )
    .bind(project_id)
    .bind(row.get::<i64, _>("system_version_id"))
    .bind(row.get::<String, _>("manifest_text"))
    .bind(row.get::<String, _>("manifest_sha256"))
    .bind(row.get::<String, _>("original_file_name"))
    .bind(row.get::<String, _>("staged_zip_storage_key"))
    .bind(row.get::<i64, _>("expected_size_bytes"))
    .bind(row.get::<String, _>("staged_zip_sha256"))
    .bind(row.get::<String, _>("staged_disk_storage_key"))
    .bind(row.get::<i64, _>("staged_disk_size_bytes"))
    .bind(row.get::<String, _>("staged_disk_sha256"))
    .bind(row.get::<String, _>("staged_iso_storage_key"))
    .bind(row.get::<i64, _>("staged_iso_size_bytes"))
    .bind(row.get::<String, _>("staged_iso_sha256"))
    .bind(chunk_size as i64)
    .bind(row.get::<i64, _>("staged_iso_chunk_count"))
    .bind(revision)
    .bind(expected)
    .execute(&mut **tx)
    .await?;
    if artifact_change.rows_affected() != 1 {
        return Err(ProjectError::Conflict(
            "The v86 artifact changed while this package was building.".to_string(),
        ));
    }
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

pub async fn attach_ready_game(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath((project_id, upload_id)): AxumPath<(i64, String)>,
) -> Result<Json<serde_json::Value>, ProjectError> {
    let uploader_id = user_id(&claims)?;
    require_project_owner(&state, project_id, uploader_id).await?;
    let mut tx = state.project_service.pool.begin().await?;
    let revision = attach_ready_game_tx(
        &mut tx,
        project_id,
        uploader_id,
        &upload_id,
        state.project_demo_config.v86_download_chunk_size,
    )
    .await?;
    sqlx::query(
        "UPDATE projects SET demo_type = 'v86', demo_url = NULL, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(project_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(serde_json::json!({ "artifact_revision": revision })))
}

pub async fn abort_game_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(upload_id): AxumPath<String>,
) -> Result<StatusCode, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let row = sqlx::query(
        "SELECT temp_storage_key, r2_upload_id, status FROM project_v86_upload_sessions WHERE id = ? AND uploader_id = ?",
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
    if let Some(r2) = &state.r2 {
        let temp: String = row.get("temp_storage_key");
        if let Some(r2_upload_id) = row.get::<Option<String>, _>("r2_upload_id") {
            let _ = r2.abort_multipart(&temp, &r2_upload_id).await;
        }
        let _ = r2.delete_object(&temp).await;
    }
    tokio::fs::remove_file(temp_upload_path(&state, "games", &upload_id))
        .await
        .ok();
    tokio::fs::remove_dir_all(
        state
            .project_demo_config
            .dir
            .join("v86/tmp/build")
            .join(&upload_id),
    )
    .await
    .ok();
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

pub async fn runtime_descriptor(
    pool: &sqlx::SqlitePool,
    slug: &str,
    r2_public_url: Option<&str>,
) -> Result<Option<V86RuntimeDescriptor>, ProjectError> {
    let row = sqlx::query(
        r#"SELECT s.name AS system_name, s.platform_key, v.id AS system_version_id,
                  v.size_bytes AS base_size, v.sha256 AS base_sha,
                  g.disk_size_bytes, g.disk_sha256,
                  g.iso_size_bytes, g.iso_sha256, g.manifest_text, g.manifest_sha256,
                  g.chunk_size_bytes, g.artifact_revision,
                  p.demo_width, p.demo_height
           FROM project_v86_games g
           JOIN projects p ON p.id = g.project_id
           JOIN posts ON posts.id = p.post_id
           JOIN v86_system_versions v ON v.id = g.system_version_id
           JOIN v86_systems s ON s.id = v.system_id
           WHERE posts.slug = ? AND posts.status = 'published' AND p.demo_type = 'v86'"#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|row| {
        let version_id: i64 = row.get("system_version_id");
        let base_sha: String = row.get("base_sha");
        let game_sha: String = row.get("disk_sha256");
        let iso_sha: String = row.get("iso_sha256");
        let save_supported = has_save_paths(&row.get::<String, _>("manifest_text"));
        let (base_url, game_url, iso_url) = match r2_public_url {
            Some(r2) => {
                let base = r2.trim_end_matches('/');
                (
                    format!("{base}/v86/assets/systems/{version_id}/{base_sha}/.img.zst"),
                    format!("{base}/v86/games/{game_sha}/.img.zst"),
                    format!("{base}/v86/games/{iso_sha}/full.iso"),
                )
            }
            None => (
                format!("v86/assets/systems/{version_id}/{base_sha}/.img.zst"),
                format!("projects/s/{slug}/v86/disk/{game_sha}/.img.zst"),
                format!("projects/s/{slug}/v86/{iso_sha}/full.iso"),
            ),
        };
        V86RuntimeDescriptor {
            platform_key: row.get("platform_key"),
            system_name: row.get("system_name"),
            system_version_id: version_id,
            artifact_revision: row.get("artifact_revision"),
            manifest_sha256: row.get("manifest_sha256"),
            slug: slug.to_string(),
            memory_size: 64 * 1024 * 1024,
            vga_memory_size: 8 * 1024 * 1024,
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
            save_supported,
            save_max_bytes: V86_SAVE_FLOPPY_BYTES as u64,
        }
    }))
}

fn has_save_paths(manifest: &str) -> bool {
    !save_files_from_manifest(manifest)
        .map(|files| files.is_empty())
        .unwrap_or(true)
}

fn immutable_chunk_response(bytes: Vec<u8>, content_type: &'static str) -> Response {
    let mut response = bytes.into_response();
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    response
}

pub async fn get_system_chunk(
    State(state): State<Arc<AppState>>,
    AxumPath((version_id, sha256, part)): AxumPath<(i64, String, String)>,
) -> Result<Response, ProjectError> {
    let storage_key: Option<String> = sqlx::query_scalar(
        r#"SELECT v.storage_key FROM v86_system_versions v
           WHERE v.id = ? AND v.sha256 = ?
             AND EXISTS (
               SELECT 1 FROM project_v86_games g
               JOIN projects p ON p.id = g.project_id
               JOIN posts ON posts.id = p.post_id
               WHERE g.system_version_id = v.id
                 AND p.demo_type = 'v86' AND posts.status = 'published'
             )"#,
    )
    .bind(version_id)
    .bind(&sha256)
    .fetch_optional(&state.project_service.pool)
    .await?;
    let storage_key = storage_key.ok_or(ProjectError::ProjectNotFound)?;
    if part == ".img" || part.contains('/') || !(part.ends_with(".img") || part.ends_with(".img.zst")) {
        return Err(ProjectError::ProjectNotFound);
    }
    if let Some(r2) = &state.r2 {
        let key = format!("v86/assets/systems/{version_id}/{sha256}/{part}");
        let bytes = r2
            .get_object(&key)
            .await
            .map_err(|_| ProjectError::ProjectNotFound)?;
        return Ok(immutable_chunk_response(bytes, "application/octet-stream"));
    }
    let bytes = tokio::fs::read(
        state
            .project_demo_config
            .dir
            .join(storage_key)
            .join("parts")
            .join(part),
    )
    .await
    .map_err(|_| ProjectError::ProjectNotFound)?;
    Ok(immutable_chunk_response(bytes, "application/octet-stream"))
}

pub async fn get_game_chunk(
    State(state): State<Arc<AppState>>,
    AxumPath((slug, sha256, part)): AxumPath<(String, String, String)>,
) -> Result<Response, ProjectError> {
    let storage_key: Option<String> = sqlx::query_scalar(
        r#"SELECT g.iso_storage_key FROM project_v86_games g
           JOIN projects p ON p.id = g.project_id
           JOIN posts ON posts.id = p.post_id
           WHERE posts.slug = ? AND posts.status = 'published'
             AND p.demo_type = 'v86' AND g.iso_sha256 = ?"#,
    )
    .bind(&slug)
    .bind(&sha256)
    .fetch_optional(&state.project_service.pool)
    .await?;
    let storage_key = storage_key.ok_or(ProjectError::ProjectNotFound)?;
    if part == ".iso" || part.contains('/') || !(part.ends_with(".iso") || part.ends_with(".iso.zst")) {
        return Err(ProjectError::ProjectNotFound);
    }
    let bytes = tokio::fs::read(
        state
            .project_demo_config
            .dir
            .join(storage_key)
            .join("parts")
            .join(part),
    )
    .await
    .map_err(|_| ProjectError::ProjectNotFound)?;
    Ok(immutable_chunk_response(bytes, "application/octet-stream"))
}

pub async fn get_game_disk_chunk(
    State(state): State<Arc<AppState>>,
    AxumPath((slug, sha256, part)): AxumPath<(String, String, String)>,
) -> Result<Response, ProjectError> {
    let storage_key: Option<String> = sqlx::query_scalar(
        r#"SELECT g.disk_storage_key FROM project_v86_games g
           JOIN projects p ON p.id = g.project_id
           JOIN posts ON posts.id = p.post_id
           WHERE posts.slug = ? AND posts.status = 'published'
             AND p.demo_type = 'v86' AND g.disk_sha256 = ?"#,
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
    if let Some(r2) = &state.r2 {
        let key = format!("{storage_key}/{part}");
        let bytes = r2
            .get_object(&key)
            .await
            .map_err(|_| ProjectError::ProjectNotFound)?;
        return Ok(immutable_chunk_response(bytes, "application/octet-stream"));
    }
    let bytes = tokio::fs::read(
        state
            .project_demo_config
            .dir
            .join(storage_key)
            .join(part),
    )
    .await
    .map_err(|_| ProjectError::ProjectNotFound)?;
    Ok(immutable_chunk_response(bytes, "application/octet-stream"))
}

pub async fn get_game_iso(
    State(state): State<Arc<AppState>>,
    AxumPath((slug, sha256)): AxumPath<(String, String)>,
) -> Result<Response, ProjectError> {
    let storage_key: Option<String> = sqlx::query_scalar(
        r#"SELECT g.iso_storage_key FROM project_v86_games g
           JOIN projects p ON p.id = g.project_id
           JOIN posts ON posts.id = p.post_id
           WHERE posts.slug = ? AND posts.status = 'published'
             AND p.demo_type = 'v86' AND g.iso_sha256 = ?"#,
    )
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

    if let Some(r2) = &state.r2 {
        let key = format!("v86/games/{sha256}/full.iso");
        let size = r2
            .object_size(&key)
            .await
            .map_err(r2_error)?
            .ok_or(ProjectError::ProjectNotFound)?;
        let reader = r2.get_object_reader(&key).await.map_err(r2_error)?;
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

    let file = tokio::fs::File::open(
        state
            .project_demo_config
            .dir
            .join(storage_key)
            .join("game.iso"),
    )
    .await
    .map_err(|_| ProjectError::ProjectNotFound)?;
    let size = file.metadata().await?.len();
    let mut response = Response::new(axum::body::Body::from_stream(ReaderStream::new(file)));
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
    Ok(response)
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

fn save_rate_limit_key(user_id: i64, project_id: i64) -> String {
    format!("{user_id}:{project_id}")
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

async fn save_project_id(
    state: &AppState,
    slug: &str,
) -> Result<i64, ProjectError> {
    sqlx::query_scalar(
        r#"SELECT p.id FROM projects p
           JOIN posts ON posts.id = p.post_id
           WHERE posts.slug = ? AND posts.status = 'published' AND p.demo_type = 'v86'"#,
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
    let project_id = save_project_id(&state, &slug).await?;
    let row = sqlx::query(
        "SELECT storage_key, size_bytes FROM v86_saves WHERE project_id = ? AND user_id = ?",
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_optional(&state.project_service.pool)
    .await?;
    let (storage_key, size) = match row {
        Some(row) => (row.get::<String, _>("storage_key"), row.get::<i64, _>("size_bytes")),
        None => return Err(ProjectError::SaveNotFound),
    };
    let compressed = match &state.r2 {
        Some(r2) => r2.get_object(&storage_key).await.map_err(r2_error)?,
        None => {
            tokio::fs::read(
                state
                    .project_demo_config
                    .dir
                    .join(&storage_key),
            )
            .await
            .map_err(|_| ProjectError::SaveNotFound)?
        }
    };
    let mut response = Response::new(axum::body::Body::from(compressed));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    response.headers_mut().insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&size.to_string())
            .map_err(|error| ProjectError::InternalError(error.to_string()))?,
    );
    Ok(response)
}

pub async fn put_game_save(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    AxumPath(slug): AxumPath<String>,
    bytes: Bytes,
) -> Result<StatusCode, ProjectError> {
    let user_id = user_id(&opt_claims.ok_or(ProjectError::Forbidden)?)?;
    let project_id = save_project_id(&state, &slug).await?;
    if bytes.is_empty() || bytes.len() > V86_SAVE_MAX_UPLOAD_BYTES {
        return Err(ProjectError::InvalidDemo(
            "The save image exceeds the allowed size.".to_string(),
        ));
    }
    let key = save_rate_limit_key(user_id, project_id);
    if save_rate_limited(&key) {
        return Err(ProjectError::Conflict(
            "Please wait before saving again.".to_string(),
        ));
    }
    let compressed = zstd_compress(&bytes)?;
    let storage_key = format!("v86/saves/{user_id}/{project_id}/save.zst");
    if let Some(r2) = &state.r2 {
        r2.put_object_bytes(&storage_key, compressed.clone())
            .await
            .map_err(r2_error)?;
    } else {
        let path = state.project_demo_config.dir.join(&storage_key);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&path, &compressed).await?;
    }
    let sha = hex::encode(Sha256::digest(&compressed));
    sqlx::query(
        r#"INSERT INTO v86_saves (project_id, user_id, storage_key, size_bytes, sha256)
           VALUES (?, ?, ?, ?, ?)
           ON CONFLICT(project_id, user_id) DO UPDATE SET
             storage_key = excluded.storage_key,
             size_bytes = excluded.size_bytes,
             sha256 = excluded.sha256,
             updated_at = CURRENT_TIMESTAMP"#,
    )
    .bind(project_id)
    .bind(user_id)
    .bind(&storage_key)
    .bind(compressed.len() as i64)
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
    let project_id = save_project_id(&state, &slug).await?;
    let row = sqlx::query(
        "SELECT storage_key FROM v86_saves WHERE project_id = ? AND user_id = ?",
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_optional(&state.project_service.pool)
    .await?;
    if let Some(row) = row {
        let storage_key: String = row.get("storage_key");
        if let Some(r2) = &state.r2 {
            let _ = r2.delete_object(&storage_key).await;
        }
        let _ = tokio::fs::remove_file(state.project_demo_config.dir.join(&storage_key)).await;
        sqlx::query("DELETE FROM v86_saves WHERE project_id = ? AND user_id = ?")
            .bind(project_id)
            .bind(user_id)
            .execute(&state.project_service.pool)
            .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::{CompressionMethod, ZipWriter, write::FileOptions};

    fn write_zip(path: &Path, files: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut writer = ZipWriter::new(file);
        let options = FileOptions::default().compression_method(CompressionMethod::Stored);
        for (name, contents) in files {
            writer.start_file(*name, options).unwrap();
            writer.write_all(contents).unwrap();
        }
        writer.finish().unwrap();
    }

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
    fn zip_validation_extracts_safe_files_and_rejects_traversal() {
        let root = std::env::temp_dir().join(format!("v86-zip-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let safe_zip = root.join("safe.zip");
        let safe_out = root.join("safe");
        write_zip(
            &safe_zip,
            &[("Doraemon.exe", b"game"), ("DATA/file.dat", b"data")],
        );
        assert!(validate_and_extract_game_zip(&safe_zip, &safe_out, 10, 1024).is_ok());
        assert_eq!(fs::read(safe_out.join("Doraemon.exe")).unwrap(), b"game");

        let unsafe_zip = root.join("unsafe.zip");
        write_zip(&unsafe_zip, &[("../outside.exe", b"bad")]);
        assert!(
            validate_and_extract_game_zip(&unsafe_zip, &root.join("unsafe"), 10, 1024).is_err()
        );
        fs::remove_dir_all(root).ok();
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
    fn windows95_manifest_resolves_a_unique_nested_executable() {
        let root = std::env::temp_dir().join(format!("v86-manifest-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("Doraemon.exe"), b"game").unwrap();

        let config = windows95_launcher_config(
            "exe=Game/Doraemon.exe\nargs=-m\nsave_paths=Save0001.dat; A/save0001.dat; backup/what.bak",
        )
        .ok()
        .unwrap();
        assert!(config.contains(r"executable=D:\Game\Doraemon.exe"));
        assert!(config.contains(r"working_directory=D:\Game"));
        assert!(config.contains("arguments=-m"));
        assert!(config.contains("delay_ms=1000"));
        assert!(config.contains("[saves]\r\n"));
        assert!(config.contains(r"file=Save0001.dat"));
        assert!(config.contains(r"file=A\save0001.dat"));
        assert!(config.contains(r"file=backup\what.bak"));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn manifest_root_exe_uses_drive_root_as_working_directory() {
        // A bare exe at the drive root resolves with no working directory.
        let config = windows95_launcher_config("exe=Doraemon.exe")
            .ok()
            .unwrap();
        assert!(config.contains(r"executable=D:\Doraemon.exe"));
        assert!(config.contains(r"working_directory=D:\"));
    }

    #[test]
    fn unwrap_single_top_level_folder_flattens_nested_wrappers() {
        let root = std::env::temp_dir().join(format!("v86-unwrap-test-{}", Uuid::new_v4()));
        let nested = root.join("Game").join("Game");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("Doraemon.exe"), b"game").unwrap();
        unwrap_single_top_level_dir(&root).unwrap();
        assert!(fs::read(root.join("Doraemon.exe")).is_ok());
        assert!(!root.join("Game").exists());
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn zip_extraction_skips_macos_junk_and_unwraps_single_folder() {
        let root = std::env::temp_dir().join(format!("v86-junk-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let zip_path = root.join("game.zip");
        let out = root.join("out");
        write_zip(
            &zip_path,
            &[
                ("Doraemon/Doraemon.exe", b"game"),
                ("__MACOSX/._Doraemon.exe", b"junk"),
                ("Doraemon/.DS_Store", b"junk"),
            ],
        );
        validate_and_extract_game_zip(&zip_path, &out, 10, 1024).unwrap();
        assert!(out.join("Doraemon").join("Doraemon.exe").is_file());
        assert!(!out.join("__MACOSX").exists());
        assert!(!out.join("Doraemon").join(".DS_Store").exists());
        unwrap_single_top_level_dir(&out).unwrap();
        assert!(out.join("Doraemon.exe").is_file());
        assert!(!out.join("Doraemon").exists());

        let junk_only = root.join("junk.zip");
        write_zip(&junk_only, &[("__MACOSX/x", b"junk"), ("a/.DS_Store", b"junk")]);
        assert!(validate_and_extract_game_zip(&junk_only, &root.join("junk_out"), 10, 1024).is_err());
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn windows95_strategy_builds_disk_launcher_manifest_and_cd() {
        if Command::new("xorriso").arg("-version").output().is_err() {
            return;
        }
        if Command::new("mformat").arg("-h").output().is_err() {
            return;
        }
        if Command::new("mcopy").arg("-h").output().is_err() {
            return;
        }
        let root = std::env::temp_dir().join(format!("v86-disk-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let game_zip = root.join("game.zip");
        write_zip(&game_zip, &[("Doraemon.exe", b"game")]);
        let manifest = "[game]\r\nexecutable=D:\\Doraemon.exe\r\narguments=-m\r\ndelay_ms=1000\r\n";
        let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
        let (disk, iso, extracted) = build_windows95_disk(
            &root,
            &assets_dir,
            "xorriso",
            "",
            "proof",
            &game_zip,
            manifest,
            10,
            1024,
        )
        .map_err(|error| panic!("build_windows95_disk failed: {error:?}"))
        .unwrap();
        assert_eq!(extracted, 4);
        assert!(disk.is_file());
        assert!(iso.is_file());
        let listing = Command::new("xorriso")
            .args(["-indev"])
            .arg(&iso)
            .args(["-find", "/", "-type", "f"])
            .output()
            .unwrap();
        let listed = String::from_utf8_lossy(&listing.stdout);
        assert!(listed.contains("AUTORUN.INF"));
        assert!(listed.contains("LAUNCHER.EXE"));
        assert!(listed.contains("V86GAME.INI"));
        assert!(!listed.contains("Doraemon.exe"));
        let mbr = fs::read(&disk).unwrap();
        assert_ne!(mbr[450], 0, "MBR partition type byte must be set");
        let disk_listing = Command::new("mdir")
            .arg("-i")
            .arg(format!("{}@@{}", disk.display(), 63 * 512))
            .arg("-/")
            .arg("::/")
            .output()
            .unwrap();
        let disk_listed = String::from_utf8_lossy(&disk_listing.stdout);
        assert!(disk_listed.contains("Doraemon.exe"));
        assert!(disk_listed.contains("DORAEMON EXE"));
        // The game must sit at the drive root: no wrapper subdirectory like
        // `Directory for ::/game` may appear in the recursive listing.
        assert!(
            !disk_listed.to_ascii_lowercase().contains("directory for ::/game"),
            "game files must not be nested under a wrapper folder"
        );
        fs::remove_dir_all(root).ok();
    }
}
