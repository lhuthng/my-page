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
    /// Upload time in UTC, e.g. `2026-09-29 21:04:33`.
    ///
    /// Present on search results, where the caller is looking at a directory of
    /// files and wants to know when each arrived. The short-name resolution
    /// used to fetch bytes omits it — nothing on that path has a use for it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
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
    /// Upload time in UTC. `created_at`, never `updated_at` — see
    /// `LinkResult::created_at` for why the latter is not displayable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct GetAliasesResponse {
    pub aliases: Vec<String>,
}

/// A chapter's chunk table, with a fetchable URL per chunk.
///
/// This is what a player that decompresses chunks itself needs and the audio
/// URL cannot give it: where each chunk's raw bytes begin, how long each is, and
/// where to get it. Without the offsets a player could not turn a seek into a
/// chunk, and would be back to fetching the chapter from the start.
#[derive(Serialize, Deserialize)]
pub struct ChunkManifestResponse {
    pub version: u32,
    /// Raw bytes per chunk, so a player can predict a chunk from an offset
    /// rather than reading the whole table to find out.
    pub chunk_size: u64,
    /// Length of the chapter once reassembled.
    pub raw_size: u64,
    pub level: i32,
    pub content_type: String,
    pub chunks: Vec<ChunkResponse>,
}

#[derive(Serialize, Deserialize)]
pub struct ChunkResponse {
    pub index: usize,
    /// Where this chunk's raw bytes start within the chapter.
    pub offset: u64,
    /// Raw (decompressed) length of this chunk.
    pub raw: u64,
    /// Length of the zstd frame as stored.
    pub compressed: u64,
    /// SHA-256 of the chunk's raw bytes, so a player can check its own
    /// decompression instead of assuming it worked.
    pub sha256: String,
    /// Where to fetch the compressed chunk. Signed and expiring when the audio
    /// lives on the bucket, and this backend's own chunk route when it does not.
    pub url: String,
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
