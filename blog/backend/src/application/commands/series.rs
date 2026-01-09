use crate::domain::entities::media::MediumDetails;

pub struct GetSeriesCommand {
    pub user_id: i64,
}

pub struct NewSeriesCommand {
    pub title: String,
    pub slug: String,
    pub description: String,
    pub user_id: i64,
    pub cover_image: Option<MediumDetails>,
}

pub struct AddPostToSeriesCommand {
    pub post_id: i64,
    pub series_id: i64,
    pub number: Option<i64>,
}
