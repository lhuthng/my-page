// Row -> entity conversion: snapshot assembly and tag/media hydration.
use std::collections::{HashMap, HashSet};

use sqlx::Row;

use crate::domain::entities::post::{PostSnapshot, PostStats, TagSummary};

use super::rows::{MediumUsageRow, MediumUsageWithNameRow, PostRow, PostDetailsRow, TagRow};
use super::PostServiceImpl;

impl PostRow {
    pub fn into_snapshot(self, tag_names: Vec<String>, tag_slugs: Vec<String>) -> PostSnapshot {
        PostSnapshot {
            id: self.post_id,
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
            stats: PostStats {
                likes: self.likes,
                views: self.views,
                comments: self.comments_count,
            },
            reading_time_minutes: self.reading_time_minutes,
        }
    }
}

impl PostServiceImpl {
    pub(super) async fn hydrate_post_rows(
        &self,
        post_rows: Vec<PostRow>,
    ) -> Result<Vec<PostSnapshot>, PostError> {
        if post_rows.is_empty() {
            return Ok(vec![]);
        }

        let placeholder = post_rows
            .iter()
            .map(|_| "?".to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let sequel = format!(
            r#"
            SELECT post_id, tag_id, tags.name AS tag_name, tags.slug AS tag_slug
            FROM post_tags
            JOIN tags ON tags.id = post_tags.tag_id
            WHERE post_tags.post_id IN ({})
            "#,
            placeholder
        );

        let mut query = sqlx::query_as::<_, TagRow>(&sequel);

        let mut posts_map: HashMap<i64, usize> = HashMap::new();
        let mut snapshots = vec![];

        for post_row in post_rows {
            posts_map.insert(post_row.post_id, snapshots.len());
            query = query.bind(post_row.post_id);
            snapshots.push(post_row.into_snapshot(vec![], vec![]));
        }

        let tag_rows = query.fetch_all(&self.pool).await?;

        for tag_row in tag_rows {
            if let Some(index) = posts_map.get(&tag_row.post_id)
                && let Some(post) = snapshots.get_mut(*index)
            {
                post.tag_names.push(tag_row.tag_name);
                post.tag_slugs.push(tag_row.tag_slug);
            }
        }

        Ok(snapshots)
    }

    async fn get_posts(
        &self,
        is_public: bool,
        featured: Option<i64>,
        limit: i64,
        offset: i64,
        order_by: String,
    ) -> Result<Vec<PostSnapshot>, PostError> {
        let mut placeholder: Vec<String> = vec![];

        placeholder.push("content_kind = 'post'".to_string());

        if is_public {
            placeholder.push("status = 'published' AND deleted_at IS NULL".to_string());
        }

        if let Some(feature) = featured {
            placeholder.push(format!("is_featured = {}", feature));
        }

        let mut placeholder = placeholder.join(" AND ");
        if !placeholder.is_empty() {
            placeholder.insert_str(0, "WHERE ");
        }

        let order_by = match order_by.as_str() {
            "created" => "created_at",
            "updated" => "updated_at",
            _ => "created_at",
        };

        let sequel = format!(
            r#"
            SELECT p.id AS post_id, title, slug, excerpt, username AS author_slug, display_name AS author_name, 'media/i/' || m.short_name AS url, m.file_type AS cover_media_type, status, views, likes, comments_count, reading_time_minutes
            FROM posts p
                JOIN users u ON u.id = p.user_id
                JOIN user_meta um ON um.user_id = p.user_id
                JOIN post_stats ps ON ps.post_id = p.id
                LEFT JOIN media m ON m.id = p.cover_media_id
            {}
            ORDER BY p.{} DESC
            LIMIT ?
            OFFSET ?
            "#,
            placeholder, order_by
        );

        let post_rows = sqlx::query_as::<_, PostRow>(&sequel)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        self.hydrate_post_rows(post_rows).await
    }
}

#[async_trait::async_trait]
