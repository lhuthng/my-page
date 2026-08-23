use std::collections::HashMap;

use sqlx::{FromRow, SqlitePool};

use crate::{
    application::{
        commands::game::{
            GetFeaturedGamesCommand, GetGameBySlugCommand, GetGameDetailsCommand,
            GetGamePostIdCommand, GetGamesByTagCommand, GetLatestGamesCommand, NewGameCommand,
            SetFeaturedGameCommand, UpdateGameCommand,
        },
        services::game::GameService,
    },
    domain::{
        entities::{
            game::{Game, GameDemo, GameLink, GameSnapshot, GameSnapshotPage, JsDosBundle},
            post::PostStats,
        },
        errors::game::GameError,
    },
    infrastructure::persistence::post::{MediumUsageWithNameRow, TagRow},
};

pub struct GameServiceImpl {
    pub pool: SqlitePool,
}

impl GameServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, FromRow)]
struct GameSnapshotRow {
    game_id: i64,
    post_id: i64,
    title: String,
    slug: String,
    excerpt: String,
    author_name: String,
    author_slug: String,
    status: String,
    url: Option<String>,
    cover_media_type: Option<String>,
    launcher_type: String,
    views: i64,
    likes: i64,
    comments_count: i64,
    reading_time_minutes: i64,
}

#[derive(Debug, FromRow)]
struct GameContentRow {
    game_id: i64,
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
    cover_media_type: Option<String>,
    cover_video_url: Option<String>,
    cover_video_type: Option<String>,
    og_image_seconds: i64,
    launcher_type: String,
    demo_width: Option<String>,
    demo_height: Option<String>,
    demo_url: Option<String>,
    instruction: String,
    cheatcode: String,
    story: String,
}

#[derive(Debug, FromRow)]
struct GameTagRow {
    game_id: i64,
    tag_name: String,
    tag_slug: String,
}

impl GameSnapshotRow {
    fn into_snapshot(self, tag_names: Vec<String>, tag_slugs: Vec<String>) -> GameSnapshot {
        GameSnapshot {
            id: self.game_id,
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
            launcher_type: self.launcher_type,
            stats: PostStats {
                views: self.views,
                likes: self.likes,
                comments: self.comments_count,
            },
            reading_time_minutes: self.reading_time_minutes,
        }
    }
}

impl GameServiceImpl {
    async fn hydrate_game_rows(
        &self,
        rows: Vec<GameSnapshotRow>,
    ) -> Result<Vec<GameSnapshot>, GameError> {
        if rows.is_empty() {
            return Ok(vec![]);
        }

        let placeholders = rows.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!(
            r#"
            SELECT games.id AS game_id, tags.name AS tag_name, tags.slug AS tag_slug
            FROM games
            JOIN post_tags ON post_tags.post_id = games.post_id
            JOIN tags ON tags.id = post_tags.tag_id
            WHERE games.id IN ({})
            "#,
            placeholders
        );

        let mut query = sqlx::query_as::<_, GameTagRow>(&sql);
        let mut map = HashMap::<i64, usize>::new();
        let mut snapshots = vec![];

        for row in rows {
            map.insert(row.game_id, snapshots.len());
            query = query.bind(row.game_id);
            snapshots.push(row.into_snapshot(vec![], vec![]));
        }

        let tag_rows = query.fetch_all(&self.pool).await?;
        for row in tag_rows {
            if let Some(index) = map.get(&row.game_id)
                && let Some(game) = snapshots.get_mut(*index)
            {
                game.tag_names.push(row.tag_name);
                game.tag_slugs.push(row.tag_slug);
            }
        }

        Ok(snapshots)
    }

