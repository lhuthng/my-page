// Project feature handlers, split by responsibility. This module is the
// public surface: route handlers for the router plus the wire types their
// signatures use. Internal helpers stay inside their sub-modules.
mod dto;
mod publish;
mod read;
mod response;
mod trash;
mod update;
mod write;

pub use dto::{
    CheckQuery, CheckResponse, DeleteProjectQuery, FeaturedProjectsQuery,
    FeaturedProjectsResponse, LatestProjectsQuery, LatestProjectsResponse, ProjectCard,
    ProjectStats, SetProjectFeaturedBody, StartJsDosUploadRequest, StartJsDosUploadResponse,
    JsDosUploadResponse, CompleteJsDosUploadResponse,
};
pub use publish::{publish_project, set_project_featured};
pub use read::{
    check_project, get_all_projects, get_featured_projects, get_project_by_slug,
    get_project_details, get_latest_projects,
};
pub use response::{DelegatedGameResponse, ProjectResponse, UpdateProjectResponse};
pub use trash::{delete_project_draft, purge_project_now, restore_project};
pub use update::update_project;
pub use write::{change_cover, new_project};
