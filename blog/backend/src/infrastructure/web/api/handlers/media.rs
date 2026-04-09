use std::str::FromStr;
use std::sync::Arc;

use axum::{
    Extension, Json,
    body::{Body, Bytes},
    extract::{Multipart, Path, Query, State},
    response::IntoResponse,
};
use http::{Response, header};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio_util::io::ReaderStream;

use crate::{
    application::{
        commands::media::{
            AddAliasCommand, ChangeAliasCommand, ChangeMediaDetailsCommand, DeleteAliasCommand,
            GetAliasesCommand, GetLinkCommand, GetMediaDetailsCommand, SearchMediaCommand,
            UploadMediumCommand,
        },
        services::media::MediaService,
    },
    domain::{
        entities::{media::MediaType, secret::Claims},
        errors::media::MediaError,
    },
    infrastructure::web::{
        api::handlers::common::{MediumData, extract_medium},
        server::AppState,
    },
};

#[axum::debug_handler]
pub async fn upload(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    mut multipart: Multipart,
) -> Result<(), MediaError> {
    let mut opt_short_name: Option<String> = None;
    let mut opt_description: Option<String> = None;
    let mut opt_filename: Option<String> = None;
    let mut opt_content_type: Option<String> = None;
    let mut opt_bytes: Option<Bytes> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| MediaError::InternalError(e.to_string()))?
    {
        let field_name = field.name().ok_or(MediaError::UploadFailed(
            "Empty field detected.".to_string(),
        ))?;

        match field_name {
            "short_name" => {
                opt_short_name = Some(field.text().await.map_err(|_| {
                    MediaError::UploadFailed("Cannot read short name.".to_string())
                })?);
            }
            "description" => {
                opt_description = Some(field.text().await.map_err(|_| {
                    MediaError::UploadFailed("Cannot read description.".to_string())
                })?);
            }
            "file" => {
                if opt_filename.is_some() {
                    return Err(MediaError::UploadFailed(
                        "Only one media is allowed at a time.".to_string(),
                    ));
                }

                let MediumData {
                    filename,
                    content_type,
                    bytes,
                } = extract_medium(field).await?;

                opt_filename = Some(filename);
                opt_content_type = Some(content_type);
                opt_bytes = Some(bytes);
            }
            _ => {
                return Err(MediaError::UploadFailed(
                    "Unknown field detected.".to_string(),
                ));
            }
        };
    }

    let short_name =
        opt_short_name.ok_or_else(|| MediaError::UploadFailed("Missing short_name".to_string()))?;
    let description = opt_description
        .ok_or_else(|| MediaError::UploadFailed("Missing description".to_string()))?;
    let file_name =
        opt_filename.ok_or_else(|| MediaError::UploadFailed("Missing file".to_string()))?;
    let content_type = opt_content_type
        .ok_or_else(|| MediaError::UploadFailed("Missing content type".to_string()))?;
    let bytes =
        opt_bytes.ok_or_else(|| MediaError::UploadFailed("Missing file bytes".to_string()))?;

    let uploader_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| MediaError::InternalError("Cannot parse id".to_string()))?;

    let cmd = UploadMediumCommand {
        uploader_id,
        short_name,
        description,
        file_name,
        content_type,
        bytes,
    };

    match state.media_service.upload(cmd, &state.media_config).await {
        Ok(()) => Ok(()),
        Err(e) => Err(e),
    }
}

#[derive(Deserialize)]
pub struct MediaQuery {
    pub term: Option<String>,
    pub size: Option<u32>,
    pub skip: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetLinkResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    short_name: Option<String>,
    url: String,
    file_type: String,
}

#[derive(Serialize, Deserialize)]
pub struct SearchResponse {
    results: Vec<GetLinkResponse>,
}

#[derive(Serialize, Deserialize)]
pub struct GetMediaDetailsResponse {
    short_name: String,
    file_type: String,
    description: String,
    aliases: Vec<String>,
}

#[axum::debug_handler]
pub async fn search(
    State(state): State<Arc<AppState>>,
    Query(query): Query<MediaQuery>,
) -> Result<impl IntoResponse, MediaError> {
    let cmd = SearchMediaCommand {
        term: query.term,
        size: query.size.unwrap_or(0),
        skip: query.skip.unwrap_or(0),
    };
    match state.media_service.search(cmd).await {
        Ok(link_results) => Ok(Json(SearchResponse {
            results: link_results
                .into_iter()
                .map(|r| GetLinkResponse {
                    short_name: r.short_name,
                    url: r.url,
                    file_type: r.file_type,
                })
                .collect(),
        })),
        Err(e) => Err(e),
    }
}

