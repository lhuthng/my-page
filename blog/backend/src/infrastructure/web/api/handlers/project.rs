use std::{
    cmp::Reverse,
    collections::HashMap,
    fs,
    io::{Cursor, Read, Seek, Write},
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use axum::{
    Extension, Json,
    body::Bytes,
    extract::{Multipart, Path as AxumPath, Query, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;
use uuid::Uuid;
use zip::ZipArchive;

use crate::{
    application::{
        commands::{
            media::{ChangePostCoverCommand, UploadMediaWithoutDescriptionCommand},
            post::{
                CheckSlugCommand, NewPostCommand, PublishCommand, UpdatePostCommand,
                UpdatePostCoverCommand,
            },
            project::{
                GetFeaturedProjectsCommand, GetLatestProjectsCommand, GetProjectBySlugCommand,
                GetProjectDetailsCommand, GetProjectPostIdCommand, NewProjectCommand,
                SetFeaturedProjectCommand, UpdateProjectCommand,
            },
        },
        services::{media::MediaService, post::PostService, project::ProjectService},
    },
    domain::{
        entities::{
            media::MediumDetails,
            project::{Project, ProjectLink, ProjectSnapshot},
            secret::Claims,
        },
        errors::{media::MediaError, project::ProjectError},
    },
    helper::{string::replace_range_unicode, time::normalize_optional_utc_timestamp},
    infrastructure::web::{
        api::handlers::common::{
            CreateCoverUpload, MediumData, apply_created_cover_upload, extract_medium,
            try_collect_create_cover_field,
        },
        api::handlers::v86::{V86RuntimeDescriptor, attach_ready_game_tx, runtime_descriptor},
        server::{AppState, ProjectDemoConfig},
    },
};

#[derive(Deserialize)]
pub struct CheckQuery {
    pub slug: Option<String>,
}

#[derive(Serialize)]
pub struct CheckResponse {
    exists: bool,
}

pub async fn check_project(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CheckQuery>,
) -> Result<impl IntoResponse, ProjectError> {
    if let Some(post_slug) = query.slug {
        let exists = state
            .post_service
            .check_slug(CheckSlugCommand { post_slug })
            .await?;
        Ok(Json(CheckResponse { exists }))
    } else {
        Ok(Json(CheckResponse { exists: true }))
    }
}

#[derive(Deserialize)]
struct ProjectData {
    title: String,
    slug: String,
    excerpt: String,
    content: String,
    tags: Vec<String>,
    links: Vec<ProjectLink>,
    number_of_files: usize,
    demo_type: String,
    demo_width: Option<String>,
    demo_height: Option<String>,
    demo_config: Option<String>,
    demo_url: Option<String>,
    v86_upload_id: Option<String>,
}

#[derive(Deserialize)]
struct ProjectPatchData {
    title: Option<String>,
    slug: Option<String>,
    excerpt: Option<String>,
    content: Option<String>,
    draft: Option<String>,
    tags: Option<Vec<String>>,
    links: Option<Vec<ProjectLink>>,
    number_of_files: usize,
    demo_type: Option<String>,
    demo_width: Option<String>,
    demo_height: Option<String>,
    demo_config: Option<String>,
    demo_url: Option<String>,
    og_image_seconds: Option<i64>,
    v86_upload_id: Option<String>,
}

#[derive(Deserialize)]
pub struct StartJsDosUploadRequest {
    pub file_name: String,
    pub size_bytes: u64,
}

#[derive(Deserialize, Serialize)]
pub struct StartJsDosUploadResponse {
    pub upload_id: String,
    pub chunk_size_bytes: u64,
    pub next_chunk_index: u64,
    pub expected_size_bytes: u64,
}

#[derive(Serialize)]
pub struct JsDosUploadResponse {
    pub received_size_bytes: u64,
    pub next_chunk_index: u64,
}

#[derive(Serialize)]
pub struct CompleteJsDosUploadResponse {
    pub project_id: i64,
    pub file_name: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub bundle_url: String,
}

struct FileData {
    file_name: String,
    bytes: Bytes,
    content_type: String,
}

#[derive(Debug)]
struct ShortNameExtraction {
    short_name: String,
    start: usize,
}

struct ParsedMultipart<T> {
    data: T,
    files: HashMap<usize, FileData>,
    short_names: HashMap<usize, String>,
    demo_zip: Option<Bytes>,
    create_cover: CreateCoverUpload,
}

async fn parse_project_multipart<T: for<'de> Deserialize<'de>>(
    mut multipart: Multipart,
    data_field: &str,
) -> Result<ParsedMultipart<T>, ProjectError> {
    let mut data: Option<T> = None;
    let mut files = HashMap::<usize, FileData>::new();
    let mut short_names = HashMap::<usize, String>::new();
    let mut demo_zip: Option<Bytes> = None;
    let mut create_cover = CreateCoverUpload::default();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ProjectError::InternalError(e.to_string()))?
    {
        let field_name = field
            .name()
            .ok_or(ProjectError::UploadFailed("Empty field found.".to_string()))?
            .to_string();

        if field_name == data_field {
            let bytes = field
                .bytes()
                .await
                .map_err(|e| ProjectError::UploadFailed(e.to_string()))?;
            data = Some(
                serde_json::from_slice::<T>(&bytes)
                    .map_err(|e| ProjectError::UploadFailed(e.to_string()))?,
            );
        } else if field_name == "demo_zip" {
            if demo_zip.is_some() {
                return Err(ProjectError::UploadFailed(
                    "Only one demo zip is allowed.".to_string(),
                ));
            }
            demo_zip = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| ProjectError::UploadFailed(e.to_string()))?,
            );
        } else if let Some(index_str) = field_name.strip_prefix("file_") {
            let index = index_str
                .parse::<usize>()
                .map_err(|_| ProjectError::UploadFailed("Invalid file index".to_string()))?;
            if files.contains_key(&index) {
                return Err(ProjectError::UploadFailed(format!(
                    "Duplicate file index {index}"
                )));
            }
            let file_name = field
                .file_name()
                .ok_or(ProjectError::UploadFailed(
                    "Cannot read file name.".to_string(),
                ))?
                .to_string();
            let content_type = field
                .content_type()
                .ok_or(ProjectError::UploadFailed(format!(
                    "Cannot read content type of {}.",
                    file_name
                )))?
                .to_string();
            let bytes = field
                .bytes()
                .await
                .map_err(|_| ProjectError::UploadFailed(format!("Cannot read {file_name}")))?;
            files.insert(
                index,
                FileData {
                    file_name,
                    bytes,
                    content_type,
                },
            );
        } else if let Some(index_str) = field_name.strip_prefix("short_name_") {
            let index = index_str
                .parse::<usize>()
                .map_err(|_| ProjectError::UploadFailed("Invalid short name index".to_string()))?;
            short_names.insert(
                index,
                field.text().await.map_err(|_| {
                    ProjectError::UploadFailed("Cannot read short name".to_string())
                })?,
            );
        } else if try_collect_create_cover_field(&field_name, field, &mut create_cover).await? {
        }
    }

    Ok(ParsedMultipart {
        data: data.ok_or(ProjectError::UploadFailed(
            "No project data is given.".to_string(),
        ))?,
        files,
        short_names,
        demo_zip,
        create_cover,
    })
}

