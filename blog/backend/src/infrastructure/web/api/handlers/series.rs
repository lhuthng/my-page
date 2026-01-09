use std::sync::Arc;

use axum::{
    Extension, Json,
    body::Bytes,
    extract::{Multipart, Query, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::{
    application::{
        commands::series::{GetSeriesCommand, NewSeriesCommand},
        services::series::SeriesService,
    },
    domain::{
        entities::{media::MediumDetails, secret::Claims},
        errors::{media::MediaError, series::SeriesError},
    },
    infrastructure::web::{
        api::handlers::common::{MediumData, extract_medium},
        server::AppState,
    },
};

#[derive(Serialize, Deserialize)]
pub struct SeriesResponse {
    pub title: String,
    pub slug: String,
}

#[derive(Serialize, Deserialize)]
pub struct GetSeriesResponse {
    pub series: Vec<SeriesResponse>,
}

pub async fn get_series(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, SeriesError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| SeriesError::InternalError("Cannot parse id".to_string()))?;

    let series = state
        .series_service
        .get_series(GetSeriesCommand { user_id })
        .await?;

    let series = series
        .into_iter()
        .map(|series| SeriesResponse {
            title: series.title,
            slug: series.slug,
        })
        .collect();

    Ok(Json(GetSeriesResponse { series }))
}

#[derive(Debug, Deserialize)]
pub struct NewSeriesBody {
    pub title: String,
    pub slug: String,
    pub description: String,
    pub cover_image_slug: Option<String>,
}

#[axum::debug_handler]
pub async fn new_series(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, SeriesError> {
    let uploader_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| SeriesError::InternalError("Cannot parse id".to_string()))?;

    let mut opt_title: Option<String> = None;
    let mut opt_slug: Option<String> = None;
    let mut opt_description: Option<String> = None;
    let mut opt_filename: Option<String> = None;
    let mut opt_content_type: Option<String> = None;
    let mut opt_bytes: Option<Bytes> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| SeriesError::InternalError(e.to_string()))?
    {
        let field_name = field.name().ok_or(MediaError::UploadFailed(
            "Empty field detected.".to_string(),
        ))?;

        match field_name {
            "file" => {
                if opt_filename.is_some() {
                    return Err(SeriesError::Media(MediaError::UploadFailed(
                        "Only one media is allowed at a time.".to_string(),
                    )));
                }
                let medium = extract_medium(field).await?;
                opt_filename = Some(medium.filename);
                opt_content_type = Some(medium.content_type);
                opt_bytes = Some(medium.bytes);
            }
            "title" => {
                opt_title = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| SeriesError::InternalError(e.to_string()))?,
                );
            }
            "slug" => {
                opt_slug = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| SeriesError::InternalError(e.to_string()))?,
                );
            }
            "description" => {
                opt_description = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| SeriesError::InternalError(e.to_string()))?,
                );
            }
            _ => {}
        }
    }

    let cover_image: Option<MediumDetails> = match (opt_filename, opt_content_type, opt_bytes) {
        (Some(filename), Some(content_type), Some(bytes)) => Some(MediumDetails {
            filename,
            content_type,
            bytes,
        }),
        _ => None,
    };

    let title = opt_title.ok_or_else(|| MediaError::UploadFailed("Missing title".to_string()))?;
    let slug = opt_slug.ok_or_else(|| MediaError::UploadFailed("Missing slug".to_string()))?;
    let description = opt_description
        .ok_or_else(|| MediaError::UploadFailed("Missing description".to_string()))?;
    state
        .series_service
        .new_series(
            NewSeriesCommand {
                title,
                slug,
                description,
                user_id: uploader_id,
                cover_image,
            },
            &state.media_config,
        )
        .await?;
    Ok(())
}
