// Generic multipart intake shared by the game and project handlers: the
// `game_data`/`project_data` JSON body, indexed inline files and short
// names, an optional demo zip, and the cover fields. Error construction is
// injected so each aggregate keeps its own error type.
use std::collections::HashMap;

use axum::body::Bytes;
use axum::extract::multipart::Multipart;

use crate::application::commands::media::UploadMediaWithoutDescriptionCommand;
use crate::application::services::media::MediaService;
use crate::domain::errors::media::MediaError;
use crate::infrastructure::web::server::AppState;

use super::cover::{CreateCoverUpload, try_collect_create_cover_field};

#[derive(Clone, Debug)]
pub struct FileData {
    pub file_name: String,
    pub bytes: Bytes,
    pub content_type: String,
}

pub struct ParsedMultipart<T> {
    pub data: T,
    pub files: HashMap<usize, FileData>,
    pub short_names: HashMap<usize, String>,
    pub demo_zip: Option<Bytes>,
    pub create_cover: CreateCoverUpload,
}

pub async fn parse_multipart<T, E>(
    mut multipart: Multipart,
    data_field: &str,
    no_data_message: &str,
    internal_error: impl Fn(String) -> E,
    upload_error: impl Fn(String) -> E,
) -> Result<ParsedMultipart<T>, E>
where
    T: for<'de> serde::Deserialize<'de>,
    E: From<MediaError>,
{
    let mut data: Option<T> = None;
    let mut files = HashMap::<usize, FileData>::new();
    let mut short_names = HashMap::<usize, String>::new();
    let mut demo_zip: Option<Bytes> = None;
    let mut create_cover = CreateCoverUpload::default();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| internal_error(e.to_string()))?
    {
        let field_name = field
            .name()
            .ok_or_else(|| upload_error("Empty field found.".to_string()))?
            .to_string();

        if field_name == data_field {
            let bytes = field
                .bytes()
                .await
                .map_err(|e| upload_error(e.to_string()))?;
            data =
                Some(serde_json::from_slice::<T>(&bytes).map_err(|e| upload_error(e.to_string()))?);
        } else if field_name == "demo_zip" {
            if demo_zip.is_some() {
                return Err(upload_error("Only one demo zip is allowed.".to_string()));
            }
            demo_zip = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| upload_error(e.to_string()))?,
            );
        } else if let Some(index_str) = field_name.strip_prefix("file_") {
            let index = index_str
                .parse::<usize>()
                .map_err(|_| upload_error("Invalid file index".to_string()))?;
            if files.contains_key(&index) {
                return Err(upload_error(format!("Duplicate file index {index}")));
            }
            let file_name = field
                .file_name()
                .ok_or_else(|| upload_error("Cannot read file name.".to_string()))?
                .to_string();
            let content_type = field
                .content_type()
                .ok_or_else(|| upload_error(format!("Cannot read content type of {}.", file_name)))?
                .to_string();
            let bytes = field
                .bytes()
                .await
                .map_err(|_| upload_error(format!("Cannot read {file_name}")))?;
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
                .map_err(|_| upload_error("Invalid short name index".to_string()))?;
            short_names.insert(
                index,
                field
                    .text()
                    .await
                    .map_err(|_| upload_error("Cannot read short name".to_string()))?,
            );
        } else if try_collect_create_cover_field(&field_name, field, &mut create_cover).await? {
        }
    }

    Ok(ParsedMultipart {
        data: data.ok_or_else(|| upload_error(no_data_message.to_string()))?,
        files,
        short_names,
        demo_zip,
        create_cover,
    })
}

pub async fn upload_inline_media<E>(
    state: &AppState,
    uploader_id: i64,
    number_of_files: usize,
    files: &HashMap<usize, FileData>,
    short_name_map: &HashMap<usize, String>,
    upload_error: impl Fn(String) -> E,
) -> Result<(), E>
where
    E: From<MediaError>,
{
    let mut short_names = Vec::<String>::new();
    let mut file_names = Vec::<String>::new();
    let mut content_types = Vec::<String>::new();
    let mut bytes_list = Vec::<Bytes>::new();

    for i in 1..=number_of_files {
        let file = files
            .get(&i)
            .ok_or_else(|| upload_error(format!("Cannot locate file_{i}")))?;
        let short_name = short_name_map
            .get(&i)
            .ok_or_else(|| upload_error(format!("Cannot locate short_name_{i}")))?;
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
