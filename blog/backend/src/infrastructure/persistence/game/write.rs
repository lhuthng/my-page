// Game write methods: create, update, featured flag.

use crate::application::commands::game::{
    NewGameCommand, SetFeaturedGameCommand, UpdateGameCommand,
};
use crate::domain::errors::game::GameError;

use super::GameServiceImpl;

impl GameServiceImpl {
    pub(super) async fn new_game(&self, cmd: NewGameCommand) -> Result<i64, GameError> {
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
                query = query.bind(game_id).bind(link.id).bind(index as i64);
            }
            query.execute(&mut *tx).await?;
        }

        tx.commit().await?;
        Ok(game_id)
    }

    pub(super) async fn update_game(&self, cmd: UpdateGameCommand) -> Result<(), GameError> {
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
                    query = query.bind(cmd.game_id).bind(link.id).bind(index as i64);
                }
                query.execute(&mut *tx).await?;
            }
        }

        tx.commit().await?;
        Ok(())
    }

    pub(super) async fn set_game_featured(
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
}
