use crate::{
    application::commands,
    domain::{entities, errors},
};

#[async_trait::async_trait]
pub trait GameService {
    async fn new_game(
        &self,
        cmd: commands::game::NewGameCommand,
    ) -> Result<i64, errors::game::GameError>;
    async fn update_game(
        &self,
        cmd: commands::game::UpdateGameCommand,
    ) -> Result<(), errors::game::GameError>;
    async fn get_game_by_slug(
        &self,
        cmd: commands::game::GetGameBySlugCommand,
    ) -> Result<entities::game::Game, errors::game::GameError>;
    async fn get_game_details(
        &self,
        cmd: commands::game::GetGameDetailsCommand,
    ) -> Result<entities::game::Game, errors::game::GameError>;
    async fn get_game_post_id(
        &self,
        cmd: commands::game::GetGamePostIdCommand,
    ) -> Result<i64, errors::game::GameError>;
    async fn get_latest_game_snapshots(
        &self,
        cmd: commands::game::GetLatestGamesCommand,
    ) -> Result<entities::game::GameSnapshotPage, errors::game::GameError>;
    async fn set_game_featured(
        &self,
        cmd: commands::game::SetFeaturedGameCommand,
    ) -> Result<(), errors::game::GameError>;
    async fn get_featured_game_snapshots(
        &self,
        cmd: commands::game::GetFeaturedGamesCommand,
    ) -> Result<Vec<entities::game::GameSnapshot>, errors::game::GameError>;
    async fn get_game_snapshots_by_tag(
        &self,
        cmd: commands::game::GetGamesByTagCommand,
    ) -> Result<Vec<entities::game::GameSnapshot>, errors::game::GameError>;
}