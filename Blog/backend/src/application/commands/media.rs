use axum::body::Bytes;

pub struct UploadMediaCommand {
    pub uploader_id: i64,
    pub short_name: String,
    pub description: String,
    pub filename: String,
    pub content_type: String,
    pub bytes: Bytes,
}