async fn upload_inline_media(
    state: &AppState,
    uploader_id: i64,
    number_of_files: usize,
    files: &HashMap<usize, FileData>,
    short_name_map: &HashMap<usize, String>,
) -> Result<(), ProjectError> {
    let mut short_names = Vec::<String>::new();
    let mut file_names = Vec::<String>::new();
    let mut content_types = Vec::<String>::new();
    let mut bytes_list = Vec::<Bytes>::new();

    for i in 1..=number_of_files {
        let file = files
            .get(&i)
            .ok_or_else(|| ProjectError::UploadFailed(format!("Cannot locate file_{i}")))?;
        let short_name = short_name_map
            .get(&i)
            .ok_or_else(|| ProjectError::UploadFailed(format!("Cannot locate short_name_{i}")))?;
        short_names.push(short_name.clone());
        file_names.push(file.file_name.clone());
        content_types.push(file.content_type.clone());
        bytes_list.push(file.bytes.clone());
    }

    if number_of_files > 0 {
        state
            .media_service
            .bulk_upload(
                UploadMediaWithoutDescriptionCommand {
                    uploader_id,
                    short_names,
                    number_of_files,
                    file_names,
                    content_types,
                    bytes_list,
                },
                &state.media_config,
            )
            .await?;
    }

    Ok(())
}

fn replace_media_short_names(content: &mut String, usage: &mut HashMap<String, i64>) {
    let syntaxes = [
        Regex::new(r"@(?:\([\d_]+\))?\[[\w-]+:([^\]]+)\]").unwrap(),
        Regex::new(r":::app\s+lottie\s+([^\s]+)").unwrap(),
    ];
    let mut extraction = Vec::<ShortNameExtraction>::new();

    for reg in syntaxes {
        for cap in reg.captures_iter(content) {
            if let Some(matched) = cap.get(1) {
                extraction.push(ShortNameExtraction {
                    short_name: matched.as_str().to_string(),
                    start: matched.start(),
                });
            }
        }
    }
    extraction.sort_by_key(|k| Reverse(k.start));

    for data in extraction {
        let len = usage.len();
        let index = usage
            .entry(data.short_name.clone())
            .or_insert_with(|| len as i64)
            .to_string();
        replace_range_unicode(content, data.start, data.short_name.len(), index);
    }
}

fn has_invalid_component(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    })
}

fn normalized_zip_path(path: &Path) -> Option<PathBuf> {
    if has_invalid_component(path) {
        return None;
    }
    let mut out = PathBuf::new();
    for component in path.components() {
        if let Component::Normal(part) = component {
            out.push(part);
        }
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

fn strip_common_root(paths: &[PathBuf]) -> Option<String> {
    let mut first_root: Option<String> = None;
    for path in paths {
        if path.file_name().is_some_and(|name| name == "index.html")
            && path.parent() == Some(Path::new(""))
        {
            return None;
        }

        let mut components = path.components();
        let first = match components.next() {
            Some(Component::Normal(part)) => part.to_string_lossy().to_string(),
            _ => return None,
        };
        if components.next().is_none() {
            return None;
        }
        match &first_root {
            Some(root) if root != &first => return None,
            None => first_root = Some(first),
            _ => {}
        }
    }
    first_root
}

fn extract_demo_zip(
    config: &ProjectDemoConfig,
    project_id: i64,
    zip_bytes: Bytes,
) -> Result<(), ProjectError> {
    if zip_bytes.len() as u64 > config.max_archive_size {
        return Err(ProjectError::InvalidDemo(
            "Demo archive is too large.".to_string(),
        ));
    }

    let mut archive = ZipArchive::new(Cursor::new(zip_bytes))
        .map_err(|e| ProjectError::InvalidDemo(e.to_string()))?;
    if archive.is_empty() {
        return Err(ProjectError::InvalidDemo(
            "Demo archive is empty.".to_string(),
        ));
    }
    if archive.len() > config.max_files {
        return Err(ProjectError::InvalidDemo(
            "Demo archive contains too many files.".to_string(),
        ));
    }

    let mut paths = Vec::<PathBuf>::new();
    for i in 0..archive.len() {
        let file = archive
            .by_index(i)
            .map_err(|e| ProjectError::InvalidDemo(e.to_string()))?;
        if file.is_dir() {
            continue;
        }
        #[cfg(unix)]
        if file
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(ProjectError::InvalidDemo(
                "Demo archive cannot contain symlinks.".to_string(),
            ));
        }
        let enclosed = file.enclosed_name().ok_or(ProjectError::InvalidDemo(
            "Demo archive contains an unsafe path.".to_string(),
        ))?;
        let normalized = normalized_zip_path(&enclosed).ok_or(ProjectError::InvalidDemo(
            "Demo archive contains an unsafe path.".to_string(),
        ))?;
        paths.push(normalized);
    }

    if paths.is_empty() {
        return Err(ProjectError::InvalidDemo(
            "Demo archive does not contain files.".to_string(),
        ));
    }

    let common_root = strip_common_root(&paths);
    let rel_paths = paths
        .iter()
        .map(|path| {
            common_root
                .as_ref()
                .and_then(|root| path.strip_prefix(root).ok())
                .map(PathBuf::from)
                .unwrap_or_else(|| path.clone())
        })
        .collect::<Vec<_>>();

    if !rel_paths.iter().any(|path| path == Path::new("index.html")) {
        return Err(ProjectError::InvalidDemo(
            "Demo archive must contain index.html.".to_string(),
        ));
    }

    let root = &config.dir;
    fs::create_dir_all(root)?;
    let tmp_dir = root.join(format!(".tmp-{}-{}", project_id, Uuid::new_v4()));
    fs::create_dir_all(&tmp_dir)?;

    let mut extracted_size = 0_u64;
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| ProjectError::InvalidDemo(e.to_string()))?;
        if file.is_dir() {
            continue;
        }
        let enclosed = file.enclosed_name().ok_or(ProjectError::InvalidDemo(
            "Demo archive contains an unsafe path.".to_string(),
        ))?;
        let original = normalized_zip_path(&enclosed).ok_or(ProjectError::InvalidDemo(
            "Demo archive contains an unsafe path.".to_string(),
        ))?;
        let rel = common_root
            .as_ref()
            .and_then(|root| original.strip_prefix(root).ok())
            .map(PathBuf::from)
            .unwrap_or(original);
        if rel.as_os_str().is_empty() || has_invalid_component(&rel) {
            fs::remove_dir_all(&tmp_dir).ok();
            return Err(ProjectError::InvalidDemo(
                "Demo archive contains an unsafe path.".to_string(),
            ));
        }

        extracted_size = extracted_size.saturating_add(file.size());
        if extracted_size > config.max_extracted_size {
            fs::remove_dir_all(&tmp_dir).ok();
            return Err(ProjectError::InvalidDemo(
                "Demo archive expands too large.".to_string(),
            ));
        }

        let out_path = tmp_dir.join(&rel);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| ProjectError::InvalidDemo(e.to_string()))?;
        let mut out = fs::File::create(out_path)?;
        out.write_all(&bytes)?;
    }

    let final_dir = root.join(project_id.to_string());
    if final_dir.exists() {
        fs::remove_dir_all(&final_dir)?;
    }
    fs::rename(&tmp_dir, &final_dir)?;

    Ok(())
}

