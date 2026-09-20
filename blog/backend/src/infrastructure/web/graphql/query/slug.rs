// Slug availability checks.
use std::collections::HashMap;

use async_graphql::{Context, Object};
use sqlx::SqlitePool;

use super::super::helpers::{
    DASHBOARD_POST_COLUMNS, DASHBOARD_POST_JOINS, attach_tags_to_posts, attach_tags_to_projects,
};
use super::super::rows::{
    CategoryRow, CommentRow, DashboardPostRow, DashboardProjectRow, GqlPostRow, GrowthDayRow,
    MediaRow, PostDetailRow, RoleCountRow, SeriesPostRow, SeriesRow, TagRow, UserInfoRow, UserRow,
};
use super::super::types::{
    CategoryConnection, CommentConnection, DashboardPostConnection, DbStats, GqlCategory,
    GqlComment, GqlDashboardOverview, GqlDashboardPost, GqlDashboardProject, GqlDashboardUser,
    GqlGrowthPoint, GqlMedia, GqlPost, GqlPostDetail, GqlRoleCounts, GqlSeries, GqlSeriesPost,
    GqlTag, GqlUser, MediaConnection, PostConnection, ProjectConnection, SeriesConnection,
    TagConnection, UserConnection,
};

use super::QueryRoot;

#[Object]
impl QueryRoot {
    async fn check_slug(&self, ctx: &Context<'_>, slug: String) -> async_graphql::Result<bool> {
        let pool = ctx.data::<SqlitePool>()?;

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE slug = ?")
            .bind(&slug)
            .fetch_one(pool)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(count == 0)
    }

    async fn check_project_slug(
        &self,
        ctx: &Context<'_>,
        slug: String,
    ) -> async_graphql::Result<bool> {
        let pool = ctx.data::<SqlitePool>()?;

        let count: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM posts p JOIN projects ON projects.post_id = p.id WHERE p.slug = ?"#,
        )
        .bind(&slug)
        .fetch_one(pool).await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(count == 0)
    }
}
