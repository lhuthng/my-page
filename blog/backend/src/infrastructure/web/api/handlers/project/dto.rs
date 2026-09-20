// Wire types for the project feature: request bodies, query params, and
// response shapes. Handlers do not declare structs inline.
use serde::{Deserialize, Serialize};

use crate::domain::entities::project::ProjectLink;

#[derive(Deserialize)]
pub struct CheckQuery {
    pub slug: Option<String>,
}

#[derive(Serialize)]
pub struct CheckResponse {
    exists: bool,
}

#[derive(Deserialize)]
pub(super) struct ProjectData {
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
    delegate_game_id: Option<i64>,
    inherit_thumbnail: Option<bool>,
    inherit_tags: Option<bool>,
}

#[derive(Deserialize)]
pub(super) struct ProjectPatchData {
    title: Option<String>,
    slug: Option<String>,
    excerpt: Option<String>,
    content: Option<String>,
    tags: Option<Vec<String>>,
    links: Option<Vec<ProjectLink>>,
    number_of_files: usize,
    demo_type: Option<String>,
    demo_width: Option<String>,
    demo_height: Option<String>,
    demo_config: Option<String>,
    demo_url: Option<String>,
    delegate_game_id: Option<i64>,
    inherit_thumbnail: Option<bool>,
    inherit_tags: Option<bool>,
    og_image_seconds: Option<i64>,
    /// Optional optimistic-lock token; see `UpdatePostCommand`.
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
    pub project_id: i64,
    pub file_name: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub bundle_url: String,
}

#[derive(Deserialize)]
pub struct DeleteProjectQuery {
    pub reason: Option<String>,
    pub detail: Option<String>,
    pub force: Option<bool>,
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
pub struct SetProjectFeaturedBody {
    pub is_featured: bool,
}