#[axum::debug_handler]
pub async fn get_link(
    State(state): State<Arc<AppState>>,
    Path(short_name): Path<String>,
) -> Result<impl IntoResponse, MediaError> {
    let link = state
        .media_service
        .get_link(GetLinkCommand { short_name })
        .await?;

    Ok(Json(GetLinkResponse {
        short_name: link.short_name,
        url: link.url,
        file_type: link.file_type,
    }))
}

#[axum::debug_handler]
pub async fn get_media(
    State(state): State<Arc<AppState>>,
    Path(short_name): Path<String>,
) -> Result<impl IntoResponse, MediaError> {
    let link = state
        .media_service
        .get_link(GetLinkCommand { short_name })
        .await?;

    // Decode the storage layout from the hash field. Three layouts exist:
    //
    //  Regular media  hash = "<sha256>"
    //                 file = <media_dir>/<sha256[0..2]>/<sha256[2..4]>/<sha256><ext>
    //
    //  Post cover     hash = ".post.<post_id>.<sha256>"
    //                 file = <media_dir>/post/<uploader_id>/<sha256><ext>
    //                 NOTE: hash encodes post_id but the dir uses uploader_id!
    //
    //  User avatar    hash = ".avt.<user_id>.<sha256>"
    //                 file = <media_dir>/avt/<uploader_id>/<sha256><ext>
    //
    //  Series cover   hash = ".srs.<user_id>.<sha256>"  (fixed)
    //                 file = <media_dir>/srs/<uploader_id>/<sha256><ext>
    //                 Older rows were incorrectly stored with ".avt." prefix
    //                 due to a copy-paste bug; those are caught by the fallback.
    //
    // Hash-based reconstruction is the primary path. If the file is missing
    // (e.g. stale stored hash from the copy-paste bug), we fall back to
    // reroot_path() which strips the old media-dir prefix from the stored
    // `url` column and re-joins it under the current MEDIA_PATH.
    let extension = MediaType::from_str(&link.file_type)?.get_extension();

    let (file_path, content_hash) = if link.hash.starts_with('.') {
        // Special layout: ".<type>.<id>.<sha256>"
        // splitn(4, '.') keeps the sha256 tail (which has no dots) in one piece:
        // ["", "<type>", "<id>", "<sha256>"]
        //
        // IMPORTANT: parts[2] is the post_id for ".post.*" entries, NOT the
        // user_id used as the on-disk subdirectory.  Always use uploader_id
        // (fetched from the DB) as the directory name — it is the user_id for
        // post covers, avatars, and series covers.
        let parts: Vec<&str> = link.hash.splitn(4, '.').collect();
        if parts.len() < 4 {
            return Err(MediaError::FileNotFound);
        }
        let type_dir = parts[1]; // "post", "avt", or "srs"
        let sha256 = parts[3]; // plain SHA-256 hex
        let path = state
            .media_config
            .dir
            .join(type_dir)
            .join(link.uploader_id.to_string()) // user_id used at upload time
            .join(format!("{}{}", sha256, extension));
        (path, sha256.to_string())
    } else {
        // Regular layout: hash is a plain SHA-256 hex string.
        let path = state
            .media_config
            .dir
            .join(&link.hash[0..2])
            .join(&link.hash[2..4])
            .join(format!("{}{}", link.hash, extension));
        (path, link.hash.clone())
    };

    // Open the file for streaming — avoids loading the entire file into RAM,
    // which previously caused OOM kills on the 256 MB machine when many
    // images were requested concurrently.
    //
    // If the hash-derived path doesn't exist (e.g. a series cover whose hash
    // was written with the wrong ".avt." prefix), fall back to reconstructing
    // the path from the stored `url` column via reroot_path().
    let file = match fs::File::open(&file_path).await {
        Ok(f) => f,
        Err(_) => {
            let fallback =
                reroot_path(&link.url, &state.media_config.dir).ok_or(MediaError::FileNotFound)?;
            fs::File::open(&fallback)
                .await
                .map_err(|_| MediaError::FileNotFound)?
        }
    };

    let body = Body::from_stream(ReaderStream::new(file));

    // Use the plain SHA-256 as the ETag — stable content identifier
    // regardless of the storage layout.
    let etag = format!("\"{}\"", content_hash);

    Response::builder()
        .status(200)
        .header(header::CONTENT_TYPE, &link.file_type)
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        .header(header::ETAG, etag)
        .body(body)
        .map_err(|e| MediaError::InternalError(e.to_string()))
}

