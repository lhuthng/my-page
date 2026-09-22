// Game persistence adapter: implements
// `application::services::game::GameService` against SQLite.
use sqlx::SqlitePool;

mod mapping;
mod read;
mod rows;
mod write;

pub struct GameServiceImpl {
    pub pool: SqlitePool,
}

impl GameServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

// `GameService` port implementation: thin delegation to the inherent
// methods in the sibling files (inherent candidates win over trait
// candidates in method resolution, so `self.<method>` calls the local one).
use crate::application::commands;
use crate::application::services::game::GameService;
use crate::domain::{entities, errors};

#[async_trait::async_trait]
impl GameService for GameServiceImpl {
    async fn new_game(
        &self,
        cmd: commands::game::NewGameCommand,
    ) -> Result<i64, errors::game::GameError> {
        self.new_game(cmd).await
    }
    async fn update_game(
        &self,
        cmd: commands::game::UpdateGameCommand,
    ) -> Result<(), errors::game::GameError> {
        self.update_game(cmd).await
    }
    async fn get_game_by_slug(
        &self,
        cmd: commands::game::GetGameBySlugCommand,
    ) -> Result<entities::game::Game, errors::game::GameError> {
        self.get_game_by_slug(cmd).await
    }
    async fn get_game_details(
        &self,
        cmd: commands::game::GetGameDetailsCommand,
    ) -> Result<entities::game::Game, errors::game::GameError> {
        self.get_game_details(cmd).await
    }
    async fn get_game_post_id(
        &self,
        cmd: commands::game::GetGamePostIdCommand,
    ) -> Result<i64, errors::game::GameError> {
        self.get_game_post_id(cmd).await
    }
    async fn get_latest_game_snapshots(
        &self,
        cmd: commands::game::GetLatestGamesCommand,
    ) -> Result<entities::game::GameSnapshotPage, errors::game::GameError> {
        self.get_latest_game_snapshots(cmd).await
    }
    async fn set_game_featured(
        &self,
        cmd: commands::game::SetFeaturedGameCommand,
    ) -> Result<(), errors::game::GameError> {
        self.set_game_featured(cmd).await
    }
    async fn get_featured_game_snapshots(
        &self,
        cmd: commands::game::GetFeaturedGamesCommand,
    ) -> Result<Vec<entities::game::GameSnapshot>, errors::game::GameError> {
        self.get_featured_game_snapshots(cmd).await
    }
    async fn get_game_snapshots_by_tag(
        &self,
        cmd: commands::game::GetGamesByTagCommand,
    ) -> Result<Vec<entities::game::GameSnapshot>, errors::game::GameError> {
        self.get_game_snapshots_by_tag(cmd).await
    }
}
