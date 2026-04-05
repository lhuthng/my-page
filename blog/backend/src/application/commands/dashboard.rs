pub struct GetOverviewCommand {
    pub user_id: i64,
    pub role: String,
}

pub struct GetDashboardPostsCommand {
    pub user_id: i64,
    pub role: String,
    pub search: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

pub struct GetDashboardUsersCommand {
    pub role: String,
    pub search: Option<String>,
    pub role_filter: Option<String>,
    pub limit: i64,
    pub offset: i64,
}