/// Re-root a stored file path under a new media directory.
///
/// The `stored_url` was written at upload time and may contain an old or
/// relative media directory prefix (e.g. `"./media/srs/2/abc.webp"` or
/// `"/old/path/to/media/post/11/abc.png"`).  This function finds the first
/// path component whose name matches `media_dir`'s own directory name and
/// returns `media_dir` joined with everything that came after it.
///
/// Returns `None` if the media directory name is not found in `stored_url`.
fn reroot_path(stored_url: &str, media_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    let dir_name = media_dir.file_name()?;
    let p = std::path::Path::new(stored_url);
    let mut after = std::path::PathBuf::new();
    let mut found = false;
    for component in p.components() {
        if found {
            after.push(component);
        } else if component.as_os_str() == dir_name {
            found = true;
        }
    }
    if found {
        Some(media_dir.join(after))
    } else {
        None
    }
}

#[axum::debug_handler]
pub async fn get_details(
    State(state): State<Arc<AppState>>,
    Path(short_name): Path<String>,
) -> Result<impl IntoResponse, MediaError> {
    let details = state
        .media_service
        .get_details(GetMediaDetailsCommand { short_name })
        .await?;

    Ok(Json(GetMediaDetailsResponse {
        short_name: details.short_name,
        description: details.description,
        file_type: details.file_type,
        aliases: details.aliases,
    }))
}

#[derive(Deserialize)]
pub struct ChangeDetailsPayload {
    pub new_short_name: Option<String>,
    pub description: Option<String>,
}

#[axum::debug_handler]
pub async fn change_details(
    State(state): State<Arc<AppState>>,
    Path(short_name): Path<String>,
    Json(payload): Json<ChangeDetailsPayload>,
) -> Result<(), MediaError> {
    let cmd = ChangeMediaDetailsCommand {
        short_name,
        new_short_name: payload.new_short_name,
        description: payload.description,
    };

    Ok(state.media_service.change_details(cmd).await?)
}

#[derive(Serialize, Deserialize)]
pub struct GetAliasesResponse {
    pub aliases: Vec<String>,
}

#[axum::debug_handler]
pub async fn get_aliases(
    State(state): State<Arc<AppState>>,
    Path(short_name): Path<String>,
) -> Result<impl IntoResponse, MediaError> {
    let cmd = GetAliasesCommand { short_name };

    Ok(Json(GetAliasesResponse {
        aliases: state.media_service.get_aliases(cmd).await?,
    }))
}

#[derive(Deserialize)]
pub struct AddAliasPayload {
    pub alias: String,
}

#[axum::debug_handler]
pub async fn add_alias(
    State(state): State<Arc<AppState>>,
    Path(short_name): Path<String>,
    Json(payload): Json<AddAliasPayload>,
) -> Result<(), MediaError> {
    let cmd = AddAliasCommand {
        short_name,
        alias: payload.alias,
    };

    Ok(state.media_service.add_alias(cmd).await?)
}

#[derive(Deserialize)]
pub struct ChangeAliasPayload {
    new_alias: String,
}

#[axum::debug_handler]
pub async fn change_alias(
    State(state): State<Arc<AppState>>,
    Path((short_name, alias)): Path<(String, String)>,
    Json(payload): Json<ChangeAliasPayload>,
) -> Result<(), MediaError> {
    let cmd = ChangeAliasCommand {
        short_name,
        old_alias: alias,
        new_alias: payload.new_alias,
    };

    Ok(state.media_service.change_alias(cmd).await?)
}

#[axum::debug_handler]
pub async fn delete_alias(
    State(state): State<Arc<AppState>>,
    Path((short_name, alias)): Path<(String, String)>,
) -> Result<(), MediaError> {
    let cmd = DeleteAliasCommand { short_name, alias };

    Ok(state.media_service.delete_alias(cmd).await?)
}
