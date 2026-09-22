// Game read methods: by-slug, details, post id, listings, tag feed.

use crate::application::commands::game::{
    GetFeaturedGamesCommand, GetGameBySlugCommand, GetGameDetailsCommand, GetGamePostIdCommand,
    GetGamesByTagCommand, GetLatestGamesCommand,
};
use crate::domain::entities::game::{Game, GameSnapshot, GameSnapshotPage};
use crate::domain::errors::game::GameError;

use super::GameServiceImpl;
use super::rows::{GameContentRow, GameSnapshotRow};

impl GameServiceImpl {
    pub(super) async fn get_game_by_slug(
        &self,
        cmd: GetGameBySlugCommand,
    ) -> Result<Game, GameError> {
        let row = sqlx::query_as::<_, GameContentRow>(
            r#"
            SELECT
                games.id AS game_id,
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
                games.launcher_type,
                games.demo_width,
                games.demo_height,
                games.demo_url,
                games.instruction,
                games.cheatcode,
                games.story
            FROM games
            JOIN posts ON posts.id = games.post_id
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
        .ok_or(GameError::GameNotFound)?;

        self.game_from_row(row, cmd.as_id).await
    }

    pub(super) async fn get_game_details(
        &self,
        cmd: GetGameDetailsCommand,
    ) -> Result<Game, GameError> {
        let row = sqlx::query_as::<_, GameContentRow>(
            r#"
            SELECT
                games.id AS game_id,
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
                games.launcher_type,
                games.demo_width,
                games.demo_height,
                games.demo_url,
                games.instruction,
                games.cheatcode,
                games.story
            FROM games
            JOIN posts ON posts.id = games.post_id
            JOIN users ON users.id = posts.user_id
            JOIN user_meta ON user_meta.user_id = users.id
            LEFT JOIN media ON media.id = posts.cover_media_id
            LEFT JOIN media avatar ON avatar.id = user_meta.avatar_image_id
            LEFT JOIN media video ON video.short_name = '.post.' || posts.id
            WHERE games.id = ?
            "#,
        )
        .bind(cmd.game_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(GameError::GameNotFound)?;

        if let Some(user_id) = cmd.required_author_id
            && user_id != row.user_id
        {
            return Err(GameError::Forbidden);
        }

        self.game_from_row(row, Some(cmd.viewing_user_id)).await
    }

    pub(super) async fn get_game_post_id(
        &self,
        cmd: GetGamePostIdCommand,
    ) -> Result<i64, GameError> {
        let row: Option<(i64, i64)> = sqlx::query_as(
            r#"
            SELECT posts.id, posts.user_id
            FROM games
            JOIN posts ON posts.id = games.post_id
            WHERE games.id = ?
            "#,
        )
        .bind(cmd.game_id)
        .fetch_optional(&self.pool)
        .await?;

        let (post_id, user_id) = row.ok_or(GameError::GameNotFound)?;
        if let Some(required) = cmd.required_author_id
            && required != user_id
        {
            return Err(GameError::Forbidden);
        }
        Ok(post_id)
    }

    pub(super) async fn get_latest_game_snapshots(
        &self,
        cmd: GetLatestGamesCommand,
    ) -> Result<GameSnapshotPage, GameError> {
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
                games.id AS game_id,
                posts.id AS post_id,
                posts.title,
                posts.slug,
                posts.excerpt,
                users.username AS author_slug,
                user_meta.display_name AS author_name,
                posts.status,
                'media/i/' || media.short_name AS url,
                media.file_type AS cover_media_type,
                games.launcher_type,
                post_stats.views,
                post_stats.likes,
                post_stats.comments_count,
                posts.reading_time_minutes
            FROM games
            JOIN posts ON posts.id = games.post_id
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

        let mut query = sqlx::query_as::<_, GameSnapshotRow>(&sql);
        if let Some(user_id) = cmd.required_author_id {
            query = query.bind(user_id);
        }
        let rows = query
            .bind(cmd.limit + 1)
            .bind(cmd.offset)
            .fetch_all(&self.pool)
            .await?;

        let mut games = self.hydrate_game_rows(rows).await?;
        let has_more = games.len() as i64 > cmd.limit;
        if has_more {
            games.truncate(cmd.limit as usize);
        }

        Ok(GameSnapshotPage { games, has_more })
    }

    pub(super) async fn get_featured_game_snapshots(
        &self,
        cmd: GetFeaturedGamesCommand,
    ) -> Result<Vec<GameSnapshot>, GameError> {
        let rows = sqlx::query_as::<_, GameSnapshotRow>(
            r#"
            SELECT
                games.id AS game_id,
                posts.id AS post_id,
                posts.title,
                posts.slug,
                posts.excerpt,
                users.username AS author_slug,
                user_meta.display_name AS author_name,
                posts.status,
                'media/i/' || media.short_name AS url,
                media.file_type AS cover_media_type,
                games.launcher_type,
                post_stats.views,
                post_stats.likes,
                post_stats.comments_count,
                posts.reading_time_minutes
            FROM games
            JOIN posts ON posts.id = games.post_id
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

        self.hydrate_game_rows(rows).await
    }

    pub(super) async fn get_game_snapshots_by_tag(
        &self,
        cmd: GetGamesByTagCommand,
    ) -> Result<Vec<GameSnapshot>, GameError> {
        let rows = sqlx::query_as::<_, GameSnapshotRow>(
            r#"
            SELECT
                games.id AS game_id,
                posts.id AS post_id,
                posts.title,
                posts.slug,
                posts.excerpt,
                users.username AS author_slug,
                user_meta.display_name AS author_name,
                posts.status,
                'media/i/' || media.short_name AS url,
                media.file_type AS cover_media_type,
                games.launcher_type,
                post_stats.views,
                post_stats.likes,
                post_stats.comments_count,
                posts.reading_time_minutes
            FROM games
            JOIN posts ON posts.id = games.post_id
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

        self.hydrate_game_rows(rows).await
    }
}
