use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::Command,
    sync::Arc,
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
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;
use uuid::Uuid;
use zip::ZipArchive;

use crate::{
    domain::{entities::secret::Claims, errors::project::ProjectError},
    infrastructure::web::server::AppState,
};

const MANIFEST_MAX_BYTES: usize = 64 * 1024;

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

#[derive(Serialize)]
pub struct ReadyGameUploadResponse {
    pub upload_id: String,
    pub status: String,
    pub zip_sha256: String,
    pub iso_sha256: String,
    pub iso_size_bytes: u64,
    pub chunk_count: u64,
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
    pub memory_size: u64,
    pub vga_memory_size: u64,
    pub display_width: String,
    pub display_height: String,
    pub chunk_size_bytes: u64,
    pub base_size_bytes: u64,
    pub base_sha256: String,
    pub base_url: String,
    pub iso_size_bytes: u64,
    pub iso_sha256: String,
    pub iso_url: String,
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
) -> Result<u64, ProjectError> {
    fs::create_dir_all(destination)?;
    let mut input = File::open(source)?;
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
        let mut output = File::create(destination.join(format!("{start}-{end}.{extension}")))?;
        output.write_all(&buffer)?;
        start = end;
        count += 1;
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
        if format!(r"D:\GAME\{}", normalized).len() >= 260 {
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
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut output = File::create(output_path)?;
            std::io::copy(&mut entry, &mut output)?;
        }
    }
    Ok(())
}

fn collect_game_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), ProjectError> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_game_files(root, &path, files)?;
        } else if path.is_file() {
            files.push(
                path.strip_prefix(root)
                    .map_err(|_| {
                        ProjectError::InternalError(
                            "Could not resolve an extracted game path.".to_string(),
                        )
                    })?
                    .to_path_buf(),
            );
        }
    }
    Ok(())
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

fn windows95_launcher_config(game_dir: &Path, manifest: &str) -> Result<String, ProjectError> {
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

    let mut files = Vec::new();
    collect_game_files(game_dir, game_dir, &mut files)?;
    let requested_lower = requested.to_ascii_lowercase();
    let requested_has_directory = requested.contains('/');
    let matches = files
        .iter()
        .filter(|relative| {
            let candidate = relative.to_string_lossy().replace('\\', "/");
            if requested_has_directory {
                candidate.eq_ignore_ascii_case(&requested)
            } else {
                relative.file_name().is_some_and(|name| {
                    name.to_string_lossy()
                        .eq_ignore_ascii_case(&requested_lower)
                })
            }
        })
        .collect::<Vec<_>>();

    let relative = match matches.as_slice() {
        [relative] => (*relative).clone(),
        [] => {
            return Err(ProjectError::InvalidDemo(format!(
                "The Windows 95 manifest executable '{requested}' was not found in the game ZIP."
            )));
        }
        _ => {
            return Err(ProjectError::InvalidDemo(format!(
                "The Windows 95 manifest executable '{requested}' is ambiguous; include its folder path."
            )));
        }
    };

    let relative_windows = relative.to_string_lossy().replace('/', "\\");
    let executable = format!(r"D:\GAME\{relative_windows}");
    if executable.len() >= 260 {
        return Err(ProjectError::InvalidDemo(
            "The resolved Windows 95 executable path is too long.".to_string(),
        ));
    }
    let working_directory = relative
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(|parent| format!(r"D:\GAME\{}", parent.to_string_lossy().replace('/', "\\")))
        .unwrap_or_else(|| r"D:\GAME".to_string());
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

    Ok(format!(
        "[game]\r\nexecutable={executable}\r\nworking_directory={working_directory}\r\narguments={arguments}\r\ndelay_ms={delay_ms}\r\n"
    ))
}

