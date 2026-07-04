use std::{
    cmp::Reverse,
    collections::HashMap,
    fs,
    io::{Cursor, Read, Write},
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use axum::{
    Extension, Json,
    body::Bytes,
    extract::{Multipart, Path as AxumPath, Query, State},
    response::IntoResponse,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
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
    helper::string::replace_range_unicode,
    infrastructure::web::{
        api::handlers::common::{MediumData, extract_medium},
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
    video_short_name: Option<String>,
    og_image_seconds: Option<i64>,
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
    video_short_name: Option<String>,
    og_image_seconds: Option<i64>,
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
}

async fn parse_project_multipart<T: for<'de> Deserialize<'de>>(
    mut multipart: Multipart,
    data_field: &str,
) -> Result<ParsedMultipart<T>, ProjectError> {
    let mut data: Option<T> = None;
    let mut files = HashMap::<usize, FileData>::new();
    let mut short_names = HashMap::<usize, String>::new();
    let mut demo_zip: Option<Bytes> = None;

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
        }
    }

    Ok(ParsedMultipart {
        data: data.ok_or(ProjectError::UploadFailed(
            "No project data is given.".to_string(),
        ))?,
        files,
        short_names,
        demo_zip,
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

    let demo_zip = parsed.demo_zip;
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

    let project_id = state
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
        .await?;

    if let Some(zip) = demo_zip {
        if let Err(err) = extract_demo_zip(&state.project_demo_config, project_id, zip) {
            return Err(err);
        }
    }

    if data.video_short_name.is_some() || data.og_image_seconds.is_some() {
        state
            .post_service
            .update_post_cover(UpdatePostCoverCommand {
                user_id: uploader_id,
                post_id,
                video_short_name: data.video_short_name,
                og_image_seconds: data.og_image_seconds,
            })
            .await?;
    }

    Ok(Json(
        serde_json::json!({ "id": project_id, "post_id": post_id }),
    ))
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

    let has_demo_url = data.demo_url.as_ref().is_some_and(|u| !u.trim().is_empty());
    let has_demo_attachments = parsed.demo_zip.is_some() || has_demo_url;
    if let Some(ref demo_type) = data.demo_type {
        if has_demo_attachments {
            match demo_type.as_str() {
                "none" => {
                    return Err(ProjectError::InvalidDemo(
                        "Demo attachments are not accepted for projects without demos."
                            .to_string(),
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

    if let Some(zip) = parsed.demo_zip {
        extract_demo_zip(&state.project_demo_config, project_id, zip)?;
    }

    if data.video_short_name.is_some() || data.og_image_seconds.is_some() {
        state
            .post_service
            .update_post_cover(UpdatePostCoverCommand {
                user_id,
                post_id,
                video_short_name: data.video_short_name,
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
    pub links: Vec<ProjectLink>,
    pub is_owner: bool,
}

fn project_response(project: Project, include_draft: bool) -> ProjectResponse {
    let demo_url = project.demo.demo_url.clone().unwrap_or_default();
    let raw_demo_url = if demo_url.contains("://") {
        Some(demo_url.clone())
    } else {
        None
    };

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
        published_at: project.published_at,
        updated_at: project.updated_at,
        demo_type: project.demo.demo_type,
        raw_demo_url,
        demo_width: project.demo.width,
        demo_height: project.demo.height,
        demo_config: project.demo.config,
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
    Ok(Json(project_response(project, include_draft)))
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
    Ok(Json(project_response(project, true)))
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
                video_short_name: None,
                og_image_seconds: Some(og_image_seconds),
            })
            .await?;
    }

    Ok(())
}
