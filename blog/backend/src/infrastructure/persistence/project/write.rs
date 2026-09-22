// Project write methods: create, update, featured flag. The link/tag
// table maintenance stays inline here (it is interleaved with the project
// upserts rather than being a free helper like the post aggregate's).

use crate::application::commands::project::{
    NewProjectCommand, SetFeaturedProjectCommand, UpdateProjectCommand,
};
use crate::domain::errors::project::ProjectError;

use super::ProjectServiceImpl;

impl ProjectServiceImpl {
    pub(super) async fn new_project(&self, cmd: NewProjectCommand) -> Result<i64, ProjectError> {
        let mut tx = self.pool.begin().await?;
        let initial_demo_url = cmd.demo_url.clone();
        let demo_type = cmd.demo_type.clone();

        let project_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO projects (
                post_id,
                demo_type,
                demo_entry_path,
                demo_width,
                demo_height,
                demo_config,
                demo_url,
                delegate_game_id,
                inherit_thumbnail,
                inherit_tags
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(cmd.post_id)
        .bind(cmd.demo_type)
        .bind(cmd.demo_entry_path)
        .bind(cmd.demo_width)
        .bind(cmd.demo_height)
        .bind(cmd.demo_config)
        .bind(&initial_demo_url)
        .bind(cmd.delegate_game_id)
        .bind(cmd.inherit_thumbnail as i64)
        .bind(cmd.inherit_tags as i64)
        .fetch_one(&mut *tx)
        .await?;

        if (demo_type == "html5" || demo_type == "webgl")
            && (initial_demo_url.is_none()
                || initial_demo_url.as_deref().unwrap_or("").trim().is_empty())
        {
            let local_demo_url = std::path::PathBuf::from(cmd.demo_url_dir)
                .join(project_id.to_string())
                .join("index.html");
            let local_demo_url_str = local_demo_url.to_str().unwrap_or("").to_string();

            sqlx::query("UPDATE projects SET demo_url = ? WHERE id = ?")
                .bind(local_demo_url_str)
                .bind(project_id)
                .execute(&mut *tx)
                .await?;
        }

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

    pub(super) async fn update_project(
        &self,
        cmd: UpdateProjectCommand,
    ) -> Result<(), ProjectError> {
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
        if cmd.demo_url.is_some() {
            fields.push("demo_url = ?");
        }
        if cmd.delegate_game_id.is_some() {
            fields.push("delegate_game_id = ?");
        }
        if cmd.inherit_thumbnail.is_some() {
            fields.push("inherit_thumbnail = ?");
        }
        if cmd.inherit_tags.is_some() {
            fields.push("inherit_tags = ?");
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
            if let Some(value) = cmd.demo_url {
                query = query.bind(value);
            }
            if let Some(value) = cmd.delegate_game_id {
                query = query.bind(value);
            }
            if let Some(value) = cmd.inherit_thumbnail {
                query = query.bind(value as i64);
            }
            if let Some(value) = cmd.inherit_tags {
                query = query.bind(value as i64);
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

    pub(super) async fn set_project_featured(
        &self,
        cmd: SetFeaturedProjectCommand,
    ) -> Result<(), ProjectError> {
        let is_featured_val = if cmd.is_featured { 1 } else { 0 };
        sqlx::query(
            r#"
            UPDATE posts
            SET is_featured = ?
            WHERE id = (SELECT post_id FROM projects WHERE id = ?)
            "#,
        )
        .bind(is_featured_val)
        .bind(cmd.project_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
