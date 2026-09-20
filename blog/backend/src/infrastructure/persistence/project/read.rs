// Project read methods: by-slug, details, post id, listings, tag feed.
use std::collections::HashMap;

use sqlx::Row;

use crate::application::{
    commands::project::{
        GetFeaturedProjectsCommand, GetLatestProjectsCommand, GetProjectBySlugCommand,
        GetProjectDetailsCommand, GetProjectPostIdCommand,
    },
    services::project::ProjectService,
};
use crate::domain::entities::project::{ProjectSnapshot, ProjectSummary};
use crate::domain::errors::project::ProjectError;

use super::mapping;
use super::rows::{ProjectContentRow, ProjectLinkRow, ProjectSnapshotRow, ProjectTagRow};
use super::ProjectServiceImpl;

#[async_trait::async_trait]
impl ProjectService for ProjectServiceImpl {
    async fn get_project_by_slug(
        &self,
        cmd: GetProjectBySlugCommand,
    ) -> Result<Project, ProjectError> {
        let row = sqlx::query_as::<_, ProjectContentRow>(
            r#"
            SELECT
                projects.id AS project_id,
                posts.id AS post_id,
                posts.user_id,
                users.username AS author_slug,
                user_meta.display_name AS author_name,
                'media/i/' || avatar.short_name AS author_avatar_url,
                posts.title,
                posts.slug,
                posts.excerpt,
                posts.content,
                posts.published_at,
                posts.updated_at,
                'media/i/' || cover.short_name AS cover_url,
                cover.file_type AS cover_media_type,
                'media/i/' || video.short_name AS cover_video_url,
                video.file_type AS cover_video_type,
                posts.og_image_seconds,
                projects.demo_type,
                projects.demo_entry_path,
                projects.demo_width,
                projects.demo_height,
                projects.demo_config,
                projects.demo_url,
                projects.delegate_game_id,
                projects.inherit_thumbnail,
                projects.inherit_tags
            FROM projects
            JOIN posts ON posts.id = projects.post_id
            JOIN users ON users.id = posts.user_id
            JOIN user_meta ON user_meta.user_id = users.id
            LEFT JOIN media cover ON cover.id = posts.cover_media_id
            LEFT JOIN media avatar ON avatar.id = user_meta.avatar_image_id
            LEFT JOIN media video ON video.short_name = '.post.' || posts.id
            WHERE posts.slug = ? AND posts.status = 'published' AND posts.deleted_at IS NULL
            "#,
        )
        .bind(&cmd.slug)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(ProjectError::ProjectNotFound)?;

        self.project_from_row(row, cmd.as_id).await
    }

    async fn get_project_details(
        &self,
        cmd: GetProjectDetailsCommand,
    ) -> Result<Project, ProjectError> {
        let row = sqlx::query_as::<_, ProjectContentRow>(
            r#"
            SELECT
                projects.id AS project_id,
                posts.id AS post_id,
                posts.user_id,
                users.username AS author_slug,
                user_meta.display_name AS author_name,
                'media/i/' || avatar.short_name AS author_avatar_url,
                posts.title,
                posts.slug,
                posts.excerpt,
                posts.content,
                posts.published_at,
                posts.updated_at,
                media.url AS cover_url,
                media.file_type AS cover_media_type,
                'media/i/' || video.short_name AS cover_video_url,
                video.file_type AS cover_video_type,
                posts.og_image_seconds,
                projects.demo_type,
                projects.demo_entry_path,
                projects.demo_width,
                projects.demo_height,
                projects.demo_config,
                projects.demo_url,
                projects.delegate_game_id,
                projects.inherit_thumbnail,
                projects.inherit_tags
            FROM projects
            JOIN posts ON posts.id = projects.post_id
            JOIN users ON users.id = posts.user_id
            JOIN user_meta ON user_meta.user_id = users.id
            LEFT JOIN media ON media.id = posts.cover_media_id
            LEFT JOIN media avatar ON avatar.id = user_meta.avatar_image_id
            LEFT JOIN media video ON video.short_name = '.post.' || posts.id
            WHERE projects.id = ?
            "#,
        )
        .bind(cmd.project_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(ProjectError::ProjectNotFound)?;

        if let Some(user_id) = cmd.required_author_id
            && user_id != row.user_id
        {
            return Err(ProjectError::Forbidden);
        }

        self.project_from_row(row, Some(cmd.viewing_user_id)).await
    }

    async fn get_project_post_id(&self, cmd: GetProjectPostIdCommand) -> Result<i64, ProjectError> {
        let row: Option<(i64, i64)> = sqlx::query_as(
            r#"
            SELECT posts.id, posts.user_id
            FROM projects
            JOIN posts ON posts.id = projects.post_id
            WHERE projects.id = ?
            "#,
        )
        .bind(cmd.project_id)
        .fetch_optional(&self.pool)
        .await?;

        let (post_id, user_id) = row.ok_or(ProjectError::ProjectNotFound)?;
        if let Some(required) = cmd.required_author_id
            && required != user_id
        {
            return Err(ProjectError::Forbidden);
        }
        Ok(post_id)
    }

    async fn get_latest_project_snapshots(
        &self,
        cmd: GetLatestProjectsCommand,
    ) -> Result<ProjectSnapshotPage, ProjectError> {
        let mut where_parts = Vec::<String>::new();
        if cmd.public_only {
            where_parts.push("posts.status = 'published' AND deleted_at IS NULL".to_string());
        }
        if cmd.required_author_id.is_some() {
            where_parts.push("posts.user_id = ?".to_string());
        }
        let where_clause = if where_parts.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_parts.join(" AND "))
        };