fn validate_jsdos_bundle(path: &Path, max_files: usize) -> Result<(u64, String), ProjectError> {
    let mut file = fs::File::open(path)?;
    let size = file.metadata()?.len();
    let mut has_manifest = false;
    let mut archive = ZipArchive::new(file.try_clone()?)
        .map_err(|e| ProjectError::InvalidDemo(format!("Invalid js-dos bundle: {e}")))?;
    if archive.is_empty() || archive.len() > max_files {
        return Err(ProjectError::InvalidDemo(
            "Invalid js-dos bundle file count.".to_string(),
        ));
    }
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|e| ProjectError::InvalidDemo(e.to_string()))?;
        if entry.name() == ".jsdos/jsdos.json" {
            has_manifest = true;
        }
        if entry.enclosed_name().is_none() {
            return Err(ProjectError::InvalidDemo(
                "js-dos bundle contains an unsafe path.".to_string(),
            ));
        }
        #[cfg(unix)]
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(ProjectError::InvalidDemo(
                "js-dos bundle cannot contain symlinks.".to_string(),
            ));
        }
    }
    if !has_manifest {
        return Err(ProjectError::InvalidDemo(
            "js-dos bundle must contain .jsdos/jsdos.json.".to_string(),
        ));
    }

    file.rewind()?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| ProjectError::InvalidDemo(e.to_string()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok((size, hex::encode(hasher.finalize())))
}

fn jsdos_temp_path(state: &AppState, upload_id: &str) -> PathBuf {
    state
        .project_demo_config
        .dir
        .join(".uploads")
        .join("jsdos")
        .join(format!("{upload_id}.part"))
}

fn jsdos_storage_key(project_id: i64, sha256: &str) -> String {
    format!("jsdos/{project_id}/{sha256}.jsdos")
}

async fn require_project_owner(
    state: &AppState,
    project_id: i64,
    user_id: i64,
) -> Result<(), ProjectError> {
    let owner: Option<i64> = sqlx::query_scalar(
        "SELECT posts.user_id FROM projects JOIN posts ON posts.id = projects.post_id WHERE projects.id = ?",
    )
    .bind(project_id)
    .fetch_optional(&state.project_service.pool)
    .await?;
    match owner {
        Some(id) if id == user_id => Ok(()),
        Some(_) => Err(ProjectError::Forbidden),
        None => Err(ProjectError::ProjectNotFound),
    }
}

pub async fn start_jsdos_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(project_id): AxumPath<i64>,
    Json(request): Json<StartJsDosUploadRequest>,
) -> Result<impl IntoResponse, ProjectError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| ProjectError::InternalError("Cannot parse user id".to_string()))?;
    require_project_owner(&state, project_id, user_id).await?;
    if !request.file_name.to_ascii_lowercase().ends_with(".jsdos") {
        return Err(ProjectError::InvalidDemo(
            "Only .jsdos bundles are accepted.".to_string(),
        ));
    }
    if request.size_bytes == 0 || request.size_bytes > state.project_demo_config.max_jsdos_size {
        return Err(ProjectError::InvalidDemo(format!(
            "js-dos bundles must be between 1 byte and {} bytes.",
            state.project_demo_config.max_jsdos_size
        )));
    }

    let upload_id = Uuid::new_v4().to_string();
    let temp_path = jsdos_temp_path(&state, &upload_id);
    if let Some(parent) = temp_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::File::create(&temp_path).await?;
    let ttl_hours = state.project_demo_config.upload_session_ttl_hours;
    sqlx::query(
        r#"INSERT INTO project_jsdos_upload_sessions
           (id, project_id, uploader_id, original_file_name, expected_size_bytes,
            chunk_size_bytes, temp_storage_key, expires_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now', ?))"#,
    )
    .bind(&upload_id)
    .bind(project_id)
    .bind(user_id)
    .bind(&request.file_name)
    .bind(request.size_bytes as i64)
    .bind(state.project_demo_config.jsdos_chunk_size as i64)
    .bind(temp_path.to_string_lossy().to_string())
    .bind(format!("+{ttl_hours} hours"))
    .execute(&state.project_service.pool)
    .await?;

    Ok(Json(StartJsDosUploadResponse {
        upload_id,
        chunk_size_bytes: state.project_demo_config.jsdos_chunk_size,
        next_chunk_index: 0,
        expected_size_bytes: request.size_bytes,
    }))
}

pub async fn append_jsdos_chunk(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath((_project_id, upload_id, chunk_index)): AxumPath<(i64, String, u64)>,
    body: Bytes,
) -> Result<impl IntoResponse, ProjectError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| ProjectError::InternalError("Cannot parse user id".to_string()))?;
    let row: Option<(i64, i64, i64, i64, i64, String, String)> = sqlx::query_as(
        "SELECT project_id, uploader_id, expected_size_bytes, received_size_bytes, next_chunk_index, temp_storage_key, status FROM project_jsdos_upload_sessions WHERE id = ?",
    )
    .bind(&upload_id)
    .fetch_optional(&state.project_service.pool)
    .await?;
    let (project_id, uploader_id, expected, received, next, temp_key, status) =
        row.ok_or(ProjectError::ProjectNotFound)?;
    if uploader_id != user_id || project_id != _project_id {
        return Err(ProjectError::Forbidden);
    }
    if status != "active" || chunk_index != next as u64 {
        return Err(ProjectError::InvalidDemo(
            "Invalid or out-of-order js-dos upload chunk.".to_string(),
        ));
    }
    let chunk_size = state.project_demo_config.jsdos_chunk_size;
    if body.is_empty()
        || body.len() as u64 > chunk_size
        || received as u64 + body.len() as u64 > expected as u64
    {
        return Err(ProjectError::InvalidDemo(
            "Invalid js-dos upload chunk size.".to_string(),
        ));
    }
    let mut file = tokio::fs::OpenOptions::new()
        .append(true)
        .open(&temp_key)
        .await?;
    file.write_all(&body).await?;
    file.flush().await?;
    let received_size = received as u64 + body.len() as u64;
    sqlx::query(
        "UPDATE project_jsdos_upload_sessions SET received_size_bytes = ?, next_chunk_index = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(received_size as i64)
    .bind((chunk_index + 1) as i64)
    .bind(&upload_id)
    .execute(&state.project_service.pool)
    .await?;
    Ok(Json(JsDosUploadResponse {
        received_size_bytes: received_size,
        next_chunk_index: chunk_index + 1,
    }))
}

