use crate::{
    application::commands,
    domain::{entities, errors},
};

#[async_trait::async_trait]
pub trait ProjectService {
    async fn new_project(
        &self,
        cmd: commands::project::NewProjectCommand,
    ) -> Result<i64, errors::project::ProjectError>;
    async fn update_project(
        &self,
        cmd: commands::project::UpdateProjectCommand,
    ) -> Result<(), errors::project::ProjectError>;
    async fn get_project_by_slug(
        &self,
        cmd: commands::project::GetProjectBySlugCommand,
    ) -> Result<entities::project::Project, errors::project::ProjectError>;
    async fn get_project_details(
        &self,
        cmd: commands::project::GetProjectDetailsCommand,
    ) -> Result<entities::project::Project, errors::project::ProjectError>;
    async fn get_project_post_id(
        &self,
        cmd: commands::project::GetProjectPostIdCommand,
    ) -> Result<i64, errors::project::ProjectError>;
    async fn get_latest_project_snapshots(
        &self,
        cmd: commands::project::GetLatestProjectsCommand,
    ) -> Result<Vec<entities::project::ProjectSnapshot>, errors::project::ProjectError>;
    async fn get_project_snapshots_by_tag(
        &self,
        cmd: commands::project::GetProjectsByTagCommand,
    ) -> Result<Vec<entities::project::ProjectSnapshot>, errors::project::ProjectError>;
}
