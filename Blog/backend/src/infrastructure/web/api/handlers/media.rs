use std::sync::Arc;

use axum::{
    Extension, Json,
    body::Bytes,
    extract::{Multipart, Path, Query, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::{
    application::{
        commands::media::{
            AddAliasCommand, ChangeAliasCommand, ChangeMediaDetailsCommand, DeleteAliasCommand,
            GetAliasesCommand, GetLinkCommand, GetMediaDetailsCommand, SearchMediaCommand,
            UploadMediumCommand,
        },
        services::media::MediaService,
    },
    domain::{entities::secret::Claims, errors::media::MediaError},
    infrastructure::web::server::AppState,
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

                let file_name = field
                    .file_name()
                    .ok_or(MediaError::UploadFailed(
                        "Cannot read file name.".to_string(),
                    ))?
                    .to_string();

                let content_type = field
                    .content_type()
                    .ok_or(MediaError::UploadFailed(format!(
                        "Cannot read content type of {}.",
                        file_name
                    )))?
                    .to_string();

                let bytes = field.bytes().await.map_err(|_| {
                    MediaError::UploadFailed(format!("Cannot read bytes of {}.", file_name))
                })?;

                opt_filename = Some(file_name);
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
                .iter()
                .map(|r| GetLinkResponse {
                    short_name: r.short_name.clone(),
                    url: r.url.clone(),
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
    }))
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
