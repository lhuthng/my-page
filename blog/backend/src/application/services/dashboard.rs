use crate::{
    application::commands::dashboard::*,
    domain::{entities::dashboard::*, errors::user::UserError},
};

#[async_trait::async_trait]
pub trait DashboardService {
    async fn get_overview(&self, cmd: GetOverviewCommand) -> Result<DashboardOverview, UserError>;
    async fn get_posts(
        &self,
        cmd: GetDashboardPostsCommand,
    ) -> Result<DashboardPostsResult, UserError>;
    async fn get_users(
        &self,
        cmd: GetDashboardUsersCommand,
    ) -> Result<DashboardUsersResult, UserError>;
    async fn get_projects(
        &self,
        cmd: GetDashboardProjectsCommand,
    ) -> Result<DashboardProjectsResult, UserError>;
    async fn update_tag(
        &self,
        cmd: UpdateDashboardTagCommand,
    ) -> Result<DashboardTagRecord, UserError>;
    async fn delete_tag(&self, cmd: DeleteDashboardTagCommand) -> Result<(), UserError>;
}