pub async fn complete_jsdos_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath((project_id, upload_id)): AxumPath<(i64, String)>,
) -> Result<impl IntoResponse, ProjectError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| ProjectError::InternalError("Cannot parse user id".to_string()))?;
    require_project_owner(&state, project_id, user_id).await?;
    let row: Option<(String, i64, i64, String, String)> = sqlx::query_as(
        "SELECT original_file_name, expected_size_bytes, received_size_bytes, temp_storage_key, status FROM project_jsdos_upload_sessions WHERE id = ? AND project_id = ? AND uploader_id = ?",
    )
    .bind(&upload_id)
    .bind(project_id)
    .bind(user_id)
    .fetch_optional(&state.project_service.pool)
    .await?;
    let (file_name, expected, received, temp_key, status) =
        row.ok_or(ProjectError::ProjectNotFound)?;
    if status != "active" || expected != received {
        return Err(ProjectError::InvalidDemo(
            "js-dos upload is incomplete.".to_string(),
        ));
    }
    let temp_path = PathBuf::from(&temp_key);
    let max_files = state.project_demo_config.max_files;
    let (size, sha256) =
        tokio::task::spawn_blocking(move || validate_jsdos_bundle(&temp_path, max_files))
            .await
            .map_err(|e| ProjectError::InternalError(e.to_string()))??;
    let storage_key = jsdos_storage_key(project_id, &sha256);
    let final_path = state.project_demo_config.dir.join(&storage_key);
    if let Some(parent) = final_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::rename(&temp_key, &final_path).await?;

    let mut tx = state.project_service.pool.begin().await?;
    sqlx::query(
        "INSERT INTO project_jsdos_bundles (project_id, storage_key, original_file_name, size_bytes, sha256) VALUES (?, ?, ?, ?, ?) ON CONFLICT(project_id) DO UPDATE SET storage_key = excluded.storage_key, original_file_name = excluded.original_file_name, size_bytes = excluded.size_bytes, sha256 = excluded.sha256, updated_at = CURRENT_TIMESTAMP",
    )
    .bind(project_id)
    .bind(&storage_key)
    .bind(&file_name)
    .bind(size as i64)
    .bind(&sha256)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE projects SET demo_type = 'jsdos', demo_url = NULL, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(project_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE project_jsdos_upload_sessions SET status = 'completed', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&upload_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(Json(CompleteJsDosUploadResponse {
        project_id,
        file_name,
        size_bytes: size,
        sha256,
        bundle_url: format!("projects/id/{project_id}/jsdos"),
    }))
}

pub async fn abort_jsdos_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath((project_id, upload_id)): AxumPath<(i64, String)>,
) -> Result<StatusCode, ProjectError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| ProjectError::InternalError("Cannot parse user id".to_string()))?;
    let temp_key: Option<String> = sqlx::query_scalar(
        "SELECT temp_storage_key FROM project_jsdos_upload_sessions WHERE id = ? AND project_id = ? AND uploader_id = ? AND status = 'active'",
    )
    .bind(&upload_id)
    .bind(project_id)
    .bind(user_id)
    .fetch_optional(&state.project_service.pool)
    .await?;
    if let Some(temp_key) = temp_key {
        sqlx::query("UPDATE project_jsdos_upload_sessions SET status = 'aborted', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(&upload_id)
            .execute(&state.project_service.pool)
            .await?;
        tokio::fs::remove_file(temp_key).await.ok();
    }
    Ok(StatusCode::NO_CONTENT)
}

fn normalize_links(links: Vec<ProjectLink>) -> Vec<ProjectLink> {
    links
        .into_iter()
        .filter(|link| !link.label.trim().is_empty() && !link.url.trim().is_empty())
        .map(|link| ProjectLink {
            label: link.label.trim().to_string(),
            url: link.url.trim().to_string(),
        })
        .collect()
}

/// Collects the per-variant autorun CD storage keys for a project. Called
/// *before* the variant rows are deleted so cleanup can reach them.
async fn v86_variant_iso_keys(state: &AppState, project_id: i64) -> Vec<String> {
    sqlx::query_scalar("SELECT iso_storage_key FROM project_v86_variants WHERE project_id = ?")
        .bind(project_id)
        .fetch_all(&state.project_service.pool)
        .await
        .unwrap_or_default()
}

/// Best-effort cleanup of a v86 game's R2 objects, its local mirror, and the
/// per-user cloud saves. Content-addressed objects are only removed once no
/// other project references them. `variant_iso_keys` are the per-variant autorun
/// CD keys (from project_v86_variants) that are about to be removed alongside
/// the primary `keys` (from project_v86_games).
async fn delete_v86_game_objects(
    state: &AppState,
    project_id: i64,
    keys: &(Option<String>, Option<String>, Option<String>),
    variant_iso_keys: &[String],
) {
    let (zip_key, iso_key, disk_key) = keys;
    if let Some(r2) = &state.r2 {
        if let Some(zip) = zip_key {
            let refs: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM project_v86_games WHERE zip_storage_key = ?",
            )
            .bind(zip)
            .fetch_one(&state.project_service.pool)
            .await
            .unwrap_or(1);
            if refs == 0 {
                let _ = r2.delete_object(zip).await;
            }
        }
        if let Some(disk) = disk_key {
            let refs: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM project_v86_games WHERE disk_storage_key = ?",
            )
            .bind(disk)
            .fetch_one(&state.project_service.pool)
            .await
            .unwrap_or(1);
            if refs == 0 {
                let _ = r2.delete_prefix(disk).await;
            }
        }
        let mut iso_keys = Vec::new();
        if let Some(iso) = iso_key {
            iso_keys.push(iso.clone());
        }
        iso_keys.extend(variant_iso_keys.iter().cloned());
        for iso in iso_keys {
            let refs_a: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM project_v86_games WHERE iso_storage_key = ?",
            )
            .bind(&iso)
            .fetch_one(&state.project_service.pool)
            .await
            .unwrap_or(1);
            let refs_v: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM project_v86_variants WHERE iso_storage_key = ?",
            )
            .bind(&iso)
            .fetch_one(&state.project_service.pool)
            .await
            .unwrap_or(1);
            if refs_a + refs_v == 0 {
                let _ = r2.delete_prefix(&iso).await;
            }
        }
    }
    if let Some(zip) = zip_key {
        let _ = tokio::fs::remove_file(state.project_demo_config.dir.join(zip)).await;
    }
    if let Some(disk) = disk_key {
        let _ = tokio::fs::remove_dir_all(state.project_demo_config.dir.join(disk)).await;
    }
    if let Some(iso) = iso_key {
        let _ = tokio::fs::remove_dir_all(state.project_demo_config.dir.join(iso)).await;
    }
    // Cloud saves are per-user; remove them along with the project.
    let save_keys: Vec<String> =
        sqlx::query_scalar("SELECT storage_key FROM v86_saves WHERE project_id = ?")
            .bind(project_id)
            .fetch_all(&state.project_service.pool)
            .await
            .unwrap_or_default();
    if let Some(r2) = &state.r2 {
        for key in save_keys.iter() {
            let _ = r2.delete_object(key).await;
        }
    } else {
        for key in save_keys.iter() {
            let _ = tokio::fs::remove_file(state.project_demo_config.dir.join(key)).await;
        }
    }
    let _ = sqlx::query("DELETE FROM v86_saves WHERE project_id = ?")
        .bind(project_id)
        .execute(&state.project_service.pool)
        .await;
}

