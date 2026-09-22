// Shared snapshot-with-tags fetchers used by the listing views.
use std::collections::HashMap;

use sqlx::SqlitePool;

use crate::domain::entities::post::{PostSnapshot, PostStats};
use crate::domain::entities::project::ProjectSnapshot;
use crate::domain::errors::user::UserError;

use crate::infrastructure::persistence::post::PostRow;

use super::rows::{DashProjectRow, DashTagRow};

impl DashProjectRow {
    pub(super) fn into_snapshot(
        self,
        tag_names: Vec<String>,
        tag_slugs: Vec<String>,
    ) -> ProjectSnapshot {
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
            cover_media_type: self.cover_media_type,
            demo_type: self.demo_type,
            stats: PostStats {
                views: self.views,
                likes: self.likes,
                comments: self.comments_count,
            },
            reading_time_minutes: self.reading_time_minutes,
        }
    }
}

pub(super) async fn fetch_project_snapshots_with_tags(
    pool: &SqlitePool,
    rows: Vec<DashProjectRow>,
) -> Result<Vec<ProjectSnapshot>, UserError> {
    if rows.is_empty() {
        return Ok(vec![]);
    }

    let placeholders = rows.iter().map(|_| "?").collect::<Vec<_>>().join(", ");

    let sql = format!(
        r#"
        SELECT pj.id AS post_id, tags.name AS tag_name, tags.slug AS tag_slug
        FROM projects pj
        JOIN post_tags ON post_tags.post_id = pj.post_id
        JOIN tags ON tags.id = post_tags.tag_id
        WHERE pj.id IN ({})
        "#,
        placeholders
    );

    let mut query = sqlx::query_as::<_, DashTagRow>(&sql);
    let mut map: HashMap<i64, usize> = HashMap::new();
    let mut snapshots = vec![];

    for row in rows {
        map.insert(row.project_id, snapshots.len());
        query = query.bind(row.project_id);
        snapshots.push(row.into_snapshot(vec![], vec![]));
    }

    let tag_rows = query.fetch_all(pool).await?;

    for tag in tag_rows {
        if let Some(&idx) = map.get(&tag.post_id)
            && let Some(project) = snapshots.get_mut(idx)
        {
            project.tag_names.push(tag.tag_name);
            project.tag_slugs.push(tag.tag_slug);
        }
    }

    Ok(snapshots)
}

pub(super) async fn fetch_snapshots_with_tags(
    pool: &SqlitePool,
    post_rows: Vec<PostRow>,
) -> Result<Vec<PostSnapshot>, UserError> {
    if post_rows.is_empty() {
        return Ok(vec![]);
    }

    let placeholders = post_rows.iter().map(|_| "?").collect::<Vec<_>>().join(", ");

    let sql = format!(
        r#"
        SELECT post_tags.post_id, tags.name AS tag_name, tags.slug AS tag_slug
        FROM post_tags
        JOIN tags ON tags.id = post_tags.tag_id
        WHERE post_tags.post_id IN ({})
        "#,
        placeholders
    );

    let mut query = sqlx::query_as::<_, DashTagRow>(&sql);
    let mut posts_map: HashMap<i64, usize> = HashMap::new();
    let mut snapshots: Vec<PostSnapshot> = vec![];

    for row in post_rows {
        posts_map.insert(row.post_id, snapshots.len());
        query = query.bind(row.post_id);
        snapshots.push(row.into_snapshot(vec![], vec![]));
    }

    let tag_rows = query.fetch_all(pool).await?;

    for tag in tag_rows {
        if let Some(&idx) = posts_map.get(&tag.post_id)
            && let Some(post) = snapshots.get_mut(idx)
        {
            post.tag_names.push(tag.tag_name);
            post.tag_slugs.push(tag.tag_slug);
        }
    }

    Ok(snapshots)
}