        let sql = format!(
            r#"
            SELECT
                projects.id AS project_id,
                posts.id AS post_id,
                posts.title,
                posts.slug,
                posts.excerpt,
                users.username AS author_slug,
                user_meta.display_name AS author_name,
                posts.status,
                'media/i/' || media.short_name AS url,
                media.file_type AS cover_media_type,
                projects.demo_type,
                post_stats.views,
                post_stats.likes,
                post_stats.comments_count,
                posts.reading_time_minutes
            FROM projects
            JOIN posts ON posts.id = projects.post_id
            JOIN users ON users.id = posts.user_id
            JOIN user_meta ON user_meta.user_id = posts.user_id
            JOIN post_stats ON post_stats.post_id = posts.id
            LEFT JOIN media ON media.id = posts.cover_media_id
            {}
            ORDER BY posts.created_at DESC
            LIMIT ?
            OFFSET ?
            "#,
            where_clause
        );

        let mut query = sqlx::query_as::<_, ProjectSnapshotRow>(&sql);
        if let Some(user_id) = cmd.required_author_id {
            query = query.bind(user_id);
        }
        let rows = query
            .bind(cmd.limit + 1)
            .bind(cmd.offset)
            .fetch_all(&self.pool)
            .await?;

        let mut projects = self.hydrate_project_rows(rows).await?;
        let has_more = projects.len() as i64 > cmd.limit;
        if has_more {
            projects.truncate(cmd.limit as usize);
        }

        Ok(ProjectSnapshotPage { projects, has_more })
    }

    async fn get_featured_project_snapshots(
        &self,
        cmd: GetFeaturedProjectsCommand,
    ) -> Result<Vec<ProjectSnapshot>, ProjectError> {
        let rows = sqlx::query_as::<_, ProjectSnapshotRow>(
            r#"
            SELECT
                projects.id AS project_id,
                posts.id AS post_id,
                posts.title,
                posts.slug,
                posts.excerpt,
                users.username AS author_slug,
                user_meta.display_name AS author_name,
                posts.status,
                'media/i/' || media.short_name AS url,
                media.file_type AS cover_media_type,
                projects.demo_type,
                post_stats.views,
                post_stats.likes,
                post_stats.comments_count,
                posts.reading_time_minutes
            FROM projects
            JOIN posts ON posts.id = projects.post_id
            JOIN users ON users.id = posts.user_id
            JOIN user_meta ON user_meta.user_id = posts.user_id
            JOIN post_stats ON post_stats.post_id = posts.id
            LEFT JOIN media ON media.id = posts.cover_media_id
            WHERE posts.status = 'published' AND posts.deleted_at IS NULL AND posts.is_featured = 1
            ORDER BY posts.created_at DESC
            LIMIT ?
            "#,
        )
        .bind(cmd.limit)
        .fetch_all(&self.pool)
        .await?;

        self.hydrate_project_rows(rows).await
    }

    async fn get_project_snapshots_by_tag(
        &self,
        cmd: GetProjectsByTagCommand,
    ) -> Result<Vec<ProjectSnapshot>, ProjectError> {
        let rows = sqlx::query_as::<_, ProjectSnapshotRow>(
            r#"
            SELECT
                projects.id AS project_id,
                posts.id AS post_id,
                posts.title,
                posts.slug,
                posts.excerpt,
                users.username AS author_slug,
                user_meta.display_name AS author_name,
                posts.status,
                'media/i/' || media.short_name AS url,
                media.file_type AS cover_media_type,
                projects.demo_type,
                post_stats.views,
                post_stats.likes,
                post_stats.comments_count,
                posts.reading_time_minutes
            FROM projects
            JOIN posts ON posts.id = projects.post_id
            JOIN users ON users.id = posts.user_id
            JOIN user_meta ON user_meta.user_id = posts.user_id
            JOIN post_stats ON post_stats.post_id = posts.id
            LEFT JOIN media ON media.id = posts.cover_media_id
            WHERE posts.status = 'published' AND posts.deleted_at IS NULL
                AND EXISTS (
                    SELECT 1
                    FROM post_tags
                    JOIN tags ON tags.id = post_tags.tag_id
                    WHERE post_tags.post_id = posts.id AND tags.slug = ?1
                )
            ORDER BY posts.created_at DESC
            LIMIT ?2 OFFSET ?3
            "#,
        )
        .bind(cmd.slug)
        .bind(cmd.limit)
        .bind(cmd.offset)
        .fetch_all(&self.pool)
        .await?;

        self.hydrate_project_rows(rows).await
    }
}