pub async fn get_jsdos_bundle(
    State(state): State<Arc<AppState>>,
    AxumPath(slug): AxumPath<String>,
) -> Result<Response, ProjectError> {
    let storage_key: Option<String> = sqlx::query_scalar(
        "SELECT b.storage_key FROM project_jsdos_bundles b JOIN projects p ON p.id = b.project_id JOIN posts ON posts.id = p.post_id WHERE posts.slug = ? AND posts.status = 'published' AND p.demo_type = 'jsdos'",
    )
    .bind(&slug)
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
            "Invalid js-dos storage key".to_string(),
        ));
    }
    let path = state.project_demo_config.dir.join(&storage_key);
    let file = tokio::fs::File::open(&path)
        .await
        .map_err(|_| ProjectError::ProjectNotFound)?;
    let metadata = file.metadata().await?;
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
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response.headers_mut().insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&metadata.len().to_string()).unwrap(),
    );
    Ok(response)
}

#[axum::debug_handler]
pub async fn new_project(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    multipart: Multipart,
) -> Result<impl IntoResponse, ProjectError> {
    let uploader_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| ProjectError::InternalError("Cannot parse id".to_string()))?;
    let parsed = parse_project_multipart::<ProjectData>(multipart, "project_data").await?;
    let mut data = parsed.data;

    if state
        .post_service
        .check_slug(CheckSlugCommand {
            post_slug: data.slug.clone(),
        })
        .await?
    {
        return Err(ProjectError::InvalidDemo(format!(
            "The project slug '{}' is already in use.",
            data.slug
        )));
    }

    let demo_zip = parsed.demo_zip;
    let create_cover = parsed.create_cover;
    let has_demo_url = data.demo_url.as_ref().is_some_and(|u| !u.trim().is_empty());
    match data.demo_type.as_str() {
        "none" => {
            if has_demo_url || demo_zip.is_some() {
                return Err(ProjectError::InvalidDemo(
                    "Demo attachments are not accepted for projects without demos.".to_string(),
                ));
            }
        }
        "html5" | "webgl" => {
            if has_demo_url {
                return Err(ProjectError::InvalidDemo(format!(
                    "Demo URL is not accepted for {} projects.",
                    data.demo_type
                )));
            }
            if demo_zip.is_none() {
                return Err(ProjectError::InvalidDemo(format!(
                    "Demo zip is required for {} projects.",
                    data.demo_type
                )));
            }
        }
        "embed" | "download" | "video" => {
            if !has_demo_url {
                return Err(ProjectError::InvalidDemo(format!(
                    "Demo URL is required for {} projects.",
                    data.demo_type
                )));
            }
        }
        "jsdos" => {
            if has_demo_url || demo_zip.is_some() {
                return Err(ProjectError::InvalidDemo(
                    "js-dos bundles must be uploaded through the js-dos upload endpoint."
                        .to_string(),
                ));
            }
        }
        "v86" => {
            if has_demo_url || demo_zip.is_some() {
                return Err(ProjectError::InvalidDemo(
                    "v86 games must be uploaded through the v86 package endpoint.".to_string(),
                ));
            }
            if data.v86_upload_id.is_none() {
                return Err(ProjectError::InvalidDemo(
                    "A completed v86 game package is required.".to_string(),
                ));
            }
        }
        _ => {
            return Err(ProjectError::InvalidDemo(format!(
                "Unsupported demo type: {}",
                data.demo_type
            )));
        }
    }

    upload_inline_media(
        &state,
        uploader_id,
        data.number_of_files,
        &parsed.files,
        &parsed.short_names,
    )
    .await?;

    let mut media_usage = HashMap::<String, i64>::new();
    replace_media_short_names(&mut data.content, &mut media_usage);

    let post_id = state
        .post_service
        .new_post(NewPostCommand {
            user_id: uploader_id,
            title: data.title,
            slug: data.slug,
            excerpt: data.excerpt,
            content: data.content,
            tags: data.tags,
            cover_media: None,
            media_usage,
            content_kind: "project".to_string(),
        })
        .await?;

    let project_result = state
        .project_service
        .new_project(NewProjectCommand {
            post_id,
            demo_type: data.demo_type,
            demo_entry_path: "index.html".to_string(),
            demo_width: data.demo_width,
            demo_height: data.demo_height,
            demo_config: data.demo_config,
            demo_url: data.demo_url,
            demo_url_dir: state
                .project_demo_config
                .dir
                .to_str()
                .unwrap_or("")
                .to_string(),
            links: normalize_links(data.links),
        })
        .await;
    let project_id = match project_result {
        Ok(project_id) => project_id,
        Err(error) => {
            sqlx::query("DELETE FROM posts WHERE id = ?")
                .bind(post_id)
                .execute(&state.project_service.pool)
                .await
                .ok();
            return Err(error.into());
        }
    };

    if let Some(upload_id) = data.v86_upload_id.as_deref() {
        let mut tx = state.project_service.pool.begin().await?;
        let attach_result = attach_ready_game_tx(
            &mut tx,
            project_id,
            uploader_id,
            upload_id,
            state.project_demo_config.v86_download_chunk_size,
        )
        .await;
        if let Err(error) = attach_result {
            tx.rollback().await.ok();
            sqlx::query("DELETE FROM posts WHERE id = ?")
                .bind(post_id)
                .execute(&state.project_service.pool)
                .await
                .ok();
            return Err(error);
        }
        tx.commit().await?;
    }

    if let Some(zip) = demo_zip {
        if let Err(err) = extract_demo_zip(&state.project_demo_config, project_id, zip) {
            return Err(err);
        }
    }

    apply_created_cover_upload(&state, uploader_id, post_id, create_cover).await?;

    Ok(Json(
        serde_json::json!({ "id": project_id, "post_id": post_id }),
    ))
}