    async fn related_games_for(&self, game_id: i64) -> Result<Vec<GameLink>, GameError> {
        Ok(sqlx::query_as::<_, (i64, String, String)>(
            r#"
            SELECT g.id, posts.title, posts.slug
            FROM related_games rg
            JOIN games g ON g.id = rg.related_game_id
            JOIN posts ON posts.id = g.post_id
            WHERE rg.game_id = ?
            ORDER BY rg.sort_order ASC, g.id ASC
            "#,
        )
        .bind(game_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|(id, title, slug)| GameLink { id, title, slug })
        .collect())
    }

    async fn game_from_row(
        &self,
        row: GameContentRow,
        viewing_user_id: Option<i64>,
    ) -> Result<Game, GameError> {
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
                return Err(GameError::InternalError(
                    "Out of range game media index found".to_string(),
                ));
            }
            let index = medium.code as usize;
            medium_urls[index] = medium.url;
            medium_short_names[index] = medium.short_name;
        }

        let jsdos_bundle = sqlx::query_as::<_, (String, String, i64, String)>(
            "SELECT storage_key, original_file_name, size_bytes, sha256 FROM game_jsdos_bundles WHERE game_id = ?",
        )
        .bind(row.game_id)
        .fetch_optional(&self.pool)
        .await?
        .map(|(storage_key, original_file_name, size_bytes, sha256)| JsDosBundle {
            storage_key,
            original_file_name,
            size_bytes,
            sha256,
        });
        let cover_url = row.cover_url;
        let cover_media_type = row.cover_media_type;
        let og_image_url = Some(format!("media/i/.post.{}.thumbnail", row.post_id));

        Ok(Game {
            id: row.game_id,
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
            cover_url,
            cover_media_type,
            og_image_url,
            cover_video_url: row.cover_video_url,
            cover_video_type: row.cover_video_type,
            og_image_seconds: row.og_image_seconds,
            demo: GameDemo {
                launcher_type: row.launcher_type,
                width: row.demo_width,
                height: row.demo_height,
                demo_url: row.demo_url,
                jsdos_bundle,
            },
            instruction: row.instruction,
            cheatcode: row.cheatcode,
            story: row.story,
            related_games: self.related_games_for(row.game_id).await?,
            is_owner: viewing_user_id == Some(row.user_id),
        })
    }
}

