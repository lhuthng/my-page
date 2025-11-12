use axum::body::Bytes;

pub struct UploadMediaCommand {
    pub uploader_id: i64,
    pub short_name: String,
    pub description: String,
    pub file_name: String,
    pub content_type: String,
    pub bytes: Bytes,
}