pub async fn delete_project_draft(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(project_id): AxumPath<i64>,
) -> Result<StatusCode, ProjectError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| ProjectError::InternalError("Cannot parse id".to_string()))?;
    require_project_owner(&state, project_id, user_id).await?;
    let mut tx = state.project_service.pool.begin().await?;
    let post_id: Option<i64> = sqlx::query_scalar("SELECT post_id FROM projects WHERE id=?")
        .bind(project_id)
        .fetch_optional(&mut *tx)
        .await?;
    let post_id = post_id.ok_or(ProjectError::ProjectNotFound)?;
    let status: String = sqlx::query_scalar("SELECT status FROM posts WHERE id=?")
        .bind(post_id)
        .fetch_one(&mut *tx)
        .await?;
    if status != "draft" {
        return Err(ProjectError::InvalidDemo(
            "Only draft projects can be automatically cleaned up.".to_string(),
        ));
    }
    let v86_storage = sqlx::query_as::<_, (Option<String>, Option<String>, Option<String>)>(
        "SELECT zip_storage_key, iso_storage_key, disk_storage_key FROM project_v86_games WHERE project_id = ?",
    )
    .bind(project_id)
    .fetch_optional(&mut *tx)
    .await?;
    let v86_variant_keys = v86_variant_iso_keys(&state, project_id).await;
    sqlx::query("DELETE FROM posts WHERE id=?")
        .bind(post_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    if let Some(keys) = v86_storage {
        delete_v86_game_objects(&state, project_id, &keys, &v86_variant_keys).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

#[axum::debug_handler]
pub async fn update_project(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(project_id): AxumPath<i64>,
    multipart: Multipart,
) -> Result<impl IntoResponse, ProjectError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| ProjectError::InternalError("Cannot parse id".to_string()))?;
    let parsed = parse_project_multipart::<ProjectPatchData>(multipart, "project_data").await?;
    let mut data = parsed.data;
    let current_demo_type: String =
        sqlx::query_scalar("SELECT demo_type FROM projects WHERE id = ?")
            .bind(project_id)
            .fetch_optional(&state.project_service.pool)
            .await?
            .ok_or(ProjectError::ProjectNotFound)?;
    let effective_demo_type = data
        .demo_type
        .as_deref()
        .unwrap_or(current_demo_type.as_str())
        .to_string();
    let old_v86_storage = sqlx::query_as::<_, (Option<String>, Option<String>, Option<String>)>(
        "SELECT zip_storage_key, iso_storage_key, disk_storage_key FROM project_v86_games WHERE project_id = ?",
    )
    .bind(project_id)
    .fetch_one(&state.project_service.pool)
    .await?;
    let old_v86_variant_keys = v86_variant_iso_keys(&state, project_id).await;

    let has_demo_url = data.demo_url.as_ref().is_some_and(|u| !u.trim().is_empty());
    let has_demo_attachments = parsed.demo_zip.is_some() || has_demo_url;
    if let Some(ref demo_type) = data.demo_type {
        if has_demo_attachments {
            match demo_type.as_str() {
                "none" => {
                    return Err(ProjectError::InvalidDemo(
                        "Demo attachments are not accepted for projects without demos.".to_string(),
                    ));
                }
                "html5" | "webgl" => {
                    if data.demo_url.is_some() {
                        return Err(ProjectError::InvalidDemo(format!(
                            "Demo URL is not accepted for {} projects.",
                            demo_type
                        )));
                    }
                    if parsed.demo_zip.is_none() {
                        return Err(ProjectError::InvalidDemo(format!(
                            "Demo zip is required for {} projects.",
                            demo_type
                        )));
                    }
                }
                "embed" | "download" | "video" => {
                    let has_url = data.demo_url.as_ref().is_some_and(|u| !u.trim().is_empty());
                    if !has_url {
                        return Err(ProjectError::InvalidDemo(format!(
                            "Demo URL is required for {} projects.",
                            demo_type
                        )));
                    }
                }
                "jsdos" => {
                    if has_demo_url || parsed.demo_zip.is_some() {
                        return Err(ProjectError::InvalidDemo(
                            "js-dos bundles must be uploaded through the js-dos upload endpoint."
                                .to_string(),
                        ));
                    }
                }
                "v86" => {
                    if has_demo_url || parsed.demo_zip.is_some() {
                        return Err(ProjectError::InvalidDemo(
                            "v86 games must be uploaded through the v86 package endpoint."
                                .to_string(),
                        ));
                    }
                }
                _ => {
                    return Err(ProjectError::InvalidDemo(format!(
                        "Unsupported demo type: {}",
                        demo_type
                    )));
                }
            }
        }
    } else if has_demo_attachments {
        return Err(ProjectError::InvalidDemo(
            "Demo type is required when providing demo attachments.".to_string(),
        ));
    }

    let post_id = state
        .project_service
        .get_project_post_id(GetProjectPostIdCommand {
            project_id,
            required_author_id: Some(user_id),
        })
        .await?;

    upload_inline_media(
        &state,
        user_id,
        data.number_of_files,
        &parsed.files,
        &parsed.short_names,
    )
    .await?;

    if data.content.as_ref().xor(data.draft.as_ref()).is_some() {
        return Err(ProjectError::UploadFailed(
            "Content and Draft must both present or both absent.".to_string(),
        ));
    }

    let mut media_usage = None;
    if let Some(content) = data.content.as_mut()
        && let Some(draft) = data.draft.as_mut()
    {
        let mut usage = HashMap::<String, i64>::new();
        replace_media_short_names(content, &mut usage);
        replace_media_short_names(draft, &mut usage);
        media_usage = Some(usage);
    }

    state
        .post_service
        .update_post(UpdatePostCommand {
            user_id,
            post_id,
            title: data.title,
            slug: data.slug,
            excerpt: data.excerpt,
            content: data.content,
            draft: data.draft,
            tags: data.tags,
            media_usage,
        })
        .await?;

    let mut demo_url = data.demo_url.filter(|u| !u.trim().is_empty());
    if parsed.demo_zip.is_some() {
        let local_demo_url = state
            .project_demo_config
            .dir
            .join(project_id.to_string())
            .join("index.html");
        demo_url = Some(local_demo_url.to_str().unwrap_or("").to_string());
    }

    let keeps_jsdos_bundle = effective_demo_type == "jsdos";
    let keeps_v86_game = effective_demo_type == "v86";

    state
        .project_service
        .update_project(UpdateProjectCommand {
            project_id,
            user_id,
            demo_type: data.demo_type,
            demo_entry_path: None,
            demo_width: data.demo_width,
            demo_height: data.demo_height,
            demo_config: data.demo_config,
            demo_url,
            links: data.links.map(normalize_links),
        })
        .await?;

    if let Some(upload_id) = data.v86_upload_id.as_deref() {
        if !keeps_v86_game {
            return Err(ProjectError::InvalidDemo(
                "A v86 package cannot be attached to a non-v86 project.".to_string(),
            ));
        }
        let mut tx = state.project_service.pool.begin().await?;
        attach_ready_game_tx(
            &mut tx,
            project_id,
            user_id,
            upload_id,
            state.project_demo_config.v86_download_chunk_size,
        )
        .await?;
        tx.commit().await?;
        if old_v86_storage.0.is_some() || old_v86_storage.1.is_some() || old_v86_storage.2.is_some() {
            delete_v86_game_objects(&state, project_id, &old_v86_storage, &old_v86_variant_keys).await;
        }
    }

    if !keeps_jsdos_bundle {
        if let Some(storage_key) = sqlx::query_scalar::<_, String>(
            "SELECT storage_key FROM project_jsdos_bundles WHERE project_id = ?",
        )
        .bind(project_id)
        .fetch_optional(&state.project_service.pool)
        .await?
        {
            sqlx::query("DELETE FROM project_jsdos_bundles WHERE project_id = ?")
                .bind(project_id)
                .execute(&state.project_service.pool)
                .await?;
            tokio::fs::remove_file(state.project_demo_config.dir.join(storage_key))
                .await
                .ok();
        }
    }

    if !keeps_v86_game {
        sqlx::query("DELETE FROM project_v86_games WHERE project_id = ?")
            .bind(project_id)
            .execute(&state.project_service.pool)
            .await?;
        if old_v86_storage.0.is_some() || old_v86_storage.1.is_some() || old_v86_storage.2.is_some() {
            delete_v86_game_objects(&state, project_id, &old_v86_storage, &old_v86_variant_keys).await;
        }
    }

    if let Some(zip) = parsed.demo_zip {
        extract_demo_zip(&state.project_demo_config, project_id, zip)?;
    }

    if data.og_image_seconds.is_some() {
        state
            .post_service
            .update_post_cover(UpdatePostCoverCommand {
                user_id,
                post_id,
                og_image_seconds: data.og_image_seconds,
            })
            .await?;
    }

    Ok(())
}

