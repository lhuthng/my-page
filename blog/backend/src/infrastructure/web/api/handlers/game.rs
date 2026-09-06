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
use once_cell::sync::Lazy;
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
            game::{
                GetFeaturedGamesCommand, GetGameBySlugCommand, GetGameDetailsCommand,
                GetGamePostIdCommand, GetLatestGamesCommand, NewGameCommand, SetFeaturedGameCommand,
                UpdateGameCommand,
            },
        },
        services::{game::GameService, media::MediaService, post::PostService},
    },
    domain::{
        entities::{
            game::{Game, GameLink, GameSnapshot},
            media::MediumDetails,
            secret::Claims,
        },
        errors::{game::GameError, media::MediaError},
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

pub async fn check_game(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CheckQuery>,
) -> Result<impl IntoResponse, GameError> {
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
struct GameData {
    title: String,
    slug: String,
    excerpt: String,
    content: String,
    tags: Vec<String>,
    number_of_files: usize,
    launcher_type: String,
    demo_width: Option<String>,
    demo_height: Option<String>,
    demo_url: Option<String>,
    instruction: String,
    cheatcode: String,
    story: String,
    related_games: Vec<GameLink>,
    v86_upload_id: Option<String>,
}

#[derive(Deserialize)]
struct GamePatchData {
    title: Option<String>,
    slug: Option<String>,
    excerpt: Option<String>,
    content: Option<String>,
    draft: Option<String>,
    tags: Option<Vec<String>>,
    number_of_files: usize,
    launcher_type: Option<String>,
    demo_width: Option<String>,
    demo_height: Option<String>,
    demo_url: Option<String>,
    instruction: Option<String>,
    cheatcode: Option<String>,
    story: Option<String>,
    related_games: Option<Vec<GameLink>>,
    og_image_seconds: Option<i64>,
    v86_upload_id: Option<String>,
    expected_updated_at: Option<String>,
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
    pub game_id: i64,
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

async fn parse_game_multipart<T: for<'de> Deserialize<'de>>(
    mut multipart: Multipart,
    data_field: &str,
) -> Result<ParsedMultipart<T>, GameError> {
    let mut data: Option<T> = None;
    let mut files = HashMap::<usize, FileData>::new();
    let mut short_names = HashMap::<usize, String>::new();
    let mut demo_zip: Option<Bytes> = None;
    let mut create_cover = CreateCoverUpload::default();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| GameError::InternalError(e.to_string()))?
    {
        let field_name = field
            .name()
            .ok_or(GameError::UploadFailed("Empty field found.".to_string()))?
            .to_string();

        if field_name == data_field {
            let bytes = field
                .bytes()
                .await
                .map_err(|e| GameError::UploadFailed(e.to_string()))?;
            data = Some(
                serde_json::from_slice::<T>(&bytes)
                    .map_err(|e| GameError::UploadFailed(e.to_string()))?,
            );
        } else if field_name == "demo_zip" {
            if demo_zip.is_some() {
                return Err(GameError::UploadFailed(
                    "Only one demo zip is allowed.".to_string(),
                ));
            }
            demo_zip = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| GameError::UploadFailed(e.to_string()))?,
            );
        } else if let Some(index_str) = field_name.strip_prefix("file_") {
            let index = index_str
                .parse::<usize>()
                .map_err(|_| GameError::UploadFailed("Invalid file index".to_string()))?;
            if files.contains_key(&index) {
                return Err(GameError::UploadFailed(format!(
                    "Duplicate file index {index}"
                )));
            }
            let file_name = field
                .file_name()
                .ok_or(GameError::UploadFailed(
                    "Cannot read file name.".to_string(),
                ))?
                .to_string();
            let content_type = field
                .content_type()
                .ok_or(GameError::UploadFailed(format!(
                    "Cannot read content type of {}.",
                    file_name
                )))?
                .to_string();
            let bytes = field
                .bytes()
                .await
                .map_err(|_| GameError::UploadFailed(format!("Cannot read {file_name}")))?;
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
                .map_err(|_| GameError::UploadFailed("Invalid short name index".to_string()))?;
            short_names.insert(
                index,
                field.text().await.map_err(|_| {
                    GameError::UploadFailed("Cannot read short name".to_string())
                })?,
            );
        } else if try_collect_create_cover_field(&field_name, field, &mut create_cover).await? {
        }
    }

    Ok(ParsedMultipart {
        data: data.ok_or(GameError::UploadFailed(
            "No game data is given.".to_string(),
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
) -> Result<(), GameError> {
    let mut short_names = Vec::<String>::new();
    let mut file_names = Vec::<String>::new();
    let mut content_types = Vec::<String>::new();
    let mut bytes_list = Vec::<Bytes>::new();

    for i in 1..=number_of_files {
        let file = files
            .get(&i)
            .ok_or_else(|| GameError::UploadFailed(format!("Cannot locate file_{i}")))?;
        let short_name = short_name_map
            .get(&i)
            .ok_or_else(|| GameError::UploadFailed(format!("Cannot locate short_name_{i}")))?;
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

// Runs on every game create and update.
static MEDIA_NAME_REGEXES: Lazy<[Regex; 2]> = Lazy::new(|| {
    [
        Regex::new(r"@(?:\([\d_]+\))?\[[\w-]+:([^\]]+)\]").unwrap(),
        Regex::new(r":::app\s+lottie\s+([^\s]+)").unwrap(),
    ]
});

fn replace_media_short_names(content: &mut String, usage: &mut HashMap<String, i64>) {
    let mut extraction = Vec::<ShortNameExtraction>::new();

    for reg in MEDIA_NAME_REGEXES.iter() {
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

// A 100 MB archive can expand to 200 MB of sync file writes, so the blocking
// pool runs the extraction instead of stalling a tokio worker for seconds.
async fn extract_demo_zip(
    config: &ProjectDemoConfig,
    game_id: i64,
    zip_bytes: Bytes,
) -> Result<(), GameError> {
    let config = config.clone();
    tokio::task::spawn_blocking(move || extract_demo_zip_blocking(config, game_id, zip_bytes))
        .await
        .map_err(|e| GameError::InternalError(e.to_string()))?
}

fn extract_demo_zip_blocking(
    config: ProjectDemoConfig,
    game_id: i64,
    zip_bytes: Bytes,
) -> Result<(), GameError> {
    if zip_bytes.len() as u64 > config.max_archive_size {
        return Err(GameError::InvalidDemo(
            "Demo archive is too large.".to_string(),
        ));
    }

    let mut archive = ZipArchive::new(Cursor::new(zip_bytes))
        .map_err(|e| GameError::InvalidDemo(e.to_string()))?;
    if archive.is_empty() {
        return Err(GameError::InvalidDemo(
            "Demo archive is empty.".to_string(),
        ));
    }
    if archive.len() > config.max_files {
        return Err(GameError::InvalidDemo(
            "Demo archive contains too many files.".to_string(),
        ));
    }

    let mut paths = Vec::<PathBuf>::new();
    for i in 0..archive.len() {
        let file = archive
            .by_index(i)
            .map_err(|e| GameError::InvalidDemo(e.to_string()))?;
        if file.is_dir() {
            continue;
        }
        #[cfg(unix)]
        if file
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(GameError::InvalidDemo(
                "Demo archive cannot contain symlinks.".to_string(),
            ));
        }
        let enclosed = file.enclosed_name().ok_or(GameError::InvalidDemo(
            "Demo archive contains an unsafe path.".to_string(),
        ))?;
        let normalized = normalized_zip_path(&enclosed).ok_or(GameError::InvalidDemo(
            "Demo archive contains an unsafe path.".to_string(),
        ))?;
        paths.push(normalized);
    }

    if paths.is_empty() {
        return Err(GameError::InvalidDemo(
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
        return Err(GameError::InvalidDemo(
            "Demo archive must contain index.html.".to_string(),
        ));
    }

    let root = &config.dir;
    fs::create_dir_all(root)?;
    let tmp_dir = root.join(format!(".tmp-game-{}-{}", game_id, Uuid::new_v4()));
    fs::create_dir_all(&tmp_dir)?;

    let mut extracted_size = 0_u64;
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| GameError::InvalidDemo(e.to_string()))?;
        if file.is_dir() {
            continue;
        }
        let enclosed = file.enclosed_name().ok_or(GameError::InvalidDemo(
            "Demo archive contains an unsafe path.".to_string(),
        ))?;
        let original = normalized_zip_path(&enclosed).ok_or(GameError::InvalidDemo(
            "Demo archive contains an unsafe path.".to_string(),
        ))?;
        let rel = common_root
            .as_ref()
            .and_then(|root| original.strip_prefix(root).ok())
            .map(PathBuf::from)
            .unwrap_or(original);
        if rel.as_os_str().is_empty() || has_invalid_component(&rel) {
            fs::remove_dir_all(&tmp_dir).ok();
            return Err(GameError::InvalidDemo(
                "Demo archive contains an unsafe path.".to_string(),
            ));
        }

        extracted_size = extracted_size.saturating_add(file.size());
        if extracted_size > config.max_extracted_size {
            fs::remove_dir_all(&tmp_dir).ok();
            return Err(GameError::InvalidDemo(
                "Demo archive expands too large.".to_string(),
            ));
        }

        let out_path = tmp_dir.join(&rel);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| GameError::InvalidDemo(e.to_string()))?;
        let mut out = fs::File::create(out_path)?;
        out.write_all(&bytes)?;
    }

    let final_dir = root.join(format!("game-{}", game_id));
    if final_dir.exists() {
        fs::remove_dir_all(&final_dir)?;
    }
    fs::rename(&tmp_dir, &final_dir)?;

    Ok(())
}

fn validate_jsdos_bundle(path: &Path, max_files: usize) -> Result<(u64, String), GameError> {
    let mut file = fs::File::open(path)?;
    let size = file.metadata()?.len();
    let mut has_manifest = false;
    let mut archive = ZipArchive::new(file.try_clone()?)
        .map_err(|e| GameError::InvalidDemo(format!("Invalid js-dos bundle: {e}")))?;
    if archive.is_empty() || archive.len() > max_files {
        return Err(GameError::InvalidDemo(
            "Invalid js-dos bundle file count.".to_string(),
        ));
    }
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|e| GameError::InvalidDemo(e.to_string()))?;
        if entry.name() == ".jsdos/jsdos.json" {
            has_manifest = true;
        }
        if entry.enclosed_name().is_none() {
            return Err(GameError::InvalidDemo(
                "js-dos bundle contains an unsafe path.".to_string(),
            ));
        }
        #[cfg(unix)]
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(GameError::InvalidDemo(
                "js-dos bundle cannot contain symlinks.".to_string(),
            ));
        }
    }
    if !has_manifest {
        return Err(GameError::InvalidDemo(
            "js-dos bundle must contain .jsdos/jsdos.json.".to_string(),
        ));
    }

    file.rewind()?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| GameError::InvalidDemo(e.to_string()))?;
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

fn jsdos_storage_key(game_id: i64, sha256: &str) -> String {
    format!("jsdos/{game_id}/{sha256}.jsdos")
}

pub async fn require_game_owner(
    state: &AppState,
    game_id: i64,
    user_id: i64,
) -> Result<(), crate::domain::errors::project::ProjectError> {
    let owner: Option<i64> = sqlx::query_scalar(
        "SELECT posts.user_id FROM games JOIN posts ON posts.id = games.post_id WHERE games.id = ?",
    )
    .bind(game_id)
    .fetch_optional(&state.game_service.pool)
    .await?;
    match owner {
        Some(id) if id == user_id => Ok(()),
        Some(_) => Err(crate::domain::errors::project::ProjectError::Forbidden),
        None => Err(crate::domain::errors::project::ProjectError::ProjectNotFound),
    }
}

pub async fn start_jsdos_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(game_id): AxumPath<i64>,
    Json(request): Json<StartJsDosUploadRequest>,
) -> Result<impl IntoResponse, GameError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse user id".to_string()))?;
    require_game_owner(&state, game_id, user_id).await?;
    if !request.file_name.to_ascii_lowercase().ends_with(".jsdos") {
        return Err(GameError::InvalidDemo(
            "Only .jsdos bundles are accepted.".to_string(),
        ));
    }
    if request.size_bytes == 0 || request.size_bytes > state.project_demo_config.max_jsdos_size {
        return Err(GameError::InvalidDemo(format!(
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
        r#"INSERT INTO game_jsdos_upload_sessions
           (id, game_id, uploader_id, original_file_name, expected_size_bytes,
            chunk_size_bytes, temp_storage_key, expires_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now', ?))"#,
    )
    .bind(&upload_id)
    .bind(game_id)
    .bind(user_id)
    .bind(&request.file_name)
    .bind(request.size_bytes as i64)
    .bind(state.project_demo_config.jsdos_chunk_size as i64)
    .bind(temp_path.to_string_lossy().to_string())
    .bind(format!("+{ttl_hours} hours"))
    .execute(&state.game_service.pool)
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
    AxumPath((_game_id, upload_id, chunk_index)): AxumPath<(i64, String, u64)>,
    body: Bytes,
) -> Result<impl IntoResponse, GameError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse user id".to_string()))?;
    let row: Option<(i64, i64, i64, i64, i64, String, String)> = sqlx::query_as(
        "SELECT game_id, uploader_id, expected_size_bytes, received_size_bytes, next_chunk_index, temp_storage_key, status FROM game_jsdos_upload_sessions WHERE id = ?",
    )
    .bind(&upload_id)
    .fetch_optional(&state.game_service.pool)
    .await?;
    let (game_id, uploader_id, expected, received, next, temp_key, status) =
        row.ok_or(GameError::GameNotFound)?;
    if uploader_id != user_id || game_id != _game_id {
        return Err(GameError::Forbidden);
    }
    if status != "active" || chunk_index != next as u64 {
        return Err(GameError::InvalidDemo(
            "Invalid or out-of-order js-dos upload chunk.".to_string(),
        ));
    }
    let chunk_size = state.project_demo_config.jsdos_chunk_size;
    if body.is_empty()
        || body.len() as u64 > chunk_size
        || received as u64 + body.len() as u64 > expected as u64
    {
        return Err(GameError::InvalidDemo(
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
        "UPDATE game_jsdos_upload_sessions SET received_size_bytes = ?, next_chunk_index = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(received_size as i64)
    .bind((chunk_index + 1) as i64)
    .bind(&upload_id)
    .execute(&state.game_service.pool)
    .await?;
    Ok(Json(JsDosUploadResponse {
        received_size_bytes: received_size,
        next_chunk_index: chunk_index + 1,
    }))
}

pub async fn complete_jsdos_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath((game_id, upload_id)): AxumPath<(i64, String)>,
) -> Result<impl IntoResponse, GameError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse user id".to_string()))?;
    require_game_owner(&state, game_id, user_id).await?;
    let row: Option<(String, i64, i64, String, String)> = sqlx::query_as(
        "SELECT original_file_name, expected_size_bytes, received_size_bytes, temp_storage_key, status FROM game_jsdos_upload_sessions WHERE id = ? AND game_id = ? AND uploader_id = ?",
    )
    .bind(&upload_id)
    .bind(game_id)
    .bind(user_id)
    .fetch_optional(&state.game_service.pool)
    .await?;
    let (file_name, expected, received, temp_key, status) =
        row.ok_or(GameError::GameNotFound)?;
    if status != "active" || expected != received {
        return Err(GameError::InvalidDemo(
            "js-dos upload is incomplete.".to_string(),
        ));
    }
    let temp_path = PathBuf::from(&temp_key);
    let max_files = state.project_demo_config.max_files;
    let (size, sha256) =
        tokio::task::spawn_blocking(move || validate_jsdos_bundle(&temp_path, max_files))
            .await
            .map_err(|e| GameError::InternalError(e.to_string()))??;
    let storage_key = jsdos_storage_key(game_id, &sha256);
    let final_path = state.project_demo_config.dir.join(&storage_key);
    if let Some(parent) = final_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::rename(&temp_key, &final_path).await?;

    let mut tx = state.game_service.pool.begin().await?;
    sqlx::query(
        "INSERT INTO game_jsdos_bundles (game_id, storage_key, original_file_name, size_bytes, sha256) VALUES (?, ?, ?, ?, ?) ON CONFLICT(game_id) DO UPDATE SET storage_key = excluded.storage_key, original_file_name = excluded.original_file_name, size_bytes = excluded.size_bytes, sha256 = excluded.sha256, updated_at = CURRENT_TIMESTAMP",
    )
    .bind(game_id)
    .bind(&storage_key)
    .bind(&file_name)
    .bind(size as i64)
    .bind(&sha256)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE games SET launcher_type = 'jsdos', demo_url = NULL, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(game_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_jsdos_upload_sessions SET status = 'completed', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&upload_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(Json(CompleteJsDosUploadResponse {
        game_id,
        file_name,
        size_bytes: size,
        sha256,
        bundle_url: format!("games/s/{}/jsdos", game_slug(&state, game_id).await?.unwrap_or_default()),
    }))
}

async fn game_slug(state: &AppState, game_id: i64) -> Result<Option<String>, GameError> {
    Ok(sqlx::query_scalar(
        "SELECT posts.slug FROM games JOIN posts ON posts.id = games.post_id WHERE games.id = ?",
    )
    .bind(game_id)
    .fetch_optional(&state.game_service.pool)
    .await?)
}

pub async fn abort_jsdos_upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath((game_id, upload_id)): AxumPath<(i64, String)>,
) -> Result<StatusCode, GameError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse user id".to_string()))?;
    let temp_key: Option<String> = sqlx::query_scalar(
        "SELECT temp_storage_key FROM game_jsdos_upload_sessions WHERE id = ? AND game_id = ? AND uploader_id = ? AND status = 'active'",
    )
    .bind(&upload_id)
    .bind(game_id)
    .bind(user_id)
    .fetch_optional(&state.game_service.pool)
    .await?;
    if let Some(temp_key) = temp_key {
        sqlx::query("UPDATE game_jsdos_upload_sessions SET status = 'aborted', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(&upload_id)
            .execute(&state.game_service.pool)
            .await?;
        tokio::fs::remove_file(temp_key).await.ok();
    }
    Ok(StatusCode::NO_CONTENT)
}

const MAX_GAME_LINKS: usize = 20;

fn normalize_links(links: Vec<GameLink>) -> Result<Vec<GameLink>, GameError> {
    let kept: Vec<GameLink> = links
        .into_iter()
        .filter(|link| !link.title.trim().is_empty() && !link.slug.trim().is_empty())
        .collect();

    if kept.len() > MAX_GAME_LINKS {
        return Err(GameError::InvalidDemo(format!(
            "A game may have at most {MAX_GAME_LINKS} related games."
        )));
    }

    Ok(kept)
}

pub async fn get_jsdos_bundle(
    State(state): State<Arc<AppState>>,
    AxumPath(slug): AxumPath<String>,
) -> Result<Response, GameError> {
    let storage_key: Option<String> = sqlx::query_scalar(
        "SELECT b.storage_key FROM game_jsdos_bundles b JOIN games g ON g.id = b.game_id JOIN posts ON posts.id = g.post_id WHERE posts.slug = ? AND posts.status = 'published' AND g.launcher_type = 'jsdos'",
    )
    .bind(&slug)
    .fetch_optional(&state.game_service.pool)
    .await?;
    let storage_key = storage_key.ok_or(GameError::GameNotFound)?;
    if Path::new(&storage_key).components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(GameError::InternalError(
            "Invalid js-dos storage key".to_string(),
        ));
    }
    let path = state.project_demo_config.dir.join(&storage_key);
    let file = tokio::fs::File::open(&path)
        .await
        .map_err(|_| GameError::GameNotFound)?;
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

fn validate_demo_url(url: Option<String>) -> Result<Option<String>, GameError> {
    match url {
        Some(u) if u.trim().is_empty() => Ok(Some(String::new())),
        Some(u) => Ok(Some(
            crate::helper::string::validate_http_url(&u, "Demo URL")
                .map_err(GameError::InvalidDemo)?,
        )),
        None => Ok(None),
    }
}

#[axum::debug_handler]
pub async fn new_game(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    multipart: Multipart,
) -> Result<impl IntoResponse, GameError> {
    let uploader_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse id".to_string()))?;
    let parsed = parse_game_multipart::<GameData>(multipart, "game_data").await?;
    let mut data = parsed.data;

    if state
        .post_service
        .check_slug(CheckSlugCommand {
            post_slug: data.slug.clone(),
        })
        .await?
    {
        return Err(GameError::InvalidDemo(format!(
            "The game slug '{}' is already in use.",
            data.slug
        )));
    }

    let demo_zip = parsed.demo_zip;
    let create_cover = parsed.create_cover;
    let has_demo_url = data.demo_url.as_ref().is_some_and(|u| !u.trim().is_empty());
    match data.launcher_type.as_str() {
        "html5" | "webgl" => {
            if has_demo_url {
                return Err(GameError::InvalidDemo(format!(
                    "Demo URL is not accepted for {} games.",
                    data.launcher_type
                )));
            }
            if demo_zip.is_none() {
                return Err(GameError::InvalidDemo(format!(
                    "Demo zip is required for {} games.",
                    data.launcher_type
                )));
            }
        }
        "embed" | "download" | "video" => {
            if !has_demo_url {
                return Err(GameError::InvalidDemo(format!(
                    "Demo URL is required for {} games.",
                    data.launcher_type
                )));
            }
        }
        "jsdos" => {
            if has_demo_url || demo_zip.is_some() {
                return Err(GameError::InvalidDemo(
                    "js-dos bundles must be uploaded through the js-dos upload endpoint."
                        .to_string(),
                ));
            }
        }
        "v86" => {
            if has_demo_url || demo_zip.is_some() {
                return Err(GameError::InvalidDemo(
                    "v86 games must be uploaded through the v86 package endpoint.".to_string(),
                ));
            }
            if data.v86_upload_id.is_none() {
                return Err(GameError::InvalidDemo(
                    "A completed v86 game package is required.".to_string(),
                ));
            }
        }
        _ => {
            return Err(GameError::InvalidDemo(format!(
                "Unsupported launcher type: {}",
                data.launcher_type
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
            content_kind: "game".to_string(),
        })
        .await?;

    let game_result = state
        .game_service
        .new_game(NewGameCommand {
            post_id,
            launcher_type: data.launcher_type,
            demo_width: data.demo_width,
            demo_height: data.demo_height,
            demo_url: validate_demo_url(data.demo_url)?,
            instruction: data.instruction,
            cheatcode: data.cheatcode,
            story: data.story,
            related_games: normalize_links(data.related_games)?,
        })
        .await;
    let game_id = match game_result {
        Ok(game_id) => game_id,
        Err(error) => {
            sqlx::query("DELETE FROM posts WHERE id = ?")
                .bind(post_id)
                .execute(&state.game_service.pool)
                .await
                .ok();
            return Err(error.into());
        }
    };

    if let Some(upload_id) = data.v86_upload_id.as_deref() {
        let mut tx = state.game_service.pool.begin().await?;
        let attach_result = attach_ready_game_tx(
            &mut tx,
            game_id,
            uploader_id,
            upload_id,
            state.project_demo_config.v86_download_chunk_size,
        )
        .await;
        if let Err(error) = attach_result {
            tx.rollback().await.ok();
            sqlx::query("DELETE FROM posts WHERE id = ?")
                .bind(post_id)
                .execute(&state.game_service.pool)
                .await
                .ok();
            return Err(error.into());
        }
        tx.commit().await?;
    }

    if let Some(zip) = demo_zip {
        if let Err(err) = extract_demo_zip(&state.project_demo_config, game_id, zip).await {
            return Err(err);
        }
    }

    apply_created_cover_upload(&state, uploader_id, post_id, create_cover).await?;

    Ok(Json(
        serde_json::json!({ "id": game_id, "post_id": post_id }),
    ))
}

#[derive(Deserialize)]
pub struct DeleteGameQuery {
    pub reason: Option<String>,
    pub detail: Option<String>,
    pub force: Option<bool>,
}

fn is_admin_or_mod_game(role: &str) -> bool {
    role == "admin" || role == "moderator"
}

async fn require_can_delete_game(
    state: &AppState,
    game_id: i64,
    claims: &Claims,
) -> Result<i64, GameError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse id".to_string()))?;
    let owner: Option<i64> = sqlx::query_scalar(
        "SELECT posts.user_id FROM games JOIN posts ON posts.id = games.post_id WHERE games.id = ?",
    )
    .bind(game_id)
    .fetch_optional(&state.game_service.pool)
    .await?;
    let owner = owner.ok_or(GameError::GameNotFound)?;
    if owner == user_id || is_admin_or_mod_game(&claims.role) {
        let post_id: Option<i64> = sqlx::query_scalar("SELECT post_id FROM games WHERE id=?")
            .bind(game_id)
            .fetch_optional(&state.game_service.pool)
            .await?;
        post_id.ok_or(GameError::GameNotFound)
    } else {
        Err(GameError::Forbidden)
    }
}

pub async fn delete_game_draft(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(game_id): AxumPath<i64>,
    Query(query): Query<DeleteGameQuery>,
) -> Result<StatusCode, GameError> {
    let post_id = require_can_delete_game(&state, game_id, &claims).await?;
    let row = sqlx::query_as::<_, (String, String, String, Option<String>)>(
        "SELECT posts.content_kind, posts.title, posts.slug, posts.deleted_at FROM posts JOIN games ON games.post_id = posts.id WHERE games.id = ?",
    )
    .bind(game_id)
    .fetch_optional(&state.game_service.pool)
    .await?
    .ok_or(GameError::GameNotFound)?;
    // For legacy shared posts (project and game share same post_id with content_kind='project'),
    // allow deletion via the game endpoint — the kind guard would otherwise block it.
    let is_shared = sqlx::query_scalar::<_, Option<i64>>("SELECT id FROM projects WHERE post_id = ?")
        .bind(post_id)
        .fetch_optional(&state.game_service.pool)
        .await?
        .is_some();
    if row.0 != "game" && !(is_shared && row.0 == "project") {
        return Err(GameError::InvalidDemo(
            "Use the typed delete endpoint for this content kind.".to_string(),
        ));
    }
    if row.3.is_some() && !is_shared {
        return Err(GameError::Conflict("Game already in trash.".to_string()));
    }
    // Shared-post game: hard-delete the game row only, keep the post for the project
    if is_shared {
        // check delegated published projects (includes the shared project itself)
        let delegating: Vec<i64> = sqlx::query_scalar(
            "SELECT projects.id FROM projects JOIN posts p ON p.id = projects.post_id WHERE projects.delegate_game_id = ? AND p.deleted_at IS NULL AND p.status = 'published'",
        )
        .bind(game_id)
        .fetch_all(&state.game_service.pool)
        .await?;
        if !delegating.is_empty() && query.force != Some(true) {
            return Err(GameError::Conflict(format!(
                "Game is delegated by {} published project(s); use ?force=true to confirm.",
                delegating.len()
            )));
        }
        let delegating_json = serde_json::to_string(&delegating).unwrap_or_else(|_| "[]".to_string());
        sqlx::query(
            "INSERT INTO game_deletion_log (game_id, slug, title, reason, detail, deleted_by, delegated_project_ids) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(game_id)
        .bind(&row.2)
        .bind(&row.1)
        .bind(query.reason.clone().unwrap_or_else(|| "user_request".to_string()))
        .bind(&query.detail)
        .bind(claims.user_id.parse::<i64>().unwrap_or(0))
        .bind(&delegating_json)
        .execute(&state.game_service.pool)
        .await
        .ok();
        sqlx::query("DELETE FROM games WHERE id = ?")
            .bind(game_id)
            .execute(&state.game_service.pool)
            .await?;
        let dir = state.project_demo_config.dir.join(format!("game-{}", game_id));
        let _ = tokio::fs::remove_dir_all(&dir).await;
        return Ok(StatusCode::NO_CONTENT);
    }
    let reason = query.reason.unwrap_or_else(|| "user_request".to_string());
    let allowed = ["user_request", "dmca", "moderation", "replaced", "other"];
    if !allowed.contains(&reason.as_str()) {
        return Err(GameError::InvalidDemo("Invalid deletion reason.".to_string()));
    }
    // check delegated published projects
    let delegating: Vec<i64> = sqlx::query_scalar(
        "SELECT projects.id FROM projects JOIN posts p ON p.id = projects.post_id WHERE projects.delegate_game_id = ? AND p.deleted_at IS NULL AND p.status = 'published'",
    )
    .bind(game_id)
    .fetch_all(&state.game_service.pool)
    .await?;
    if !delegating.is_empty() && query.force != Some(true) {
        return Err(GameError::Conflict(format!(
            "Game is delegated by {} published project(s); use ?force=true to confirm.",
            delegating.len()
        )));
    }
    // capture delegated ids for log (even before soft-delete, keep for later hard purge)
    let delegating_json = serde_json::to_string(&delegating).unwrap_or_else(|_| "[]".to_string());
    // log for tombstone (also survives hard purge)
    sqlx::query(
        "INSERT INTO game_deletion_log (game_id, slug, title, reason, detail, deleted_by, delegated_project_ids) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(game_id)
    .bind(&row.2)
    .bind(&row.1)
    .bind(&reason)
    .bind(&query.detail)
    .bind(claims.user_id.parse::<i64>().unwrap_or(0))
    .bind(&delegating_json)
    .execute(&state.game_service.pool)
    .await
    .ok();
    sqlx::query(
        "UPDATE posts SET deleted_at = CURRENT_TIMESTAMP, deletion_reason = ?, deletion_detail = ?, deleted_by = ?, scheduled_purge_at = datetime('now','+7 days'), prev_status = status WHERE id = ?",
    )
    .bind(&reason)
    .bind(&query.detail)
    .bind(claims.user_id.parse::<i64>().unwrap_or(0))
    .bind(post_id)
    .execute(&state.game_service.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn restore_game(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(game_id): AxumPath<i64>,
) -> Result<StatusCode, GameError> {
    let post_id = require_can_delete_game(&state, game_id, &claims).await?;
    let deleted_at: Option<String> = sqlx::query_scalar("SELECT deleted_at FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_optional(&state.game_service.pool)
        .await?
        .ok_or(GameError::GameNotFound)?;
    if deleted_at.is_none() {
        return Err(GameError::Conflict("Game is not in trash.".to_string()));
    }
    sqlx::query(
        "UPDATE posts SET deleted_at = NULL, deletion_reason = NULL, deletion_detail = NULL, deleted_by = NULL, scheduled_purge_at = NULL, prev_status = NULL WHERE id = ?",
    )
    .bind(post_id)
    .execute(&state.game_service.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn purge_game_now(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(game_id): AxumPath<i64>,
) -> Result<StatusCode, GameError> {
    if claims.role != "admin" {
        return Err(GameError::Forbidden);
    }
    let post_id: Option<i64> = sqlx::query_scalar("SELECT post_id FROM games WHERE id=?")
        .bind(game_id)
        .fetch_optional(&state.game_service.pool)
        .await?
        .ok_or(GameError::GameNotFound)?;
    sqlx::query("DELETE FROM posts WHERE id = ?")
        .bind(post_id)
        .execute(&state.game_service.pool)
        .await?;
    let dir = state.project_demo_config.dir.join(format!("game-{}", game_id));
    let _ = tokio::fs::remove_dir_all(&dir).await;
    Ok(StatusCode::NO_CONTENT)
}

#[axum::debug_handler]
pub async fn update_game(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(game_id): AxumPath<i64>,
    multipart: Multipart,
) -> Result<impl IntoResponse, GameError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse id".to_string()))?;

    let post_id = state
        .game_service
        .get_game_post_id(GetGamePostIdCommand {
            game_id,
            required_author_id: Some(user_id),
        })
        .await?;

    let parsed = parse_game_multipart::<GamePatchData>(multipart, "game_data").await?;
    let mut data = parsed.data;
    let current_launcher_type: String =
        sqlx::query_scalar("SELECT launcher_type FROM games WHERE id = ?")
            .bind(game_id)
            .fetch_optional(&state.game_service.pool)
            .await?
            .ok_or(GameError::GameNotFound)?;
    let effective_launcher_type = data
        .launcher_type
        .as_deref()
        .unwrap_or(current_launcher_type.as_str())
        .to_string();

    let has_demo_url = data.demo_url.as_ref().is_some_and(|u| !u.trim().is_empty());
    let has_demo_attachments = parsed.demo_zip.is_some() || has_demo_url;
    if let Some(ref launcher_type) = data.launcher_type {
        if has_demo_attachments {
            match launcher_type.as_str() {
                "html5" | "webgl" => {
                    if data.demo_url.is_some() {
                        return Err(GameError::InvalidDemo(format!(
                            "Demo URL is not accepted for {} games.",
                            launcher_type
                        )));
                    }
                    if parsed.demo_zip.is_none() {
                        return Err(GameError::InvalidDemo(format!(
                            "Demo zip is required for {} games.",
                            launcher_type
                        )));
                    }
                }
                "embed" | "download" | "video" => {
                    let has_url = data.demo_url.as_ref().is_some_and(|u| !u.trim().is_empty());
                    if !has_url {
                        return Err(GameError::InvalidDemo(format!(
                            "Demo URL is required for {} games.",
                            launcher_type
                        )));
                    }
                }
                "jsdos" => {
                    if has_demo_url || parsed.demo_zip.is_some() {
                        return Err(GameError::InvalidDemo(
                            "js-dos bundles must be uploaded through the js-dos upload endpoint."
                                .to_string(),
                        ));
                    }
                }
                "v86" => {
                    if has_demo_url || parsed.demo_zip.is_some() {
                        return Err(GameError::InvalidDemo(
                            "v86 games must be uploaded through the v86 package endpoint."
                                .to_string(),
                        ));
                    }
                }
                _ => {
                    return Err(GameError::InvalidDemo(format!(
                        "Unsupported launcher type: {}",
                        launcher_type
                    )));
                }
            }
        }
    } else if has_demo_attachments {
        return Err(GameError::InvalidDemo(
            "Launcher type is required when providing demo attachments.".to_string(),
        ));
    }

    upload_inline_media(
        &state,
        user_id,
        data.number_of_files,
        &parsed.files,
        &parsed.short_names,
    )
    .await?;

    if data.content.as_ref().xor(data.draft.as_ref()).is_some() {
        return Err(GameError::UploadFailed(
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

    let updated_at = state
        .post_service
        .update_post(UpdatePostCommand {
            user_id,
            required_author_id: Some(user_id),
            expected_updated_at: data.expected_updated_at.take(),
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

    let mut demo_url = validate_demo_url(data.demo_url)?.filter(|u| !u.trim().is_empty());
    if parsed.demo_zip.is_some() {
        let local_demo_url = state
            .project_demo_config
            .dir
            .join(format!("game-{game_id}"))
            .join("index.html");
        demo_url = Some(local_demo_url.to_str().unwrap_or("").to_string());
    }

    let keeps_jsdos_bundle = effective_launcher_type == "jsdos";
    let keeps_v86_game = effective_launcher_type == "v86";

    state
        .game_service
        .update_game(UpdateGameCommand {
            game_id,
            user_id,
            launcher_type: data.launcher_type,
            demo_width: data.demo_width,
            demo_height: data.demo_height,
            demo_url,
            instruction: data.instruction,
            cheatcode: data.cheatcode,
            story: data.story,
            related_games: data.related_games.map(normalize_links).transpose()?,
        })
        .await?;

    if let Some(upload_id) = data.v86_upload_id.as_deref() {
        if !keeps_v86_game {
            return Err(GameError::InvalidDemo(
                "A v86 package cannot be attached to a non-v86 game.".to_string(),
            ));
        }
        let mut tx = state.game_service.pool.begin().await?;
        attach_ready_game_tx(
            &mut tx,
            game_id,
            user_id,
            upload_id,
            state.project_demo_config.v86_download_chunk_size,
        )
        .await?;
        tx.commit().await?;
    }

    if !keeps_jsdos_bundle {
        if let Some(storage_key) = sqlx::query_scalar::<_, String>(
            "SELECT storage_key FROM game_jsdos_bundles WHERE game_id = ?",
        )
        .bind(game_id)
        .fetch_optional(&state.game_service.pool)
        .await?
        {
            sqlx::query("DELETE FROM game_jsdos_bundles WHERE game_id = ?")
                .bind(game_id)
                .execute(&state.game_service.pool)
                .await?;
            tokio::fs::remove_file(state.project_demo_config.dir.join(storage_key))
                .await
                .ok();
        }
    }

    if !keeps_v86_game {
        sqlx::query("DELETE FROM game_v86_games WHERE game_id = ?")
            .bind(game_id)
            .execute(&state.game_service.pool)
            .await?;
    }

    if let Some(zip) = parsed.demo_zip {
        extract_demo_zip(&state.project_demo_config, game_id, zip).await?;
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

    Ok(Json(UpdateGameResponse { updated_at }))
}

#[derive(Serialize)]
pub struct UpdateGameResponse {
    pub updated_at: String,
}

#[axum::debug_handler]
pub async fn publish_game(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(game_id): AxumPath<i64>,
) -> Result<(), GameError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse id".to_string()))?;
    let post_id = state
        .game_service
        .get_game_post_id(GetGamePostIdCommand {
            game_id,
            required_author_id: Some(user_id),
        })
        .await?;
    let launcher_type: String = sqlx::query_scalar("SELECT launcher_type FROM games WHERE id = ?")
        .bind(game_id)
        .fetch_one(&state.game_service.pool)
        .await?;
    if launcher_type == "jsdos" {
        let has_bundle: Option<i64> =
            sqlx::query_scalar("SELECT game_id FROM game_jsdos_bundles WHERE game_id = ?")
                .bind(game_id)
                .fetch_optional(&state.game_service.pool)
                .await?;
        if has_bundle.is_none() {
            return Err(GameError::InvalidDemo(
                "A completed js-dos bundle is required before publishing.".to_string(),
            ));
        }
    }
    if launcher_type == "v86" {
        let has_game: Option<i64> =
            sqlx::query_scalar("SELECT game_id FROM game_v86_games WHERE game_id = ?")
                .bind(game_id)
                .fetch_optional(&state.game_service.pool)
                .await?;
        if has_game.is_none() {
            return Err(GameError::InvalidDemo(
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
pub struct GameResponse {
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
    pub launcher_type: String,
    pub demo_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_demo_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demo_width: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demo_height: Option<String>,
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
    pub instruction: String,
    pub cheatcode: String,
    pub story: String,
    pub related_games: Vec<GameLink>,
    pub is_owner: bool,
}

fn game_response(game: Game, include_draft: bool) -> GameResponse {
    let mut demo_url = game.demo.demo_url.clone().unwrap_or_default();
    let raw_demo_url = if demo_url.contains("://") {
        Some(demo_url.clone())
    } else {
        None
    };
    let jsdos_bundle_url = game
        .demo
        .jsdos_bundle
        .as_ref()
        .map(|_| format!("games/s/{}/jsdos", game.slug));
    let jsdos_bundle_file_name = game
        .demo
        .jsdos_bundle
        .as_ref()
        .map(|bundle| bundle.original_file_name.clone());
    let jsdos_bundle_size_bytes = game
        .demo
        .jsdos_bundle
        .as_ref()
        .map(|bundle| bundle.size_bytes);
    if game.demo.launcher_type == "jsdos" && jsdos_bundle_url.is_some() {
        demo_url = format!("games/s/{}/jsdos", game.slug);
    }

    GameResponse {
        demo_url,
        id: game.id,
        post_id: game.post_id,
        title: game.title,
        slug: game.slug,
        tags: game.tags,
        author_name: game.author_name,
        author_slug: game.author_slug,
        author_avatar_url: game.author_avatar_url,
        excerpt: game.excerpt,
        content: game.content,
        draft: include_draft.then_some(game.draft),
        medium_urls: game.medium_urls,
        medium_short_names: game.medium_short_names,
        cover_url: game.cover_url,
        cover_media_type: game.cover_media_type,
        og_image_url: game.og_image_url,
        cover_video_url: game.cover_video_url,
        cover_video_type: game.cover_video_type,
        og_image_seconds: game.og_image_seconds,
        published_at: normalize_optional_utc_timestamp(game.published_at),
        updated_at: normalize_optional_utc_timestamp(game.updated_at),
        launcher_type: game.demo.launcher_type,
        raw_demo_url,
        demo_width: game.demo.width,
        demo_height: game.demo.height,
        jsdos_bundle_url,
        jsdos_bundle_file_name,
        jsdos_bundle_size_bytes,
        v86_runtime: None,
        v86_system_version_id: None,
        v86_manifest: None,
        v86_artifact_revision: None,
        v86_game_file_name: None,
        instruction: game.instruction,
        cheatcode: game.cheatcode,
        story: game.story,
        related_games: game.related_games,
        is_owner: game.is_owner,
    }
}

#[derive(Deserialize)]
pub struct GetGameQuery {
    pub with_draft: Option<bool>,
}

pub async fn get_game_by_slug(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    AxumPath(slug): AxumPath<String>,
    Query(query): Query<GetGameQuery>,
) -> Result<impl IntoResponse, GameError> {
    let mut as_id = None;
    let include_draft = query.with_draft.unwrap_or(false);
    if include_draft && let Some(claims) = opt_claims {
        as_id = Some(
            claims
                .user_id
                .parse::<i64>()
                .map_err(|_| GameError::InternalError("Cannot parse id".to_string()))?,
        );
    }

    let game = state
        .game_service
        .get_game_by_slug(GetGameBySlugCommand { slug, as_id })
        .await?;
    let mut response = game_response(game, include_draft);
    if response.launcher_type == "v86" {
        response.v86_runtime = runtime_descriptor(
            &state.game_service.pool,
            &response.slug,
            state.artifact_base_url(),
        )
        .await?;
        let game = sqlx::query(
            "SELECT system_version_id, manifest_text, artifact_revision FROM game_v86_games WHERE game_id = ?",
        )
        .bind(response.id)
        .fetch_optional(&state.game_service.pool)
        .await?;
        if let Some(game) = game {
            use sqlx::Row;
            response.v86_system_version_id = Some(game.get("system_version_id"));
            response.v86_manifest = Some(game.get("manifest_text"));
            response.v86_artifact_revision = Some(game.get("artifact_revision"));
        }
    }
    Ok(Json(response))
}

pub async fn get_game_details(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(game_id): AxumPath<i64>,
) -> Result<impl IntoResponse, GameError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse id".to_string()))?;
    let is_admin = claims.role == "admin";
    let game = state
        .game_service
        .get_game_details(GetGameDetailsCommand {
            game_id,
            viewing_user_id: user_id,
            required_author_id: if is_admin { None } else { Some(user_id) },
        })
        .await?;
    let mut response = game_response(game, true);
    if response.launcher_type == "v86" {
        let game = sqlx::query(
            "SELECT system_version_id, manifest_text, artifact_revision FROM game_v86_games WHERE game_id = ?",
        )
        .bind(response.id)
        .fetch_optional(&state.game_service.pool)
        .await?;
        if let Some(game) = game {
            use sqlx::Row;
            response.v86_system_version_id = Some(game.get("system_version_id"));
            response.v86_manifest = Some(game.get("manifest_text"));
            response.v86_artifact_revision = Some(game.get("artifact_revision"));
        }
    }
    Ok(Json(response))
}

#[derive(Deserialize)]
pub struct LatestGamesQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct GameStats {
    pub views: i64,
    pub likes: i64,
    pub comments: i64,
}

#[derive(Serialize)]
pub struct GameCard {
    pub id: i64,
    pub post_id: i64,
    pub title: String,
    pub slug: String,
    pub tag_names: Vec<String>,
    pub tag_slugs: Vec<String>,
    pub excerpt: String,
    pub author_name: String,
    pub author_slug: String,
    pub launcher_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub cover_media_type: Option<String>,
    pub stats: GameStats,
    pub reading_time_minutes: i64,
}

impl From<GameSnapshot> for GameCard {
    fn from(value: GameSnapshot) -> Self {
        GameCard {
            id: value.id,
            post_id: value.post_id,
            title: value.title,
            slug: value.slug,
            tag_names: value.tag_names,
            tag_slugs: value.tag_slugs,
            excerpt: value.excerpt,
            author_name: value.author_name,
            author_slug: value.author_slug,
            launcher_type: value.launcher_type,
            url: value.url,
            cover_media_type: value.cover_media_type,
            stats: GameStats {
                views: value.stats.views,
                likes: value.stats.likes,
                comments: value.stats.comments,
            },
            reading_time_minutes: value.reading_time_minutes,
        }
    }
}

#[derive(Serialize)]
pub struct LatestGamesResponse {
    pub games: Vec<GameCard>,
    pub has_more: bool,
}

pub async fn get_latest_games(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LatestGamesQuery>,
) -> Result<impl IntoResponse, GameError> {
    let games = state
        .game_service
        .get_latest_game_snapshots(GetLatestGamesCommand {
            limit: query.limit.unwrap_or(24),
            offset: query.offset.unwrap_or(0),
            public_only: true,
            required_author_id: None,
        })
        .await?;
    Ok(Json(LatestGamesResponse {
        games: games.games.into_iter().map(Into::into).collect(),
        has_more: games.has_more,
    }))
}

#[derive(Deserialize)]
pub struct FeaturedGamesQuery {
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct FeaturedGamesResponse {
    pub featured_games: Vec<GameCard>,
    pub has_more: bool,
}

pub async fn get_featured_games(
    State(state): State<Arc<AppState>>,
    Query(query): Query<FeaturedGamesQuery>,
) -> Result<impl IntoResponse, GameError> {
    let games = state
        .game_service
        .get_featured_game_snapshots(GetFeaturedGamesCommand {
            limit: query.limit.unwrap_or(5),
        })
        .await?;

    Ok(Json(FeaturedGamesResponse {
        featured_games: games.into_iter().map(Into::into).collect(),
        has_more: false,
    }))
}

#[derive(Deserialize)]
pub struct SetGameFeaturedBody {
    pub is_featured: bool,
}

pub async fn set_game_featured(
    State(state): State<Arc<AppState>>,
    AxumPath(game_id): AxumPath<i64>,
    Json(body): Json<SetGameFeaturedBody>,
) -> Result<impl IntoResponse, GameError> {
    state
        .game_service
        .set_game_featured(SetFeaturedGameCommand {
            game_id,
            is_featured: body.is_featured,
        })
        .await?;
    Ok(())
}

pub async fn get_all_games(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<LatestGamesQuery>,
) -> Result<impl IntoResponse, GameError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse id".to_string()))?;
    let games = state
        .game_service
        .get_latest_game_snapshots(GetLatestGamesCommand {
            limit: query.limit.unwrap_or(100),
            offset: query.offset.unwrap_or(0),
            public_only: false,
            required_author_id: (claims.role != "admin").then_some(user_id),
        })
        .await?;
    Ok(Json(LatestGamesResponse {
        games: games.games.into_iter().map(Into::into).collect(),
        has_more: games.has_more,
    }))
}

#[axum::debug_handler]
pub async fn change_cover(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    AxumPath(game_id): AxumPath<i64>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, GameError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| GameError::InternalError("Cannot parse id.".to_string()))?;
    let post_id = state
        .game_service
        .get_game_post_id(GetGamePostIdCommand {
            game_id,
            required_author_id: Some(user_id),
        })
        .await?;

    let mut medium: Option<MediumData> = None;
    let mut opt_og_image_seconds: Option<i64> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| GameError::InternalError(e.to_string()))?
    {
        let field_name = field.name().ok_or(MediaError::UploadFailed(
            "Empty field detected.".to_string(),
        ))?;
        match field_name {
            "file" => {
                if medium.is_some() {
                    return Err(GameError::UploadFailed(
                        "Only one media is allowed at a time.".to_string(),
                    ));
                }
                medium = Some(extract_medium(field).await?);
            }
            "og_image_seconds" => {
                let text = field.text().await.map_err(|e| {
                    GameError::InternalError(format!("Failed to read og_image_seconds: {}", e))
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
    } = medium.ok_or(GameError::UploadFailed("Missing file".to_string()))?;

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