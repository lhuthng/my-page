use crate::domain::entities::project::ProjectLink;

pub struct NewProjectCommand {
    pub post_id: i64,
    pub demo_type: String,
    pub demo_entry_path: String,
    pub demo_width: Option<String>,
    pub demo_height: Option<String>,
    pub demo_config: Option<String>,
    pub demo_url: Option<String>,
    pub links: Vec<ProjectLink>,
}

pub struct UpdateProjectCommand {
    pub project_id: i64,
    pub user_id: i64,
    pub demo_type: Option<String>,
    pub demo_entry_path: Option<String>,
    pub demo_width: Option<String>,
    pub demo_height: Option<String>,
    pub demo_config: Option<String>,
    pub demo_url: Option<String>,
    pub links: Option<Vec<ProjectLink>>,
}

pub struct GetProjectBySlugCommand {
    pub slug: String,
    pub as_id: Option<i64>,
}

pub struct GetProjectDetailsCommand {
    pub project_id: i64,
    pub viewing_user_id: i64,
    pub required_author_id: Option<i64>,
}

pub struct GetProjectPostIdCommand {
    pub project_id: i64,
    pub required_author_id: Option<i64>,
}

pub struct GetLatestProjectsCommand {
    pub limit: i64,
    pub offset: i64,
    pub public_only: bool,
    pub required_author_id: Option<i64>,
}

pub struct GetProjectsByTagCommand {
    pub slug: String,
    pub limit: i64,
    pub offset: i64,
}
