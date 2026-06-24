use std::collections::HashMap;

use sqlx::SqlitePool;

use super::rows::{DashboardPostRow, DashboardProjectRow, TagJoinRow};
use super::types::{GqlDashboardPost, GqlDashboardProject};

pub const DASHBOARD_POST_COLUMNS: &str = r#"
    p.id AS post_id, p.title, p.slug, p.excerpt,
    u.username AS author_slug, um.display_name AS author_name,
    'media/i/' || m.short_name AS url, m.file_type AS cover_media_type, p.status,
    ps.views, ps.likes, ps.comments_count
"#;

pub const DASHBOARD_POST_JOINS: &str = r#"
    FROM posts p
    JOIN users u ON u.id = p.user_id
    JOIN user_meta um ON um.user_id = p.user_id
    JOIN post_stats ps ON ps.post_id = p.id
    LEFT JOIN media m ON m.id = p.cover_media_id
"#;

pub async fn attach_tags_to_posts(
    pool: &SqlitePool,
    rows: Vec<DashboardPostRow>,
) -> Result<Vec<GqlDashboardPost>, sqlx::Error> {
    if rows.is_empty() {
        return Ok(vec![]);
    }
    let placeholders: Vec<String> = rows.iter().map(|_| "?".to_string()).collect();
    let sql = format!(
        r#"
        SELECT post_tags.post_id, tags.name AS tag_name, tags.slug AS tag_slug
        FROM post_tags
        JOIN tags ON tags.id = post_tags.tag_id
        WHERE post_tags.post_id IN ({})
        "#,
        placeholders.join(", ")
    );
    let mut query = sqlx::query_as::<_, TagJoinRow>(&sql);
    for row in &rows {
        query = query.bind(row.post_id);
    }
    let tag_rows = query.fetch_all(pool).await?;

    let mut tag_map: HashMap<i64, (Vec<String>, Vec<String>)> = HashMap::new();
    for tag in tag_rows {
        let entry = tag_map.entry(tag.post_id).or_insert((vec![], vec![]));
        entry.0.push(tag.tag_name);
        entry.1.push(tag.tag_slug);
    }

    let posts = rows
        .into_iter()
        .map(|r| {
            let (tag_names, tag_slugs) = tag_map.remove(&r.post_id).unwrap_or((vec![], vec![]));
            GqlDashboardPost {
                id: r.post_id,
                title: r.title,
                slug: r.slug,
                excerpt: r.excerpt,
                author_name: r.author_name,
                author_slug: r.author_slug,
                tag_names,
                tag_slugs,
                status: r.status,
                cover_url: r.url,
                cover_media_type: r.cover_media_type,
                views: r.views,
                likes: r.likes,
                comments_count: r.comments_count,
            }
        })
        .collect();

    Ok(posts)
}

pub async fn attach_tags_to_projects(
    pool: &SqlitePool,
    rows: Vec<DashboardProjectRow>,
) -> Result<Vec<GqlDashboardProject>, sqlx::Error> {
    if rows.is_empty() {
        return Ok(vec![]);
    }
    let placeholders: Vec<String> = rows.iter().map(|_| "?".to_string()).collect();
    let sql = format!(
        r#"
        SELECT post_tags.post_id, tags.name AS tag_name, tags.slug AS tag_slug
        FROM post_tags
        JOIN tags ON tags.id = post_tags.tag_id
        WHERE post_tags.post_id IN ({})
        "#,
        placeholders.join(", ")
    );
    let mut query = sqlx::query_as::<_, TagJoinRow>(&sql);
    for row in &rows {
        query = query.bind(row.post_id);
    }
    let tag_rows = query.fetch_all(pool).await?;

    let mut tag_map: HashMap<i64, (Vec<String>, Vec<String>)> = HashMap::new();
    for tag in tag_rows {
        let entry = tag_map.entry(tag.post_id).or_insert((vec![], vec![]));
        entry.0.push(tag.tag_name);
        entry.1.push(tag.tag_slug);
    }

    let projects = rows
        .into_iter()
        .map(|r| {
            let (tag_names, tag_slugs) = tag_map.remove(&r.post_id).unwrap_or((vec![], vec![]));
            GqlDashboardProject {
                id: r.project_id,
                post_id: r.post_id,
                title: r.title,
                slug: r.slug,
                excerpt: r.excerpt,
                author_name: r.author_name,
                author_slug: r.author_slug,
                tag_names,
                tag_slugs,
                status: r.status,
                cover_url: r.url,
                cover_media_type: r.cover_media_type,
                demo_type: r.demo_type,
                views: r.views,
                likes: r.likes,
                comments_count: r.comments_count,
            }
        })
        .collect();

    Ok(projects)
}