#[async_trait::async_trait]
impl GameService for GameServiceImpl {
    async fn new_game(&self, cmd: NewGameCommand) -> Result<i64, GameError> {
        let mut tx = self.pool.begin().await?;
        let game_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO games (
                post_id, launcher_type, demo_width, demo_height, demo_url,
                instruction, cheatcode, story
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(cmd.post_id)
        .bind(cmd.launcher_type)
        .bind(cmd.demo_width)
        .bind(cmd.demo_height)
        .bind(cmd.demo_url)
        .bind(cmd.instruction)
        .bind(cmd.cheatcode)
        .bind(cmd.story)
        .fetch_one(&mut *tx)
        .await?;

        if !cmd.related_games.is_empty() {
            let values = cmd
                .related_games
                .iter()
                .map(|_| "(?, ?, ?)".to_string())
                .collect::<Vec<_>>()
                .join(", ");
            let sql = format!(
                "INSERT INTO related_games (game_id, related_game_id, sort_order) VALUES {}",
                values
            );
            let mut query = sqlx::query(&sql);
            for (index, link) in cmd.related_games.iter().enumerate() {
                query = query
                    .bind(game_id)
                    .bind(link.id)
                    .bind(index as i64);
            }
            query.execute(&mut *tx).await?;
        }

        tx.commit().await?;
        Ok(game_id)
    }

    async fn update_game(&self, cmd: UpdateGameCommand) -> Result<(), GameError> {
        let post_user_id: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT posts.user_id
            FROM games
            JOIN posts ON posts.id = games.post_id
            WHERE games.id = ?
            "#,
        )
        .bind(cmd.game_id)
        .fetch_optional(&self.pool)
        .await?;

        if post_user_id.is_none() {
            return Err(GameError::GameNotFound);
        }
        if post_user_id != Some(cmd.user_id) {
            return Err(GameError::Forbidden);
        }

        let mut tx = self.pool.begin().await?;
        let mut fields = vec![];
        if cmd.launcher_type.is_some() {
            fields.push("launcher_type = ?");
        }
        if cmd.demo_width.is_some() {
            fields.push("demo_width = ?");
        }
        if cmd.demo_height.is_some() {
            fields.push("demo_height = ?");
        }
        if cmd.demo_url.is_some() {
            fields.push("demo_url = ?");
        }
        if cmd.instruction.is_some() {
            fields.push("instruction = ?");
        }
        if cmd.cheatcode.is_some() {
            fields.push("cheatcode = ?");
        }
        if cmd.story.is_some() {
            fields.push("story = ?");
        }

        if !fields.is_empty() {
            fields.push("updated_at = CURRENT_TIMESTAMP");
            let sql = format!("UPDATE games SET {} WHERE id = ?", fields.join(", "));
            let mut query = sqlx::query(&sql);
            if let Some(value) = cmd.launcher_type {
                query = query.bind(value);
            }
            if let Some(value) = cmd.demo_width {
                query = query.bind(value);
            }
            if let Some(value) = cmd.demo_height {
                query = query.bind(value);
            }
            if let Some(value) = cmd.demo_url {
                query = query.bind(value);
            }
            if let Some(value) = cmd.instruction {
                query = query.bind(value);
            }
            if let Some(value) = cmd.cheatcode {
                query = query.bind(value);
            }
            if let Some(value) = cmd.story {
                query = query.bind(value);
            }
            query.bind(cmd.game_id).execute(&mut *tx).await?;
        }

        if let Some(related) = cmd.related_games {
            sqlx::query("DELETE FROM related_games WHERE game_id = ?")
                .bind(cmd.game_id)
                .execute(&mut *tx)
                .await?;
            if !related.is_empty() {
                let values = related
                    .iter()
                    .map(|_| "(?, ?, ?)".to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                let sql = format!(
                    "INSERT INTO related_games (game_id, related_game_id, sort_order) VALUES {}",
                    values
                );
                let mut query = sqlx::query(&sql);
                for (index, link) in related.iter().enumerate() {
                    query = query
                        .bind(cmd.game_id)
                        .bind(link.id)
                        .bind(index as i64);
                }
                query.execute(&mut *tx).await?;
            }
        }

        tx.commit().await?;
        Ok(())
    }

    async fn get_game_by_slug(
        &self,
        cmd: GetGameBySlugCommand,
    ) -> Result<Game, GameError> {
        if let Some(id) = cmd.as_id {
            let allowed: Option<i64> = sqlx::query_scalar(
                r#"
                SELECT posts.id
                FROM games
                JOIN posts ON posts.id = games.post_id
                WHERE posts.user_id = ? AND posts.slug = ?
                "#,
            )
            .bind(id)
            .bind(&cmd.slug)
            .fetch_optional(&self.pool)
            .await?;

            if allowed.is_none() {
                return Err(GameError::Forbidden);
            }
        }

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
                posts.draft,
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

    async fn get_game_details(
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
                posts.draft,
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

    async fn get_game_post_id(&self, cmd: GetGamePostIdCommand) -> Result<i64, GameError> {
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

    async fn get_latest_game_snapshots(
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

    async fn set_game_featured(
        &self,
        cmd: SetFeaturedGameCommand,
    ) -> Result<(), GameError> {
        let is_featured_val = if cmd.is_featured { 1 } else { 0 };
        sqlx::query(
            r#"
            UPDATE posts
            SET is_featured = ?
            WHERE id = (SELECT post_id FROM games WHERE id = ?)
            "#,
        )
        .bind(is_featured_val)
        .bind(cmd.game_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_featured_game_snapshots(
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

    async fn get_game_snapshots_by_tag(
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
