// Dashboard persistence adapter: implements
// `application::services::dashboard::DashboardService` against SQLite.
use sqlx::SqlitePool;

mod overview;
mod rows;
mod shared;
mod tags;
mod views;

pub struct DashboardServiceImpl {
    pub pool: SqlitePool,
}

impl DashboardServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

// `DashboardService` port implementation: thin delegation to the inherent
// methods in the sibling files (inherent candidates win over trait
// candidates in method resolution, so `self.<method>` calls the local one).
use crate::application::commands::dashboard::*;
use crate::application::services::dashboard::DashboardService;
use crate::domain::entities::dashboard::*;
use crate::domain::errors::user::UserError;

#[async_trait::async_trait]
impl DashboardService for DashboardServiceImpl {
    async fn get_overview(&self, cmd: GetOverviewCommand) -> Result<DashboardOverview, UserError> {
        self.get_overview(cmd).await
    }
    async fn get_posts(
        &self,
        cmd: GetDashboardPostsCommand,
    ) -> Result<DashboardPostsResult, UserError> {
        self.get_posts(cmd).await
    }
    async fn get_users(
        &self,
        cmd: GetDashboardUsersCommand,
    ) -> Result<DashboardUsersResult, UserError> {
        self.get_users(cmd).await
    }
    async fn get_projects(
        &self,
        cmd: GetDashboardProjectsCommand,
    ) -> Result<DashboardProjectsResult, UserError> {
        self.get_projects(cmd).await
    }
    async fn update_tag(
        &self,
        cmd: UpdateDashboardTagCommand,
    ) -> Result<DashboardTagRecord, UserError> {
        self.update_tag(cmd).await
    }
    async fn delete_tag(&self, cmd: DeleteDashboardTagCommand) -> Result<(), UserError> {
        self.delete_tag(cmd).await
    }
}
