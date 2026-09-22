// Single-post reads: the full post body and the detail view with media.

use crate::application::commands::post::{GetDetailedPostsCommand, GetPostCommand};
use crate::domain::entities::post::{Post, PostDetails, PostSeries, PostSummary};
use crate::domain::errors::post::PostError;

use super::PostServiceImpl;
use super::rows::{MediumUsageRow, MediumUsageWithNameRow, PostContentRow, PostDetailsRow, TagRow};

impl PostServiceImpl {
    pub(super) async fn get_post(&self, cmd: GetPostCommand) -> Result<Post, PostError> {
        let PostContentRow {
            post_id,
            author_name,
            author_slug,
            author_avatar_url,
            title,
            excerpt,
            content,
            published_at,
            updated_at,
            url,
            cover_media_type,
            cover_video_url,
            cover_video_type,
            og_image_seconds,
            reading_time_minutes,
        } = sqlx::query_as::<_, PostContentRow>(
            r#"
            SELECT posts.id AS post_id, users.username AS author_slug, user_meta.display_name AS author_name, title, excerpt, content, published_at, posts.updated_at AS updated_at, 'media/i/' || m1.short_name AS url, m1.file_type AS cover_media_type, 'media/i/' || video.short_name AS cover_video_url, video.file_type AS cover_video_type, posts.og_image_seconds, posts.reading_time_minutes, 'media/i/' || m2.short_name AS author_avatar_url
            FROM posts
            JOIN users ON posts.user_id = users.id
            JOIN user_meta ON user_meta.user_id = users.id
            LEFT JOIN media m1 ON m1.id = posts.cover_media_id
            LEFT JOIN media m2 ON m2.id = user_meta.avatar_image_id
            LEFT JOIN media video ON video.short_name = '.post.' || posts.id
            WHERE posts.slug = ? AND status = 'published' AND posts.deleted_at IS NULL AND posts.content_kind = 'post'
            "#,
        )
        .bind(&cmd.slug)
        .fetch_one(&self.pool)
        .await?;

        let medium_usage_rows = sqlx::query_as::<_, MediumUsageRow>(
            r#"
            SELECT code, url
            FROM post_media_usages
            JOIN media on media.id = medium_id
            WHERE post_media_usages.post_id = ?
            "#,
        )
        .bind(post_id)
        .fetch_all(&self.pool)
        .await?;

        let len = medium_usage_rows.len();

        let mut medium_urls = vec![String::new(); len];

        let len = len as i64;

        for MediumUsageRow { code, url } in medium_usage_rows {
            if code < 0 || code > len {
                return Err(PostError::InternalError(
                    "Out of range index found".to_string(),
                ));
            }

            let index = code;

            if index >= len {
                return Err(PostError::InternalError(
                    "Oversized insertion found".to_string(),
                ));
            }

            let index = index as usize;

            medium_urls[index] = url;
        }

        let tags: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT slug
            FROM post_tags
            JOIN tags ON tags.id = post_tags.tag_id
            WHERE post_id = ?
            "#,
        )
        .bind(post_id)
        .fetch_all(&self.pool)
        .await?;

        let series_opt = sqlx::query_as::<_, (i64, String, String, String, i64)>(
            r#"
            SELECT s.id, s.title, s.slug, 'media/i/' || m.short_name, sp.number
            FROM series_post sp
            JOIN series s ON s.id = sp.series_id
            LEFT JOIN media m ON m.id = s.cover_image_id
            WHERE sp.post_id = ?
            LIMIT 1
            "#,
        )
        .bind(post_id)
        .fetch_optional(&self.pool)
        .await?;

        let mut post_series = None;

        if let Some((id, series_title, series_slug, series_cover_url, number)) = series_opt {
            let previous_post_opt = sqlx::query_as::<_, (String, String, Option<String>)>(
                r#"
                SELECT p.title, p.slug, 'media/i/' || m.short_name
                FROM series_post sp
                JOIN series s ON s.id = sp.series_id
                JOIN posts p ON p.id = sp.post_id
                LEFT JOIN media m ON m.id = p.cover_media_id
                WHERE s.id = ? AND sp.number < ? AND p.content_kind = 'post'
                ORDER BY sp.number DESC
                LIMIT 1
                "#,
            )
            .bind(id)
            .bind(number)
            .fetch_optional(&self.pool)
            .await?;

            let mut previous_post: Option<PostSummary> = None;
            if let Some((title, slug, cover_url)) = previous_post_opt {
                previous_post = Some(PostSummary {
                    title,
                    slug,
                    cover_url,
                });
            }

            let next_post_opt = sqlx::query_as::<_, (String, String, Option<String>)>(
                r#"
                SELECT p.title, p.slug, 'media/i/' || m.short_name
                FROM series_post sp
                JOIN series s ON s.id = sp.series_id
                JOIN posts p ON p.id = sp.post_id
                LEFT JOIN media m ON m.id = p.cover_media_id
                WHERE s.id = ? AND sp.number > ? AND p.content_kind = 'post'
                ORDER BY sp.number ASC
                LIMIT 1
                "#,
            )
            .bind(id)
            .bind(number)
            .fetch_optional(&self.pool)
            .await?;

            let mut next_post: Option<PostSummary> = None;
            if let Some((title, slug, cover_url)) = next_post_opt {
                next_post = Some(PostSummary {
                    title,
                    slug,
                    cover_url,
                });
            }
            post_series = Some(PostSeries {
                series_title,
                series_slug,
                series_cover_url,
                previous_post,
                next_post,
            });
        }

