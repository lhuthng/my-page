use std::collections::HashMap;

use sqlx::{FromRow, SqlitePool};

use crate::{
    application::{
        commands::project::{
            GetLatestProjectsCommand, GetProjectBySlugCommand, GetProjectDetailsCommand,
            GetProjectPostIdCommand, GetProjectsByTagCommand, NewProjectCommand,
            UpdateProjectCommand,
        },
        services::project::ProjectService,
    },
    domain::{
        entities::{
            post::PostStats,
            project::{Project, ProjectDemo, ProjectLink, ProjectSnapshot},
        },
        errors::project::ProjectError,
    },
    infrastructure::persistence::post::{MediumUsageWithNameRow, TagRow},
};

pub struct ProjectServiceImpl {
    pub pool: SqlitePool,
}

impl ProjectServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, FromRow)]
struct ProjectSnapshotRow {
    project_id: i64,
    post_id: i64,
    title: String,
    slug: String,
    excerpt: String,
    author_name: String,
    author_slug: String,
    status: String,
    url: Option<String>,
    demo_type: String,
    views: i64,
    likes: i64,
    comments_count: i64,
}

#[derive(Debug, FromRow)]
struct ProjectContentRow {
    project_id: i64,
    post_id: i64,
    user_id: i64,
    author_name: String,
    author_slug: String,
    author_avatar_url: Option<String>,
    title: String,
    slug: String,
    excerpt: String,
    content: String,
    draft: String,
    published_at: Option<String>,
    updated_at: Option<String>,
    cover_url: Option<String>,
    demo_type: String,
    demo_entry_path: String,
    demo_width: Option<String>,
    demo_height: Option<String>,
    demo_config: Option<String>,
}

#[derive(Debug, FromRow)]
struct ProjectLinkRow {
    label: String,
    url: String,
}

#[derive(Debug, FromRow)]
struct ProjectTagRow {
    project_id: i64,
    tag_name: String,
    tag_slug: String,
}

impl ProjectSnapshotRow {
    fn into_snapshot(self, tag_names: Vec<String>, tag_slugs: Vec<String>) -> ProjectSnapshot {
        ProjectSnapshot {
            id: self.project_id,
            post_id: self.post_id,
            title: self.title,
            slug: self.slug,
            tag_names,
            tag_slugs,
            excerpt: self.excerpt,
            author_name: self.author_name,
            author_slug: self.author_slug,
            status: self.status,
            url: self.url,
            demo_type: self.demo_type,
            stats: PostStats {
                views: self.views,
                likes: self.likes,
                comments: self.comments_count,
            },
        }
    }
}

impl ProjectServiceImpl {
    async fn hydrate_project_rows(
        &self,
        rows: Vec<ProjectSnapshotRow>,
    ) -> Result<Vec<ProjectSnapshot>, ProjectError> {
        if rows.is_empty() {
            return Ok(vec![]);
        }

        let placeholders = rows.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!(
            r#"
            SELECT projects.id AS project_id, tags.name AS tag_name, tags.slug AS tag_slug
            FROM projects
            JOIN post_tags ON post_tags.post_id = projects.post_id
            JOIN tags ON tags.id = post_tags.tag_id
            WHERE projects.id IN ({})
            "#,
            placeholders
        );

        let mut query = sqlx::query_as::<_, ProjectTagRow>(&sql);
        let mut map = HashMap::<i64, usize>::new();
        let mut snapshots = vec![];

        for row in rows {
            map.insert(row.project_id, snapshots.len());
            query = query.bind(row.project_id);
            snapshots.push(row.into_snapshot(vec![], vec![]));
        }

        let tag_rows = query.fetch_all(&self.pool).await?;
        for row in tag_rows {
            if let Some(index) = map.get(&row.project_id)
                && let Some(project) = snapshots.get_mut(*index)
            {
                project.tag_names.push(row.tag_name);
                project.tag_slugs.push(row.tag_slug);
            }
        }

        Ok(snapshots)
    }

    async fn links_for_project(&self, project_id: i64) -> Result<Vec<ProjectLink>, ProjectError> {
        Ok(sqlx::query_as::<_, ProjectLinkRow>(
            r#"
            SELECT label, url
            FROM project_links
            WHERE project_id = ?
            ORDER BY sort_order ASC, id ASC
            "#,
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| ProjectLink {
            label: row.label,
            url: row.url,
        })
        .collect())
    }