#[axum::debug_handler]
pub async fn publish_project(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(project_id): AxumPath<i64>,
) -> Result<(), ProjectError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| ProjectError::InternalError("Cannot parse id".to_string()))?;
    let post_id = state
        .project_service
        .get_project_post_id(GetProjectPostIdCommand {
            project_id,
            required_author_id: Some(user_id),
        })
        .await?;
    let demo_type: String = sqlx::query_scalar("SELECT demo_type FROM projects WHERE id = ?")
        .bind(project_id)
        .fetch_one(&state.project_service.pool)
        .await?;
    if demo_type == "jsdos" {
        let has_bundle: Option<i64> =
            sqlx::query_scalar("SELECT project_id FROM project_jsdos_bundles WHERE project_id = ?")
                .bind(project_id)
                .fetch_optional(&state.project_service.pool)
                .await?;
        if has_bundle.is_none() {
            return Err(ProjectError::InvalidDemo(
                "A completed js-dos bundle is required before publishing.".to_string(),
            ));
        }
    }
    if demo_type == "v86" {
        let has_game: Option<i64> =
            sqlx::query_scalar("SELECT project_id FROM project_v86_games WHERE project_id = ?")
                .bind(project_id)
                .fetch_optional(&state.project_service.pool)
                .await?;
        if has_game.is_none() {
            return Err(ProjectError::InvalidDemo(
                "A completed v86 game artifact is required before publishing.".to_string(),
            ));
        }
    }
    state
        .post_service
        .publish(PublishCommand { user_id, post_id })
        .await?;
    Ok(())
}

#[derive(Serialize)]
pub struct ProjectResponse {
    pub id: i64,
    pub post_id: i64,
    pub title: String,
    pub slug: String,
    pub tags: Vec<String>,
    pub author_name: String,
    pub author_slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_avatar_url: Option<String>,
    pub excerpt: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft: Option<String>,
    pub medium_urls: Vec<String>,
    pub medium_short_names: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_media_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub og_image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_video_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_video_type: Option<String>,
    pub og_image_seconds: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    pub demo_type: String,
    pub demo_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_demo_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demo_width: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demo_height: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demo_config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsdos_bundle_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsdos_bundle_file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsdos_bundle_size_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v86_runtime: Option<V86RuntimeDescriptor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v86_system_version_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v86_manifest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v86_artifact_revision: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v86_game_file_name: Option<String>,
    pub links: Vec<ProjectLink>,
    pub is_owner: bool,
}

fn project_response(project: Project, include_draft: bool) -> ProjectResponse {
    let mut demo_url = project.demo.demo_url.clone().unwrap_or_default();
    let raw_demo_url = if demo_url.contains("://") {
        Some(demo_url.clone())
    } else {
        None
    };
    let jsdos_bundle_url = project
        .demo
        .jsdos_bundle
        .as_ref()
        .map(|_| format!("projects/s/{}/jsdos", project.slug));
    let jsdos_bundle_file_name = project
        .demo
        .jsdos_bundle
        .as_ref()
        .map(|bundle| bundle.original_file_name.clone());
    let jsdos_bundle_size_bytes = project
        .demo
        .jsdos_bundle
        .as_ref()
        .map(|bundle| bundle.size_bytes);
    if project.demo.demo_type == "jsdos" && jsdos_bundle_url.is_some() {
        demo_url = format!("projects/s/{}/jsdos", project.slug);
    }

    ProjectResponse {
        demo_url,
        id: project.id,
        post_id: project.post_id,
        title: project.title,
        slug: project.slug,
        tags: project.tags,
        author_name: project.author_name,
        author_slug: project.author_slug,
        author_avatar_url: project.author_avatar_url,
        excerpt: project.excerpt,
        content: project.content,
        draft: include_draft.then_some(project.draft),
        medium_urls: project.medium_urls,
        medium_short_names: project.medium_short_names,
        cover_url: project.cover_url,
        cover_media_type: project.cover_media_type,
        og_image_url: project.og_image_url,
        cover_video_url: project.cover_video_url,
        cover_video_type: project.cover_video_type,
        og_image_seconds: project.og_image_seconds,
        published_at: normalize_optional_utc_timestamp(project.published_at),
        updated_at: normalize_optional_utc_timestamp(project.updated_at),
        demo_type: project.demo.demo_type,
        raw_demo_url,
        demo_width: project.demo.width,
        demo_height: project.demo.height,
        demo_config: project.demo.config,
        jsdos_bundle_url,
        jsdos_bundle_file_name,
        jsdos_bundle_size_bytes,
        v86_runtime: None,
        v86_system_version_id: None,
        v86_manifest: None,
        v86_artifact_revision: None,
        v86_game_file_name: None,
        links: project.links,
        is_owner: project.is_owner,
    }
}

#[derive(Deserialize)]
pub struct GetProjectQuery {
    pub with_draft: Option<bool>,
}

pub async fn get_project_by_slug(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    AxumPath(slug): AxumPath<String>,
    Query(query): Query<GetProjectQuery>,
) -> Result<impl IntoResponse, ProjectError> {
    let mut as_id = None;
    let include_draft = query.with_draft.unwrap_or(false);
    if include_draft && let Some(claims) = opt_claims {
        as_id = Some(
            claims
                .user_id
                .parse::<i64>()
                .map_err(|_| ProjectError::InternalError("Cannot parse id".to_string()))?,
        );
    }

    let project = state
        .project_service
        .get_project_by_slug(GetProjectBySlugCommand { slug, as_id })
        .await?;
    let mut response = project_response(project, include_draft);
    if response.demo_type == "v86" {
        response.v86_runtime = runtime_descriptor(
            &state.project_service.pool,
            &response.slug,
            state.config.r2_public_url.as_deref(),
        )
        .await?;
        let game = sqlx::query(
            "SELECT system_version_id, manifest_text, artifact_revision, original_file_name FROM project_v86_games WHERE project_id = ?",
        )
        .bind(response.id)
        .fetch_optional(&state.project_service.pool)
        .await?;
        if let Some(game) = game {
            use sqlx::Row;
            response.v86_system_version_id = Some(game.get("system_version_id"));
            response.v86_manifest = Some(game.get("manifest_text"));
            response.v86_artifact_revision = Some(game.get("artifact_revision"));
            response.v86_game_file_name = Some(game.get("original_file_name"));
        }
    }
    Ok(Json(response))
}

