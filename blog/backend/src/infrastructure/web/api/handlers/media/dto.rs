// Media wire types: query params, payloads, and response shapes.
use serde::{Deserialize, Serialize};

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

#[derive(Deserialize)]
pub struct ChangeDetailsPayload {
    pub new_short_name: Option<String>,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct AddAliasPayload {
    pub alias: String,
}

#[derive(Deserialize)]
pub struct ChangeAliasPayload {
    new_alias: String,
}

