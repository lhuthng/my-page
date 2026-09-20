// Tiny helpers shared by every v86 sub-module: caller identity, upload
// expiry, and the storage-error conversion.
use chrono::Utc;

use crate::domain::{entities::secret::Claims, errors::project::ProjectError};

pub(super) fn user_id(claims: &Claims) -> Result<i64, ProjectError> {
    claims
        .user_id
        .parse::<i64>()
        .map_err(|_| ProjectError::InternalError("Cannot parse user id".to_string()))
}

pub(super) fn ensure_upload_not_expired(expires_at: &str) -> Result<(), ProjectError> {
    let expires_at = chrono::DateTime::parse_from_rfc3339(expires_at)
        .map_err(|_| ProjectError::InternalError("Invalid upload expiry timestamp.".to_string()))?;
    if expires_at.with_timezone(&Utc) <= Utc::now() {
        return Err(ProjectError::Conflict(
            "This upload session has expired.".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn storage_error(error: crate::infrastructure::storage::StorageError) -> ProjectError {
    ProjectError::InternalError(error.to_string())
}