pub async fn get_project_details(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(project_id): AxumPath<i64>,
) -> Result<impl IntoResponse, ProjectError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| ProjectError::InternalError("Cannot parse id".to_string()))?;
    let is_admin = claims.role == "admin";
    let project = state
        .project_service
        .get_project_details(GetProjectDetailsCommand {
            project_id,
            viewing_user_id: user_id,
            required_author_id: if is_admin { None } else { Some(user_id) },
        })
        .await?;
    let mut response = project_response(project, true);
    if response.demo_type == "v86" {
        let game = sqlx::query(
            "SELECT system_version_id, manifest_text, artifact_revision, original_file_name FROM project_v86_games WHERE project_id = ?",
        )
        .bind(response.id)
        .fetch_optional(&state.project_service.pool)
        .await?;
        if let Some(game) = game {
            use sqlx::Row;
            response.v86_system_version_id = Some(game.get("system_version_id"));
            response.v86_manifest = Some(game.get("manifest_text"));
            response.v86_artifact_revision = Some(game.get("artifact_revision"));
            response.v86_game_file_name = Some(game.get("original_file_name"));
        }
    }
    Ok(Json(response))
}

#[derive(Deserialize)]
pub struct LatestProjectsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct ProjectStats {
    pub views: i64,
    pub likes: i64,
    pub comments: i64,
}

#[derive(Serialize)]
pub struct ProjectCard {
    pub id: i64,
    pub post_id: i64,
    pub title: String,
    pub slug: String,
    pub tag_names: Vec<String>,
    pub tag_slugs: Vec<String>,
    pub excerpt: String,
    pub author_name: String,
    pub author_slug: String,
    pub demo_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub cover_media_type: Option<String>,
    pub stats: ProjectStats,
    pub reading_time_minutes: i64,
}

impl From<ProjectSnapshot> for ProjectCard {
    fn from(value: ProjectSnapshot) -> Self {
        ProjectCard {
            id: value.id,
            post_id: value.post_id,
            title: value.title,
            slug: value.slug,
            tag_names: value.tag_names,
            tag_slugs: value.tag_slugs,
            excerpt: value.excerpt,
            author_name: value.author_name,
            author_slug: value.author_slug,
            demo_type: value.demo_type,
            url: value.url,
            cover_media_type: value.cover_media_type,
            stats: ProjectStats {
                views: value.stats.views,
                likes: value.stats.likes,
                comments: value.stats.comments,
            },
            reading_time_minutes: value.reading_time_minutes,
        }
    }
}

#[derive(Serialize)]
pub struct LatestProjectsResponse {
    pub projects: Vec<ProjectCard>,
    pub has_more: bool,
}

pub async fn get_latest_projects(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LatestProjectsQuery>,
) -> Result<impl IntoResponse, ProjectError> {
    let projects = state
        .project_service
        .get_latest_project_snapshots(GetLatestProjectsCommand {
            limit: query.limit.unwrap_or(24),
            offset: query.offset.unwrap_or(0),
            public_only: true,
            required_author_id: None,
        })
        .await?;
    Ok(Json(LatestProjectsResponse {
        projects: projects.projects.into_iter().map(Into::into).collect(),
        has_more: projects.has_more,
    }))
}

#[derive(Deserialize)]
pub struct FeaturedProjectsQuery {
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct FeaturedProjectsResponse {
    pub featured_projects: Vec<ProjectCard>,
    pub has_more: bool,
}

pub async fn get_featured_projects(
    State(state): State<Arc<AppState>>,
    Query(query): Query<FeaturedProjectsQuery>,
) -> Result<impl IntoResponse, ProjectError> {
    let projects = state
        .project_service
        .get_featured_project_snapshots(GetFeaturedProjectsCommand {
            limit: query.limit.unwrap_or(5),
        })
        .await?;

    Ok(Json(FeaturedProjectsResponse {
        featured_projects: projects.into_iter().map(Into::into).collect(),
        has_more: false,
    }))
}

#[derive(Deserialize)]
pub struct SetProjectFeaturedBody {
    pub is_featured: bool,
}

pub async fn set_project_featured(
    State(state): State<Arc<AppState>>,
    AxumPath(project_id): AxumPath<i64>,
    Json(body): Json<SetProjectFeaturedBody>,
) -> Result<impl IntoResponse, ProjectError> {
    state
        .project_service
        .set_project_featured(SetFeaturedProjectCommand {
            project_id,
            is_featured: body.is_featured,
        })
        .await?;
    Ok(())
}

pub async fn get_all_projects(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<LatestProjectsQuery>,
) -> Result<impl IntoResponse, ProjectError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| ProjectError::InternalError("Cannot parse id".to_string()))?;
    let projects = state
        .project_service
        .get_latest_project_snapshots(GetLatestProjectsCommand {
            limit: query.limit.unwrap_or(100),
            offset: query.offset.unwrap_or(0),
            public_only: false,
            required_author_id: (claims.role != "admin").then_some(user_id),
        })
        .await?;
    Ok(Json(LatestProjectsResponse {
        projects: projects.projects.into_iter().map(Into::into).collect(),
        has_more: projects.has_more,
    }))
}

#[axum::debug_handler]
pub async fn change_cover(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(project_id): AxumPath<i64>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ProjectError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| ProjectError::InternalError("Cannot parse id.".to_string()))?;
    let post_id = state
        .project_service
        .get_project_post_id(GetProjectPostIdCommand {
            project_id,
            required_author_id: Some(user_id),
        })
        .await?;

    let mut medium: Option<MediumData> = None;
    let mut opt_og_image_seconds: Option<i64> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ProjectError::InternalError(e.to_string()))?
    {
        let field_name = field.name().ok_or(MediaError::UploadFailed(
            "Empty field detected.".to_string(),
        ))?;
        match field_name {
            "file" => {
                if medium.is_some() {
                    return Err(ProjectError::UploadFailed(
                        "Only one media is allowed at a time.".to_string(),
                    ));
                }
                medium = Some(extract_medium(field).await?);
            }
            "og_image_seconds" => {
                let text = field.text().await.map_err(|e| {
                    ProjectError::InternalError(format!("Failed to read og_image_seconds: {}", e))
                })?;
                opt_og_image_seconds = text.trim().parse::<i64>().ok();
            }
            _ => {}
        }
    }
    let MediumData {
        filename,
        content_type,
        bytes,
    } = medium.ok_or(ProjectError::UploadFailed("Missing file".to_string()))?;

    state
        .media_service
        .change_post_cover(
            ChangePostCoverCommand {
                post_id,
                user_id,
                medium_details: MediumDetails {
                    filename,
                    content_type,
                    bytes,
                },
                og_image_seconds: opt_og_image_seconds,
            },
            &state.media_config,
        )
        .await?;

    if let Some(og_image_seconds) = opt_og_image_seconds {
        state
            .post_service
            .update_post_cover(UpdatePostCoverCommand {
                user_id,
                post_id,
                og_image_seconds: Some(og_image_seconds),
            })
            .await?;
    }

    Ok(())
}
