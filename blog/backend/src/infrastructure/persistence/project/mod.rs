// Project persistence adapter: implements
// `application::services::project::ProjectService` against SQLite.
use sqlx::SqlitePool;

mod mapping;
mod read;
mod rows;
mod write;

pub struct ProjectServiceImpl {
    pub pool: SqlitePool,
}

impl ProjectServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

// `ProjectService` port implementation: thin delegation to the inherent
// methods in the sibling files (inherent candidates win over trait
// candidates in method resolution, so `self.<method>` calls the local one).
use crate::application::commands;
use crate::application::services::project::ProjectService;
use crate::domain::{entities, errors};

#[async_trait::async_trait]
impl ProjectService for ProjectServiceImpl {
    async fn new_project(
        &self,
        cmd: commands::project::NewProjectCommand,
    ) -> Result<i64, errors::project::ProjectError> {
        self.new_project(cmd).await
    }
    async fn update_project(
        &self,
        cmd: commands::project::UpdateProjectCommand,
    ) -> Result<(), errors::project::ProjectError> {
        self.update_project(cmd).await
    }
    async fn get_project_by_slug(
        &self,
        cmd: commands::project::GetProjectBySlugCommand,
    ) -> Result<entities::project::Project, errors::project::ProjectError> {
        self.get_project_by_slug(cmd).await
    }
    async fn get_project_details(
        &self,
        cmd: commands::project::GetProjectDetailsCommand,
    ) -> Result<entities::project::Project, errors::project::ProjectError> {
        self.get_project_details(cmd).await
    }
    async fn get_project_post_id(
        &self,
        cmd: commands::project::GetProjectPostIdCommand,
    ) -> Result<i64, errors::project::ProjectError> {
        self.get_project_post_id(cmd).await
    }
    async fn get_latest_project_snapshots(
        &self,
        cmd: commands::project::GetLatestProjectsCommand,
    ) -> Result<entities::project::ProjectSnapshotPage, errors::project::ProjectError> {
        self.get_latest_project_snapshots(cmd).await
    }
    async fn set_project_featured(
        &self,
        cmd: commands::project::SetFeaturedProjectCommand,
    ) -> Result<(), errors::project::ProjectError> {
        self.set_project_featured(cmd).await
    }
    async fn get_featured_project_snapshots(
        &self,
        cmd: commands::project::GetFeaturedProjectsCommand,
    ) -> Result<Vec<entities::project::ProjectSnapshot>, errors::project::ProjectError> {
        self.get_featured_project_snapshots(cmd).await
    }
    async fn get_project_snapshots_by_tag(
        &self,
        cmd: commands::project::GetProjectsByTagCommand,
    ) -> Result<Vec<entities::project::ProjectSnapshot>, errors::project::ProjectError> {
        self.get_project_snapshots_by_tag(cmd).await
    }
}
