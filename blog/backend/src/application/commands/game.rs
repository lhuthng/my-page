use crate::domain::entities::game::GameLink;

pub struct NewGameCommand {
    pub post_id: i64,
    pub launcher_type: String,
    pub demo_width: Option<String>,
    pub demo_height: Option<String>,
    pub demo_url: Option<String>,
    pub instruction: String,
    pub cheatcode: String,
    pub story: String,
    pub related_games: Vec<GameLink>,
}

pub struct UpdateGameCommand {
    pub game_id: i64,
    pub user_id: i64,
    pub launcher_type: Option<String>,
    pub demo_width: Option<String>,
    pub demo_height: Option<String>,
    pub demo_url: Option<String>,
    pub instruction: Option<String>,
    pub cheatcode: Option<String>,
    pub story: Option<String>,
    pub related_games: Option<Vec<GameLink>>,
}

pub struct GetGameBySlugCommand {
    pub slug: String,
    pub as_id: Option<i64>,
}

pub struct GetGameDetailsCommand {
    pub game_id: i64,
    pub viewing_user_id: i64,
    pub required_author_id: Option<i64>,
}

pub struct GetGamePostIdCommand {
    pub game_id: i64,
    pub required_author_id: Option<i64>,
}

pub struct GetLatestGamesCommand {
    pub limit: i64,
    pub offset: i64,
    pub public_only: bool,
    pub required_author_id: Option<i64>,
}

pub struct GetFeaturedGamesCommand {
    pub limit: i64,
}

pub struct SetFeaturedGameCommand {
    pub game_id: i64,
    pub is_featured: bool,
}

pub struct GetGamesByTagCommand {
    pub slug: String,
    pub limit: i64,
    pub offset: i64,
}