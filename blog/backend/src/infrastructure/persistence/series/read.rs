// Series read methods: user series and the public catalogue.
use std::collections::HashMap;

use sqlx::Row;

use crate::application::{
    commands::series::{GetAllSeriesCommand, GetSeriesCommand},
    services::series::SeriesService,
};
use crate::domain::entities::series::SeriesSnapshot;
use crate::domain::errors::series::SeriesError;

use super::SeriesServiceImpl;

#[async_trait::async_trait]
impl SeriesService for SeriesServiceImpl {
    async fn get_series(&self, cmd: GetSeriesCommand) -> Result<Vec<SeriesSnapshot>, SeriesError> {
        // Row type: (series_id, title, slug, description, cover_url, post_count, owner_username)
        type Row = (
            i64,
            String,
            String,
            String,
            Option<String>,
            i64,
            Option<String>,
        );

        let rows: Vec<Row> = if cmd.is_admin {
            sqlx::query_as::<_, Row>(
                r#"
                SELECT s.id, s.title, s.slug, s.description, 'media/i/' || m.short_name,
                       COUNT(sp.post_id) AS post_count,
                       u.username AS owner_username
                FROM series s
                LEFT JOIN media m ON m.id = s.cover_image_id
                LEFT JOIN series_post sp ON sp.series_id = s.id
                LEFT JOIN users u ON u.id = s.user_id
                GROUP BY s.id
                ORDER BY s.created_at DESC
                "#,
            )
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, Row>(
                r#"
                SELECT s.id, s.title, s.slug, s.description, 'media/i/' || m.short_name,
                       COUNT(sp.post_id) AS post_count,
                       NULL AS owner_username
                FROM series s
                LEFT JOIN media m ON m.id = s.cover_image_id
                LEFT JOIN series_post sp ON sp.series_id = s.id
                WHERE s.user_id = ?
                GROUP BY s.id
                ORDER BY s.created_at DESC
                "#,
            )
            .bind(cmd.user_id)
            .fetch_all(&self.pool)
            .await?
        };

        let result = rows
            .into_iter()
            .map(
                |(id, title, slug, description, url, post_count, owner_username)| SeriesSnapshot {
                    id,
                    title,
                    slug,
                    description,
                    url,
                    post_count,
                    owner_username,
                },
            )
            .collect();

        Ok(result)
    }
    async fn get_all_series(
        &self,
        cmd: GetAllSeriesCommand,
    ) -> Result<Vec<SeriesWithPosts>, SeriesError> {
        let mut tx = self.pool.begin().await?;
        let all_series_posts = sqlx::query_as::<_, (i64, i64, i64)>(
            r#"
            SELECT sp.series_id, sp.post_id, sp.number
            FROM series_post sp
            JOIN posts ON posts.id = sp.post_id
            WHERE posts.status = 'published' AND posts.deleted_at IS NULL AND posts.content_kind = 'post'
              AND sp.series_id IN (
                  SELECT DISTINCT sp2.series_id
                  FROM series_post sp2
                  JOIN posts p2 ON p2.id = sp2.post_id
                  WHERE p2.status = 'published' AND deleted_at IS NULL AND p2.content_kind = 'post'
                  ORDER BY sp2.series_id DESC
                  LIMIT ?
                  OFFSET ?
              )
            ORDER BY sp.series_id DESC, sp.number DESC;
            "#,
        )
        .bind(cmd.limit)
        .bind(cmd.offset)
        .fetch_all(&mut *tx)
        .await?;

        let mut series_set = HashSet::<i64>::new();
        let mut post_set = HashSet::<i64>::new();
        let mut series_post_hash_map = HashMap::<i64, Vec<(i64, i64)>>::new();

        for &(series_id, post_id, number) in &all_series_posts {
            series_set.insert(series_id);
            post_set.insert(post_id);
            series_post_hash_map
                .entry(series_id)
                .or_default()
                .push((post_id, number));
        }

        // Create series hash map
        let all_series_ids = series_set
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!(
            r#"
            SELECT s.id, s.title, s.slug, u.username, um.display_name, s.description, 'media/i/' || m.short_name
            FROM series s
            LEFT JOIN users u ON u.id = s.user_id
            LEFT JOIN user_meta um ON um.user_id = u.id
            LEFT JOIN media m ON s.cover_image_id = m.id
            WHERE s.id IN ({})
            "#,
            all_series_ids
        );
        let all_series = sqlx::query_as::<
            _,
            (i64, String, String, String, String, String, Option<String>),
        >(&sql)
        .fetch_all(&mut *tx)
        .await?;

        let mut series_hash_map =
            HashMap::<i64, (String, String, String, String, String, Option<String>)>::new();
        for (id, title, slug, username, display_name, description, url) in all_series {
            series_hash_map.insert(id, (title, slug, username, display_name, description, url));
        }

        // Create post hash map
        let all_post_ids = post_set
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!(
            r#"
            SELECT p.id, p.title, p.slug, p.excerpt, um.display_name, u.username, p.status, 'media/i/' || m.short_name, m.file_type AS cover_media_type, ps.views, ps.likes, ps.comments_count, p.reading_time_minutes
            FROM posts p
            JOIN post_stats ps ON ps.post_id = p.id
            LEFT JOIN users u ON u.id = p.user_id
            LEFT JOIN user_meta um ON um.user_id = u.id
            LEFT JOIN media m ON m.id = p.cover_media_id
            WHERE p.id IN ({})
            "#,
            all_post_ids
        );
        let all_posts = sqlx::query_as::<
            _,
            (
                i64,
                String,
                String,
                String,
                String,
                String,
                String,
                Option<String>,
                Option<String>,
                i64,
                i64,
                i64,
                i64,
            ),
        >(&sql)
        .fetch_all(&mut *tx)
        .await?;

        let mut post_hash_map = HashMap::<
            i64,
            (
                String,
                String,
                String,
                String,
                String,
                String,
                Option<String>,
                Option<String>,
                i64,
                i64,
                i64,
                i64,
            ),
        >::new();
        for (
            id,
            title,
            slug,
            excerpt,
            display_name,
            username,
            status,
            url,
            cover_media_type,
            views,
            likes,
            comments_count,
            reading_time_minutes,
        ) in all_posts
        {
            post_hash_map.insert(
                id,
                (
                    title,
                    slug,
                    excerpt,
                    display_name,
                    username,
                    status,
                    url,
                    cover_media_type,
                    views,
                    likes,
                    comments_count,
                    reading_time_minutes,
                ),
            );
        }

        // Create tag hash_map
        let sql = format!(
            r#"
            SELECT pt.post_id, t.name, t.slug
            FROM post_tags pt
            LEFT JOIN tags t ON pt.tag_id = t.id
            WHERE pt.post_id IN ({})
            "#,
            all_post_ids
        );
        let all_post_tags = sqlx::query_as::<_, (i64, String, String)>(&sql)
            .fetch_all(&mut *tx)
            .await?;

        let mut tag_name_hash_map = HashMap::<i64, Vec<String>>::new();
        let mut tag_slug_hash_map = HashMap::<i64, Vec<String>>::new();
        for (id, name, slug) in all_post_tags {
            tag_name_hash_map.entry(id).or_default().push(name);
            tag_slug_hash_map.entry(id).or_default().push(slug);
        }

        // You have series_hash_map, series_post_hash_map, post_hash_map, and tag_hash_map
        let mut result = Vec::<SeriesWithPosts>::new();

        let mut series_ids: Vec<_> = series_post_hash_map.keys().cloned().collect();
        series_ids.sort_by(|a, b| b.cmp(a));

        for series_id in series_ids {
            if let Some(posts_with_numbers) = series_post_hash_map.get(&series_id) {
                let mut numbers = Vec::<i64>::new();
                let mut posts = Vec::<PostSnapshot>::new();

                // String, String, String, String, String, Option<String>
                if let Some((title, slug, username, display_name, description, url)) =
                    series_hash_map.get(&series_id)
                {
                    for &(post_id, number) in posts_with_numbers {
                        numbers.push(number);

                        if let Some((
                            title,
                            slug,
                            excerpt,
                            author_name,
                            author_slug,
                            status,
                            url,
                            cover_media_type,
                            views,
                            likes,
                            comments_count,
                            reading_time_minutes,
                        )) = post_hash_map.get(&post_id)
                            && let Some((tag_names, tag_slugs)) = tag_name_hash_map
                                .get(&post_id)
                                .zip(tag_slug_hash_map.get(&post_id))
                        {
                            posts.push(PostSnapshot {
                                id: post_id,
                                title: title.clone(),
                                slug: slug.clone(),
                                tag_names: tag_names.clone(),
                                tag_slugs: tag_slugs.clone(),
                                excerpt: excerpt.clone(),
                                author_name: author_name.clone(),
                                author_slug: author_slug.clone(),
                                status: status.clone(),
                                url: url.clone(),
                                cover_media_type: cover_media_type.clone(),
                                stats: PostStats {
                                    views: *views,
                                    likes: *likes,
                                    comments: *comments_count,
                                },
                                reading_time_minutes: *reading_time_minutes,
                            });
                        }
                    }
                    result.push(SeriesWithPosts {
                        id: series_id,
                        title: title.clone(),
                        slug: slug.clone(),
                        username: username.clone(),
                        display_name: display_name.clone(),
                        description: description.clone(),
                        url: url.clone(),
                        posts,
                        numbers,
                    })
                }
            }
        }

        Ok(result)
    }
}