        Ok(Post {
            id: post_id,
            title,
            author_name,
            author_slug,
            author_avatar_url,
            tags,
            excerpt,
            content,
            published_at,
            updated_at,
            medium_urls,
            post_series,
            cover_url: url,
            cover_media_type,
            cover_video_url,
            cover_video_type,
            og_image_seconds,
            reading_time_minutes,
        })
    }
    pub(super) async fn get_post_details(
        &self,
        cmd: GetDetailedPostsCommand,
    ) -> Result<PostDetails, PostError> {
        let post_row = sqlx::query_as::<_, PostDetailsRow>(
            r#"
            SELECT
                posts.id AS post_id,
                title,
                posts.slug AS slug,
                excerpt,
                series_post.series_id AS series_id,
                content,
                is_featured,
                user_id,
                cover.url AS cover_url,
                cover.file_type AS cover_media_type,
                posts.og_image_seconds,
                posts.updated_at AS updated_at
            FROM posts
            LEFT JOIN series_post ON series_post.post_id = posts.id
            LEFT JOIN media cover ON cover.id = posts.cover_media_id
            WHERE posts.id = ?;
            "#,
        )
        .bind(cmd.post_id)
        .fetch_one(&self.pool)
        .await?;

        if let Some(user_id) = cmd.required_author_id
            && user_id != post_row.user_id
        {
            return Err(PostError::Forbidden);
        }

        let tag_rows = sqlx::query_as::<_, TagRow>(
            r#"
            SELECT post_id, name AS tag_name, slug AS tag_slug
            FROM post_tags
            JOIN tags ON tags.id = post_tags.tag_id
            WHERE post_id = ?
            "#,
        )
        .bind(post_row.post_id)
        .fetch_all(&self.pool)
        .await?;

        let mut series_slug: Option<String> = None;
        let mut series_cover_url: Option<String> = None;
        if let Some(series_id) = post_row.series_id {
            let series = sqlx::query_as::<_, (String, Option<String>)>(
                r#"
                SELECT series.slug, url
                FROM series
                LEFT JOIN media ON media.id = cover_image_id
                WHERE series.id = ?
                "#,
            )
            .bind(series_id)
            .fetch_one(&self.pool)
            .await?;

            series_slug = Some(series.0);
            series_cover_url = series.1;
        }

        let medium_usage_rows = sqlx::query_as::<_, MediumUsageWithNameRow>(
            r#"
            SELECT code, url, short_name
            FROM post_media_usages
            JOIN media ON media.id = medium_id
            WHERE post_media_usages.post_id = ?
            "#,
        )
        .bind(post_row.post_id)
        .fetch_all(&self.pool)
        .await?;

        let len = medium_usage_rows.len();

        let mut medium_urls = vec![String::new(); len];
        let mut medium_short_names = vec![String::new(); len];

        let len = len as i64;

        for MediumUsageWithNameRow {
            code,
            url,
            short_name,
        } in medium_usage_rows
        {
            if code < 0 || code > len {
                return Err(PostError::InternalError(
                    "Out of range index found".to_string(),
                ));
            }

            let index = code;

            if index >= len {
                return Err(PostError::InternalError(
                    "Oversized insertion found".to_string(),
                ));
            }

            let index = index as usize;

            medium_urls[index] = url;
            medium_short_names[index] = short_name;
        }

        let cover_url = post_row.cover_url;
        let cover_media_type = post_row.cover_media_type;
        let og_image_url = Some(format!("media/i/.post.{}.thumbnail", post_row.post_id));

        Ok(PostDetails {
            id: post_row.post_id,
            title: post_row.title,
            slug: post_row.slug,
            tags: tag_rows
                .into_iter()
                .map(|tag_row| tag_row.tag_slug)
                .collect(),
            excerpt: post_row.excerpt,
            series_slug,
            series_cover_url,
            content: post_row.content,
            is_featured: post_row.is_featured,
            medium_urls,
            medium_short_names,
            cover_url,
            cover_media_type,
            og_image_seconds: post_row.og_image_seconds,
            is_owner: post_row.user_id == cmd.viewing_user_id,
            og_image_url,
            updated_at: crate::helper::time::normalize_optional_utc_timestamp(post_row.updated_at),
        })
    }
}
