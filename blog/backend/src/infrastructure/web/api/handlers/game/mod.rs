// Game feature handlers, split by responsibility. This module is the public
// surface: route handlers for the router plus the wire types their
// signatures use. Internal helpers stay inside their sub-modules.
mod dto;
mod jsdos;
mod publish;
mod read;
mod response;
mod trash;
mod update;
mod write;

#[cfg(test)]
mod tests;

pub use dto::{
    CheckQuery, CheckResponse, CompleteJsDosUploadResponse, DeleteGameQuery,
    FeaturedGamesQuery, FeaturedGamesResponse, GameCard, GameStats, JsDosUploadResponse,
    LatestGamesQuery, LatestGamesResponse, SetGameFeaturedBody, StartJsDosUploadRequest,
    StartJsDosUploadResponse,
};
pub use jsdos::{
    abort_jsdos_upload, append_jsdos_chunk, complete_jsdos_upload, get_jsdos_bundle,
    start_jsdos_upload,
};
pub use publish::{publish_game, set_game_featured};
pub use read::{
    check_game, get_all_games, get_featured_games, get_game_by_slug, get_game_details,
    get_latest_games,
};
pub use response::{GameResponse, UpdateGameResponse};
pub use trash::{delete_game_draft, purge_game_now, restore_game};
pub use update::update_game;
pub use write::{change_cover, new_game};
