use axum::body::Bytes;

use crate::domain::entities::media::MediumDetails;

pub struct UploadMediumCommand {
    pub uploader_id: i64,
    pub short_name: String,
    pub description: String,
    pub file_name: String,
    pub content_type: String,
    pub bytes: Bytes,
}

pub struct ChangeAvatarCommand {
    pub user_id: i64,
    pub medium_details: MediumDetails,
}

pub struct ChangePostCoverCommand {
    pub user_id: i64,
    pub post_id: i64,
    pub medium_details: MediumDetails,
}

pub struct ChangeSeriesCoverCommand {
    pub user_id: i64,
    pub series_id: i64,
    pub medium_details: MediumDetails,
}

pub struct UploadMediaWithoutDescriptionCommand {
    pub uploader_id: i64,
    pub number_of_files: usize,
    pub short_names: Vec<String>,
    pub file_names: Vec<String>,
    pub content_types: Vec<String>,
    pub bytes_list: Vec<Bytes>,
}

pub struct SearchMediaCommand {
    pub term: Option<String>,
    pub size: u32,
    pub skip: u32,
}

pub struct GetLinkCommand {
    pub short_name: String,
}

pub struct GetMediaDetailsCommand {
    pub short_name: String,
}

pub struct ChangeMediaDetailsCommand {
    pub description: Option<String>,
    pub short_name: String,
    pub new_short_name: Option<String>,
}

pub struct GetAliasesCommand {
    pub short_name: String,
}

pub struct AddAliasCommand {
    pub short_name: String,
    pub alias: String,
}

pub struct ChangeAliasCommand {
    pub short_name: String,
    pub old_alias: String,
    pub new_alias: String,
}

pub struct DeleteAliasCommand {
    pub short_name: String,
    pub alias: String,
}