    async fn project_from_row(
        &self,
        row: ProjectContentRow,
        viewing_user_id: Option<i64>,
    ) -> Result<Project, ProjectError> {
        let tag_rows = sqlx::query_as::<_, TagRow>(
            r#"
            SELECT post_id, tags.name AS tag_name, tags.slug AS tag_slug
            FROM post_tags
            JOIN tags ON tags.id = post_tags.tag_id
            WHERE post_id = ?
            "#,
        )
        .bind(row.post_id)
        .fetch_all(&self.pool)
        .await?;

        let medium_rows = sqlx::query_as::<_, MediumUsageWithNameRow>(
            r#"
            SELECT code, url, short_name
            FROM post_media_usages
            JOIN media ON media.id = medium_id
            WHERE post_media_usages.post_id = ?
            "#,
        )
        .bind(row.post_id)
        .fetch_all(&self.pool)
        .await?;

        let len = medium_rows.len() as i64;
        let mut medium_urls = vec![String::new(); medium_rows.len()];
        let mut medium_short_names = vec![String::new(); medium_rows.len()];
        for medium in medium_rows {
            if medium.code < 0 || medium.code >= len {
                return Err(ProjectError::InternalError(
                    "Out of range project media index found".to_string(),
                ));
            }
            let index = medium.code as usize;
            medium_urls[index] = medium.url;
            medium_short_names[index] = medium.short_name;
        }

        Ok(Project {
            id: row.project_id,
            post_id: row.post_id,
            title: row.title,
            slug: row.slug,
            author_name: row.author_name,
            author_slug: row.author_slug,
            author_avatar_url: row.author_avatar_url,
            tags: tag_rows.into_iter().map(|row| row.tag_slug).collect(),
            excerpt: row.excerpt,
            content: row.content,
            draft: row.draft,
            published_at: row.published_at,
            updated_at: row.updated_at,
            medium_urls,
            medium_short_names,
            cover_url: row.cover_url,
            demo: ProjectDemo {
                demo_type: row.demo_type,
                entry_path: row.demo_entry_path,
                width: row.demo_width,
                height: row.demo_height,
                config: row.demo_config,
            },
            links: self.links_for_project(row.project_id).await?,
            is_owner: viewing_user_id == Some(row.user_id),
        })
    }
}