fn build_windows95_iso(
    state_dir: &Path,
    xorriso_bin: &str,
    upload_id: &str,
    zip_path: &Path,
    manifest: &str,
    max_files: usize,
    max_extracted_size: u64,
) -> Result<PathBuf, ProjectError> {
    let build_dir = state_dir
        .join("v86")
        .join("tmp")
        .join("build")
        .join(upload_id);
    if build_dir.exists() {
        fs::remove_dir_all(&build_dir)?;
    }
    let disc_dir = build_dir.join("disc");
    let game_dir = disc_dir.join("GAME");
    fs::create_dir_all(&game_dir)?;
    validate_and_extract_game_zip(zip_path, &game_dir, max_files, max_extracted_size)?;

    let launcher = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
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
        windows95_launcher_config(&game_dir, manifest)?.as_bytes(),
    )?;
    fs::write(disc_dir.join("V86GAME.MANIFEST"), manifest.as_bytes())?;

    let iso_path = build_dir.join("game.iso");
    let output = Command::new(xorriso_bin)
        .args(["-as", "mkisofs", "-J", "-V", "V86GAME", "-o"])
        .arg(&iso_path)
        .arg(&disc_dir)
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
    Ok(iso_path)
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
        (system.is_active && system.current_version > 0)
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
    let upload_id = Uuid::new_v4().to_string();
    let temp_path = temp_upload_path(&state, "systems", &upload_id);
    if let Some(parent) = temp_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::File::create(&temp_path).await?;
    let expires_at =
        Utc::now() + Duration::hours(state.project_demo_config.upload_session_ttl_hours as i64);
    sqlx::query(
        r#"INSERT INTO v86_system_upload_sessions
           (id, uploader_id, system_id, name, platform_key, expected_current_version,
            original_file_name, expected_size_bytes, upload_chunk_size_bytes,
            temp_storage_key, expires_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
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
    .bind(temp_path.to_string_lossy().as_ref())
    .bind(expires_at.to_rfc3339())
    .execute(&state.project_service.pool)
    .await?;
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
        "SELECT expected_size_bytes, received_size_bytes, next_chunk_index, upload_chunk_size_bytes, temp_storage_key, status, expires_at FROM {table} WHERE id = ? AND uploader_id = ?"
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
    let new_received = received + bytes.len() as i64;
    let new_next = next + 1;
    let update = format!(
        "UPDATE {table} SET received_size_bytes = ?, next_chunk_index = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND status = 'active' AND next_chunk_index = ?"
    );
    let changed = sqlx::query(&update)
        .bind(new_received)
        .bind(new_next)
        .bind(upload_id)
        .bind(next)
        .execute(&state.project_service.pool)
        .await?;
    if changed.rows_affected() != 1 {
        return Err(ProjectError::Conflict(
            "The upload was changed concurrently.".to_string(),
        ));
    }
    let append_result = async {
        let mut file = tokio::fs::OpenOptions::new()
            .append(true)
            .open(&temp_key)
            .await?;
        file.write_all(&bytes).await?;
        file.flush().await
    }
    .await;
    if let Err(error) = append_result {
        let failed = format!(
            "UPDATE {table} SET status = 'failed', error_message = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
        );
        sqlx::query(&failed)
            .bind("ISO build failed")
            .bind(upload_id)
            .execute(&state.project_service.pool)
            .await
            .ok();
        return Err(error.into());
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
        "SELECT temp_storage_key, status FROM v86_system_upload_sessions WHERE id = ? AND uploader_id = ?",
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
    tokio::fs::remove_file(row.get::<String, _>("temp_storage_key"))
        .await
        .ok();
    Ok(StatusCode::NO_CONTENT)
}

pub async fn complete_system_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(upload_id): AxumPath<String>,
) -> Result<Json<V86SystemResponse>, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let row = sqlx::query(
        "SELECT system_id, name, platform_key, expected_current_version, original_file_name, expected_size_bytes, received_size_bytes, temp_storage_key, status, expires_at FROM v86_system_upload_sessions WHERE id = ? AND uploader_id = ?",
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
    let temp_key: String = row.get("temp_storage_key");
    let mut signature = [0_u8; 512];
    File::open(&temp_key)?.read_exact(&mut signature)?;
    if signature[510..512] != [0x55, 0xaa] {
        return Err(ProjectError::InvalidDemo(
            "The base IMG does not contain a valid boot-sector signature.".to_string(),
        ));
    }
    let (size, sha256) = sha256_file(Path::new(&temp_key))?;
    let system_id: Option<i64> = row.get("system_id");
    let expected_version: i64 = row.get("expected_current_version");
    let name: String = row.get("name");
    let platform_key: String = row.get("platform_key");
    let original_file_name: String = row.get("original_file_name");
    let mut tx = state.project_service.pool.begin().await?;
    let (system_id, version_number) = if let Some(system_id) = system_id {
        let current: i64 =
            sqlx::query_scalar("SELECT current_version FROM v86_systems WHERE id = ?")
                .bind(system_id)
                .fetch_one(&mut *tx)
                .await?;
        if current != expected_version {
            return Err(ProjectError::Conflict(
                "The system was replaced by another administrator.".to_string(),
            ));
        }
        (system_id, current + 1)
    } else {
        let result = sqlx::query("INSERT INTO v86_systems (name, platform_key) VALUES (?, ?)")
            .bind(&name)
            .bind(&platform_key)
            .execute(&mut *tx)
            .await?;
        (result.last_insert_rowid(), 1)
    };
    let storage_key = format!("v86/systems/{system_id}/{version_number}/{sha256}");
    let destination = state.project_demo_config.dir.join(&storage_key);
    let parts = destination.join("parts");
    let chunk_size = state.project_demo_config.v86_download_chunk_size;
    let source = PathBuf::from(&temp_key);
    let parts_for_build = parts.clone();
    let source_for_build = source.clone();
    let chunk_count = tokio::task::spawn_blocking(move || {
        split_asset(&source_for_build, &parts_for_build, chunk_size, "img")
    })
    .await
    .map_err(|e| ProjectError::InternalError(e.to_string()))??;
    fs::create_dir_all(&destination)?;
    fs::remove_file(&source)?;
    let version_result = sqlx::query(
        r#"INSERT INTO v86_system_versions
           (system_id, version_number, original_file_name, storage_key, size_bytes,
            sha256, chunk_size_bytes, chunk_count)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(system_id)
    .bind(version_number)
    .bind(original_file_name)
    .bind(&storage_key)
    .bind(size as i64)
    .bind(&sha256)
    .bind(chunk_size as i64)
    .bind(chunk_count as i64)
    .execute(&mut *tx)
    .await?;
    let system_update = sqlx::query(
        "UPDATE v86_systems SET current_version = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND current_version = ?",
    )
    .bind(version_number)
    .bind(system_id)
    .bind(expected_version)
    .execute(&mut *tx)
    .await?;
    if system_update.rows_affected() != 1 {
        return Err(ProjectError::Conflict(
            "The system was replaced by another administrator.".to_string(),
        ));
    }
    sqlx::query(
        "UPDATE v86_system_upload_sessions SET status = 'consumed', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(&upload_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    let _ = version_result.last_insert_rowid();
    let Json(systems) = list_systems(State(state)).await?;
    systems
        .into_iter()
        .find(|system| system.id == system_id)
        .map(Json)
        .ok_or(ProjectError::ProjectNotFound)
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
    let storage_key: String = row.get("storage_key");
    sqlx::query("DELETE FROM v86_system_versions WHERE id = ?")
        .bind(version_id)
        .execute(&state.project_service.pool)
        .await?;
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
    let changed = sqlx::query("DELETE FROM v86_systems WHERE id = ?")
        .bind(system_id)
        .execute(&state.project_service.pool)
        .await?;
    if changed.rows_affected() != 1 {
        return Err(ProjectError::ProjectNotFound);
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
    let temp_path = temp_upload_path(&state, "games", &upload_id);
    if let Some(parent) = temp_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let (file_name, expected_size, upload_required) = if let Some(project_id) =
        request.source_project_id
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
            tokio::fs::File::create(&temp_path).await?;
            (file_name.clone(), size, true)
        } else {
            let source_key: String = source.get("zip_storage_key");
            tokio::fs::copy(state.project_demo_config.dir.join(source_key), &temp_path).await?;
            (
                source.get::<String, _>("original_file_name"),
                source.get::<i64, _>("zip_size_bytes") as u64,
                false,
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
        tokio::fs::File::create(&temp_path).await?;
        (file_name, size, true)
    };
    let expires_at =
        Utc::now() + Duration::hours(state.project_demo_config.upload_session_ttl_hours as i64);
    sqlx::query(
        r#"INSERT INTO project_v86_upload_sessions
           (id, uploader_id, source_project_id, system_version_id,
            expected_artifact_revision, manifest_text, manifest_sha256,
            original_file_name, expected_size_bytes, received_size_bytes,
            upload_chunk_size_bytes, temp_storage_key, expires_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
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
    .bind(if upload_required {
        0
    } else {
        expected_size as i64
    })
    .bind(state.project_demo_config.v86_upload_chunk_size as i64)
    .bind(temp_path.to_string_lossy().as_ref())
    .bind(expires_at.to_rfc3339())
    .execute(&state.project_service.pool)
    .await?;
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

pub async fn complete_game_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(upload_id): AxumPath<String>,
) -> Result<Json<ReadyGameUploadResponse>, ProjectError> {
    let uploader_id = user_id(&claims)?;
    let row = sqlx::query(
        "SELECT manifest_text, original_file_name, expected_size_bytes, received_size_bytes, temp_storage_key, status, expires_at FROM project_v86_upload_sessions WHERE id = ? AND uploader_id = ?",
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
    sqlx::query(
        "UPDATE project_v86_upload_sessions SET status = 'building', updated_at = CURRENT_TIMESTAMP WHERE id = ? AND status = 'active'",
    )
    .bind(&upload_id)
    .execute(&state.project_service.pool)
    .await?;
    let manifest: String = row.get("manifest_text");
    let temp_key: String = row.get("temp_storage_key");
    let original_file_name: String = row.get("original_file_name");
    let state_dir = state.project_demo_config.dir.clone();
    let xorriso = state.project_demo_config.xorriso_bin.clone();
    let max_files = state.project_demo_config.max_v86_game_files;
    let max_extracted = state.project_demo_config.max_v86_game_extracted_size;
    let upload_for_build = upload_id.clone();
    let temp_for_build = PathBuf::from(&temp_key);
    let manifest_for_build = manifest.clone();
    let build_result = tokio::task::spawn_blocking(move || {
        build_windows95_iso(
            &state_dir,
            &xorriso,
            &upload_for_build,
            &temp_for_build,
            &manifest_for_build,
            max_files,
            max_extracted,
        )
    })
    .await
    .map_err(|e| ProjectError::InternalError(e.to_string()))
    .and_then(|result| result);
    let iso_path = match build_result {
        Ok(path) => path,
        Err(error) => {
            sqlx::query(
                "UPDATE project_v86_upload_sessions SET status = 'failed', error_message = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            )
            .bind("ISO build failed")
            .bind(&upload_id)
            .execute(&state.project_service.pool)
            .await
            .ok();
            return Err(error);
        }
    };
    let (_zip_size, zip_sha) = sha256_file(Path::new(&temp_key))?;
    let (iso_size, iso_sha) = sha256_file(&iso_path)?;
    let storage_root = state
        .project_demo_config
        .dir
        .join("v86")
        .join("games")
        .join(&upload_id);
    fs::create_dir_all(&storage_root)?;
    let zip_key = format!("v86/games/{upload_id}/{zip_sha}.zip");
    let iso_key = format!("v86/games/{upload_id}/{iso_sha}");
    let zip_path = state.project_demo_config.dir.join(&zip_key);
    fs::rename(&temp_key, &zip_path)?;
    let iso_dir = state.project_demo_config.dir.join(&iso_key);
    fs::create_dir_all(&iso_dir)?;
    fs::rename(&iso_path, iso_dir.join("game.iso"))?;
    let chunk_size = state.project_demo_config.v86_download_chunk_size;
    let iso_source = iso_dir.join("game.iso");
    let parts = iso_dir.join("parts");
    let chunk_count =
        tokio::task::spawn_blocking(move || split_asset(&iso_source, &parts, chunk_size, "iso"))
            .await
            .map_err(|e| ProjectError::InternalError(e.to_string()))??;
    sqlx::query(
        r#"UPDATE project_v86_upload_sessions
           SET status = 'ready', staged_zip_storage_key = ?, staged_zip_sha256 = ?,
               staged_iso_storage_key = ?, staged_iso_sha256 = ?,
               staged_iso_size_bytes = ?, staged_iso_chunk_count = ?,
               updated_at = CURRENT_TIMESTAMP
           WHERE id = ? AND status = 'building'"#,
    )
    .bind(&zip_key)
    .bind(&zip_sha)
    .bind(&iso_key)
    .bind(&iso_sha)
    .bind(iso_size as i64)
    .bind(chunk_count as i64)
    .bind(&upload_id)
    .execute(&state.project_service.pool)
    .await?;
    let _ = original_file_name;
    Ok(Json(ReadyGameUploadResponse {
        upload_id,
        status: "ready".to_string(),
        zip_sha256: zip_sha,
        iso_sha256: iso_sha,
        iso_size_bytes: iso_size,
        chunk_count,
    }))
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
            iso_storage_key, iso_size_bytes, iso_sha256, chunk_size_bytes,
            chunk_count, artifact_revision)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
           ON CONFLICT(project_id) DO UPDATE SET
             system_version_id = excluded.system_version_id,
             manifest_text = excluded.manifest_text,
             manifest_sha256 = excluded.manifest_sha256,
             original_file_name = excluded.original_file_name,
             zip_storage_key = excluded.zip_storage_key,
             zip_size_bytes = excluded.zip_size_bytes,
             zip_sha256 = excluded.zip_sha256,
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
        "SELECT temp_storage_key, staged_zip_storage_key, staged_iso_storage_key, status FROM project_v86_upload_sessions WHERE id = ? AND uploader_id = ?",
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
    let temp: String = row.get("temp_storage_key");
    tokio::fs::remove_file(temp).await.ok();
    if let Some(zip_key) = row.get::<Option<String>, _>("staged_zip_storage_key") {
        tokio::fs::remove_file(state.project_demo_config.dir.join(zip_key))
            .await
            .ok();
    }
    if let Some(iso_key) = row.get::<Option<String>, _>("staged_iso_storage_key") {
        tokio::fs::remove_dir_all(state.project_demo_config.dir.join(iso_key))
            .await
            .ok();
    }
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

pub async fn runtime_descriptor(
    pool: &sqlx::SqlitePool,
    slug: &str,
) -> Result<Option<V86RuntimeDescriptor>, ProjectError> {
    let row = sqlx::query(
        r#"SELECT s.name AS system_name, s.platform_key, v.id AS system_version_id,
                  v.size_bytes AS base_size, v.sha256 AS base_sha,
                  g.iso_size_bytes, g.iso_sha256, g.manifest_sha256,
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
        let iso_sha: String = row.get("iso_sha256");
        V86RuntimeDescriptor {
            platform_key: row.get("platform_key"),
            system_name: row.get("system_name"),
            system_version_id: version_id,
            artifact_revision: row.get("artifact_revision"),
            manifest_sha256: row.get("manifest_sha256"),
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
            base_url: format!("v86/assets/systems/{version_id}/{base_sha}/.img"),
            iso_size_bytes: row.get::<i64, _>("iso_size_bytes") as u64,
            iso_sha256: iso_sha.clone(),
            iso_url: format!("projects/s/{slug}/v86/{iso_sha}/full.iso"),
        }
    }))
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
    if part == ".img" || part.contains('/') || !part.ends_with(".img") {
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
    if part == ".iso" || part.contains('/') || !part.ends_with(".iso") {
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
        let root = std::env::temp_dir().join(format!("v86-parts-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("disk.img");
        fs::write(&source, b"abcdefghij").unwrap();
        let parts = root.join("parts");
        assert_eq!(split_asset(&source, &parts, 4, "img").ok().unwrap(), 3);
        assert_eq!(fs::read(parts.join("0-4.img")).unwrap(), b"abcd");
        assert_eq!(fs::read(parts.join("4-8.img")).unwrap(), b"efgh");
        assert_eq!(fs::read(parts.join("8-12.img")).unwrap(), b"ij\0\0");
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn windows95_manifest_resolves_a_unique_nested_executable() {
        let root = std::env::temp_dir().join(format!("v86-manifest-test-{}", Uuid::new_v4()));
        let game = root.join("Doraemon");
        fs::create_dir_all(&game).unwrap();
        fs::write(game.join("Doraemon.exe"), b"game").unwrap();

        let config = windows95_launcher_config(&root, "exe=Doraemon.exe\nargs=-m")
            .ok()
            .unwrap();
        assert!(config.contains(r"executable=D:\GAME\Doraemon\Doraemon.exe"));
        assert!(config.contains(r"working_directory=D:\GAME\Doraemon"));
        assert!(config.contains("arguments=-m"));
        assert!(config.contains("delay_ms=1000"));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn windows95_strategy_builds_launcher_manifest_and_game_iso() {
        if Command::new("xorriso").arg("-version").output().is_err() {
            return;
        }
        let root = std::env::temp_dir().join(format!("v86-iso-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let game_zip = root.join("game.zip");
        write_zip(&game_zip, &[("Doraemon.exe", b"game")]);
        let manifest =
            "[game]\r\nexecutable=D:\\GAME\\Doraemon.exe\r\narguments=-m\r\ndelay_ms=1000\r\n";
        let iso = build_windows95_iso(&root, "xorriso", "proof", &game_zip, manifest, 10, 1024)
            .ok()
            .unwrap();
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
        assert!(listed.contains("Doraemon.exe"));
        fs::remove_dir_all(root).ok();
    }
}
