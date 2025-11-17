use axum::body::Bytes;

pub struct UploadMediaCommand {
    pub uploader_id: i64,
    pub short_name: String,
    pub description: String,
    pub filename: String,
    pub content_type: String,
    pub bytes: Bytes,
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
    pub description: String,
    pub short_name: String,
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
