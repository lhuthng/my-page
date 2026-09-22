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
    pub short_name: Option<String>,
    pub url: String,
    pub file_type: String,
}

#[derive(Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<GetLinkResponse>,
}

#[derive(Serialize, Deserialize)]
pub struct GetMediaDetailsResponse {
    pub short_name: String,
    pub file_type: String,
    pub description: String,
    pub aliases: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct GetAliasesResponse {
    pub aliases: Vec<String>,
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
pub struct ChangeAliasPayload {
    pub new_alias: String,
}
