// Tiny helpers shared by the audiobook sub-modules: caller identity, role
// check, multipart text fields, and list pagination.
use crate::domain::{entities::secret::Claims, errors::audiobook::AudiobookError};

fn caller_id(claims: &Claims) -> Result<i64, AudiobookError> {
    claims
        .user_id
        .parse::<i64>()
        .map_err(|_| AudiobookError::InternalError("Cannot parse id".to_string()))
}

fn is_admin(claims: &Claims) -> bool {
    claims.role == "admin"
}

/// Clamp a caller-supplied page window so a request cannot pull the whole table.
fn page_window(limit: Option<i64>, offset: Option<i64>, default: i64) -> (i64, i64) {
    (
        crate::helper::string::clamp_page_size(limit, default, 100),
        crate::helper::string::clamp_offset(offset),
    )
}


async fn read_text(field: axum::extract::multipart::Field<'_>) -> Result<String, AudiobookError> {
    field
        .text()
        .await
        .map_err(|e| AudiobookError::Validation(format!("Cannot read field: {e}")))
}
