// Row -> entity conversion: snapshot assembly, related games, tag hydration.
use std::collections::HashMap;

use crate::domain::entities::game::{Game, GameDemo, GameLink, GameSnapshot, JsDosBundle};
use crate::domain::entities::post::PostStats;
use crate::domain::errors::game::GameError;

use crate::infrastructure::persistence::post::{MediumUsageWithNameRow, TagRow};

use super::GameServiceImpl;
use super::rows::{GameContentRow, GameSnapshotRow, GameTagRow};

impl GameSnapshotRow {
    pub(super) fn into_snapshot(
        self,
        tag_names: Vec<String>,
        tag_slugs: Vec<String>,
    ) -> GameSnapshot {
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
    pub(super) async fn hydrate_game_rows(
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

    pub(super) async fn related_games_for(&self, game_id: i64) -> Result<Vec<GameLink>, GameError> {
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

    pub(super) async fn game_from_row(
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