#[async_trait::async_trait]
impl ProjectService for ProjectServiceImpl {
    async fn new_project(&self, cmd: NewProjectCommand) -> Result<i64, ProjectError> {
        let mut tx = self.pool.begin().await?;

        let project_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO projects (
                post_id,
                demo_type,
                demo_entry_path,
                demo_width,
                demo_height,
                demo_config
            )
            VALUES (?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(cmd.post_id)
        .bind(cmd.demo_type)
        .bind(cmd.demo_entry_path)
        .bind(cmd.demo_width)
        .bind(cmd.demo_height)
        .bind(cmd.demo_config)
        .fetch_one(&mut *tx)
        .await?;

        if !cmd.links.is_empty() {
            let values = cmd
                .links
                .iter()
                .map(|_| "(?, ?, ?, ?)".to_string())
                .collect::<Vec<_>>()
                .join(", ");
            let sql = format!(
                "INSERT INTO project_links (project_id, label, url, sort_order) VALUES {}",
                values
            );
            let mut query = sqlx::query(&sql);
            for (index, link) in cmd.links.iter().enumerate() {
                query = query
                    .bind(project_id)
                    .bind(&link.label)
                    .bind(&link.url)
                    .bind(index as i64);
            }
            query.execute(&mut *tx).await?;
        }

        tx.commit().await?;
        Ok(project_id)
    }

    async fn update_project(&self, cmd: UpdateProjectCommand) -> Result<(), ProjectError> {
        let post_user_id: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT posts.user_id
            FROM projects
            JOIN posts ON posts.id = projects.post_id
            WHERE projects.id = ?
            "#,
        )
        .bind(cmd.project_id)
        .fetch_optional(&self.pool)
        .await?;

        if post_user_id.is_none() {
            return Err(ProjectError::ProjectNotFound);
        }
        if post_user_id != Some(cmd.user_id) {
            return Err(ProjectError::Forbidden);
        }

        let mut tx = self.pool.begin().await?;
        let mut fields = vec![];
        if cmd.demo_type.is_some() {
            fields.push("demo_type = ?");
        }
        if cmd.demo_entry_path.is_some() {
            fields.push("demo_entry_path = ?");
        }
        if cmd.demo_width.is_some() {
            fields.push("demo_width = ?");
        }
        if cmd.demo_height.is_some() {
            fields.push("demo_height = ?");
        }
        if cmd.demo_config.is_some() {
            fields.push("demo_config = ?");
        }

        if !fields.is_empty() {
            fields.push("updated_at = CURRENT_TIMESTAMP");
            let sql = format!("UPDATE projects SET {} WHERE id = ?", fields.join(", "));
            let mut query = sqlx::query(&sql);
            if let Some(value) = cmd.demo_type {
                query = query.bind(value);
            }
            if let Some(value) = cmd.demo_entry_path {
                query = query.bind(value);
            }
            if let Some(value) = cmd.demo_width {
                query = query.bind(value);
            }
            if let Some(value) = cmd.demo_height {
                query = query.bind(value);
            }
            if let Some(value) = cmd.demo_config {
                query = query.bind(value);
            }
            query.bind(cmd.project_id).execute(&mut *tx).await?;
        }

        if let Some(links) = cmd.links {
            sqlx::query("DELETE FROM project_links WHERE project_id = ?")
                .bind(cmd.project_id)
                .execute(&mut *tx)
                .await?;
            if !links.is_empty() {
                let values = links
                    .iter()
                    .map(|_| "(?, ?, ?, ?)".to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                let sql = format!(
                    "INSERT INTO project_links (project_id, label, url, sort_order) VALUES {}",
                    values
                );
                let mut query = sqlx::query(&sql);
                for (index, link) in links.iter().enumerate() {
                    query = query
                        .bind(cmd.project_id)
                        .bind(&link.label)
                        .bind(&link.url)
                        .bind(index as i64);
                }
                query.execute(&mut *tx).await?;
            }
        }

        tx.commit().await?;
        Ok(())
    }

    async fn get_project_by_slug(
        &self,
        cmd: GetProjectBySlugCommand,
    ) -> Result<Project, ProjectError> {
        if let Some(id) = cmd.as_id {
            let allowed: Option<i64> = sqlx::query_scalar(
                r#"
                SELECT posts.id
                FROM projects
                JOIN posts ON posts.id = projects.post_id
                WHERE posts.user_id = ? AND posts.slug = ?
                "#,
            )
            .bind(id)
            .bind(&cmd.slug)
            .fetch_optional(&self.pool)
            .await?;

            if allowed.is_none() {
                return Err(ProjectError::Forbidden);
            }
        }

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
                posts.draft,
                posts.published_at,
                posts.updated_at,
                'media/i/' || cover.short_name AS cover_url,
                projects.demo_type,
                projects.demo_entry_path,
                projects.demo_width,
                projects.demo_height,
                projects.demo_config
            FROM projects
            JOIN posts ON posts.id = projects.post_id
            JOIN users ON users.id = posts.user_id
            JOIN user_meta ON user_meta.user_id = users.id
            LEFT JOIN media cover ON cover.id = posts.cover_image_id
            LEFT JOIN media avatar ON avatar.id = user_meta.avatar_image_id
            WHERE posts.slug = ? AND posts.status = 'published'
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
                posts.draft,
                posts.published_at,
                posts.updated_at,
                media.url AS cover_url,
                projects.demo_type,
                projects.demo_entry_path,
                projects.demo_width,
                projects.demo_height,
                projects.demo_config
            FROM projects
            JOIN posts ON posts.id = projects.post_id
            JOIN users ON users.id = posts.user_id
            JOIN user_meta ON user_meta.user_id = users.id
            LEFT JOIN media ON media.id = posts.cover_image_id
            LEFT JOIN media avatar ON avatar.id = user_meta.avatar_image_id
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

    async fn get_project_post_id(
        &self,
        cmd: GetProjectPostIdCommand,
    ) -> Result<i64, ProjectError> {
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
    ) -> Result<Vec<ProjectSnapshot>, ProjectError> {
        let mut where_parts = Vec::<String>::new();
        if cmd.public_only {
            where_parts.push("posts.status = 'published'".to_string());
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
                projects.demo_type,
                post_stats.views,
                post_stats.likes,
                post_stats.comments_count
            FROM projects
            JOIN posts ON posts.id = projects.post_id
            JOIN users ON users.id = posts.user_id
            JOIN user_meta ON user_meta.user_id = posts.user_id
            JOIN post_stats ON post_stats.post_id = posts.id
            LEFT JOIN media ON media.id = posts.cover_image_id
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
        let rows = query.bind(cmd.limit).bind(cmd.offset).fetch_all(&self.pool).await?;

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
                projects.demo_type,
                post_stats.views,
                post_stats.likes,
                post_stats.comments_count
            FROM projects
            JOIN posts ON posts.id = projects.post_id
            JOIN users ON users.id = posts.user_id
            JOIN user_meta ON user_meta.user_id = posts.user_id
            JOIN post_stats ON post_stats.post_id = posts.id
            LEFT JOIN media ON media.id = posts.cover_image_id
            WHERE posts.status = 'published'
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
